#[cfg(not(windows))]
compile_error!("this program can only be built on windows platforms");

use std::{
	borrow::Cow,
	ffi::{
		OsStr,
		OsString,
	},
	fs,
	io::{
		self,
		Write,
	},
	os::windows::ffi::OsStrExt,
	path::PathBuf,
};

use clap::{
	arg,
	crate_version,
	value_parser,
	Command,
};

macro_rules! template {
	[$name:literal] => {
		Template::new(include_bytes!(
			concat!(env!("OUT_DIR"), "/", $name, ".exe")
		))
	};
}

static CURRENT_ARCH: &str = if cfg!(target_arch = "x86_64") {
	"x64"
} else {
	"x32"
};

static ANCHOR: &[u8] = b"---CMDC COMMAND STRING---";
const REPLACEME: &[u8] = b"r\0e\0p\0l\0a\0c\0e\0m\0e\0";

static TEMPLATE32: Template = template!("template32");
static TEMPLATE64: Template = template!("template64");

const MAX_CMD: usize = 32765 * 2;
const CMD_SIZE: usize = 32767 * 2 + REPLACEME.len();

#[derive(Copy, Clone)]
struct Template {
	data: &'static [u8],
	replacement_index: usize,
}

impl Template {
	const fn new(data: &'static [u8]) -> Self {
		const fn slice_eq(data: &[u8], index: usize, right: &[u8]) -> bool {
			let mut i = 0;
			while i < right.len() {
				if data[i + index] != right[i] {
					return false;
				}
				i += 1;
			}
			true
		}

		assert!(data.len() >= REPLACEME.len());

		let mut i = 0;
		while i <= data.len() - REPLACEME.len() {
			if slice_eq(data, i, REPLACEME) {
				return Self {
					data,
					replacement_index: i,
				};
			}
			i += 1;
		}

		panic!("string not found in template");
	}

	fn generate(self, replacement: &[u8]) -> Vec<u8> {
		assert_eq!(
			replacement.len(),
			CMD_SIZE,
			"replacement must be padded to CMD_SIZE bytes"
		);
		let mut buf = self.data.to_vec();
		buf[self.replacement_index..self.replacement_index + replacement.len()]
			.copy_from_slice(replacement);

		buf
	}
}

fn make_command_line<I>(argv0: &OsStr, args: I) -> Vec<u16>
where
	I: IntoIterator,
	I::Item: AsRef<OsStr>,
{
	// Encode the command and arguments in a command line string such
	// that the spawned process may recover them using CommandLineToArgvW.
	let mut cmd: Vec<u16> = Vec::new();

	// Always quote the program name to avoid ambiguity when
	// the child process parses its arguments.
	// Note that quotes aren't escaped here because they can't be used in arg0.
	// But that's ok because file paths can't contain quotes.
	cmd.push('"' as u16);
	cmd.extend(argv0.encode_wide());
	cmd.push('"' as u16);

	for arg in args {
		cmd.push(' ' as u16);
		append_arg(&mut cmd, arg.as_ref());
	}
	cmd
}

fn append_arg(cmd: &mut Vec<u16>, arg: &OsStr) {
	// If an argument has 0 characters then we need to quote it to ensure
	// that it actually gets passed through on the command line or otherwise
	// it will be dropped entirely when parsed on the other end.
	let quote = arg.is_empty()
		|| arg
			.encode_wide()
			.any(|c| c == ' ' as u16 || c == '\t' as u16);

	if quote {
		cmd.push('"' as u16);
	}

	let mut backslashes: usize = 0;
	for x in arg.encode_wide() {
		if x == '\\' as u16 {
			backslashes += 1;
		} else {
			if x == '"' as u16 {
				// Add n+1 backslashes to total 2n+1 before internal '"'.
				cmd.extend((0..=backslashes).map(|_| '\\' as u16));
			}
			backslashes = 0;
		}
		cmd.push(x);
	}

	if quote {
		// Add n backslashes to total 2n before ending '"'.
		cmd.extend((0..backslashes).map(|_| '\\' as u16));
		cmd.push('"' as u16);
	}
}

fn read_command(data: &[u8]) -> Option<Cow<'_, str>> {
	if data.len() <= ANCHOR.len() + MAX_CMD {
		return None;
	}

	let anchor = data.windows(ANCHOR.len()).position(|w| w == ANCHOR)?;
	// Anchor found, command string is placed right after it in the executable
	let start = anchor + ANCHOR.len();
	if start >= data.len() {
		return None;
	}

	// The command terminates with a null word (\0\0), find it
	// let end = (start..data.len() - 1)
	// .step_by(2)
	// .find(|&i| data[i] == 0 && data[i + 1] == 0)?;
	let end = start + data[start..].chunks(2).position(|w| w == [0, 0])? * 2;

	if start == end {
		None
	} else {
		// It's UTF16-LE encoded but rust strings are UTF8, we have to convert
		assert!(
			(end - start) % 2 == 0,
			"internal error: expected the byte slice to have even number of items"
		);
		Some(
			encoding_rs::UTF_16LE
				.decode_without_bom_handling(&data[start..end])
				.0,
		)
	}
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
	let m = Command::new("cmdc")
		.about("Compile a command into an executable")
		.version(crate_version!())
		.args(&[
			arg!(-i --inspect [executable] "Print the command in an executable compiled with this tool").exclusive(true)
			.value_parser(value_parser!(PathBuf)),
			arg!(-o --out [file_name] "The output file name (use - for stdout)")
				.allow_invalid_utf8(true)
				.required_unless_present("inspect")
				.value_parser(value_parser!(OsString)),
			arg!(-a --arch [arch] "The target architecture")
				.possible_values(["x32", "x64"])
				.case_insensitive(true)
				.default_value(CURRENT_ARCH),
			arg!([command] "The command to run, without any arguments")
				.allow_invalid_utf8(true)
				.value_parser(value_parser!(OsString))
				.required_unless_present("inspect"),
			arg!([args] ... "The arguments to embed into the program")
				.allow_invalid_utf8(true)
				.value_parser(value_parser!(OsString)),
		])
		.get_matches();

	if let Some(p) = m.get_one::<PathBuf>("inspect") {
		const SINCE: &str = "0.3.0";
		let data = fs::read(p)?;
		match read_command(&data) {
			Some(s) => println!("{s}"),
			None => {
				return Err(format!(
					"{}: not an executable produced by cmdc version >= {}",
					p.display(),
					SINCE
				)
				.into())
			}
		}
		return Ok(());
	}

	let cmd = make_command_line(
		m.get_one::<OsString>("command").unwrap(),
		m.get_many::<OsString>("args").into_iter().flatten(),
	);

	let mut cmd = cmd
		.into_iter()
		.flat_map(|w| w.to_le_bytes())
		.collect::<Vec<_>>();

	if cmd.len() >= MAX_CMD {
		return Err("the command is too long for windows to handle".into());
	}
	// Pad rest of it with zeroes.
	cmd.extend((0..CMD_SIZE - cmd.len()).map(|_| 0));

	let template = match m.get_one::<String>("arch").unwrap().as_str() {
		"x64" | "X64" => TEMPLATE64,
		"x32" | "X32" => TEMPLATE32,
		_ => unreachable!(),
	};

	let data = template.generate(&cmd);
	let p = m.value_of_os("out").unwrap();
	if p == "-" {
		let mut stdout = io::stdout().lock();
		stdout.write_all(&data)?;
	} else {
		fs::write(m.value_of_os("out").unwrap(), &data)?;
	}

	Ok(())
}

fn main() {
	if let Err(e) = run() {
		eprintln!("error: {e}");
		std::process::exit(1);
	}
}
