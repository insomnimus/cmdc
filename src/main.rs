#[cfg(not(windows))]
compile_error!("this program can only be built on windows platforms");

use std::{
	ffi::OsStr,
	fs,
	os::windows::ffi::OsStrExt,
};

use clap::{
	arg,
	crate_version,
	Command,
};

const CURRENT_ARCH: &str = if cfg!(target_arch = "x86_64") {
	"x64"
} else {
	"x32"
};

static TEMPLATE32: Template = Template::new(include_bytes!("template32.exe"));
static TEMPLATE64: Template = Template::new(include_bytes!("template64.exe"));
// "replaceme" in utf-16 as bytes
const REPLACEME: &[u8] = &[
	0, 114, 0, 101, 0, 112, 0, 108, 0, 97, 0, 99, 0, 101, 0, 109, 0, 101,
];
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

	// Always quote the program name so CreateProcess to avoid ambiguity when
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

fn run() -> Result<(), Box<dyn std::error::Error>> {
	let m = Command::new("cmdc")
		.about("Compile a command into an executable")
		.version(crate_version!())
		.args(&[
			arg!(-o --out <file_name> "The output file name").allow_invalid_utf8(true),
			arg!(-a --arch [arch] "The target architecture")
				.possible_values(["x32", "x64"])
				.case_insensitive(true)
				.default_value(CURRENT_ARCH),
			arg!(<command> "The command to run, without any arguments").allow_invalid_utf8(true),
			arg!([args] ... "The arguments to embed into the program").allow_invalid_utf8(true),
		])
		.get_matches();

	let cmd = make_command_line(
		m.value_of_os("command").unwrap(),
		m.values_of_os("args").into_iter().flatten(),
	);

	let mut cmd = cmd
		.into_iter()
		.flat_map(|w| w.to_be_bytes())
		.collect::<Vec<_>>();

	if cmd.len() >= MAX_CMD {
		return Err("the command is too long for windows to handle".into());
	}
	// Pad rest of it with zeroes.
	cmd.extend((0..CMD_SIZE - cmd.len()).map(|_| 0));

	let template = match m.value_of("arch").unwrap() {
		"x64" | "X64" => TEMPLATE64,
		"x32" | "X32" => TEMPLATE32,
		_ => unreachable!(),
	};

	let data = template.generate(&cmd);
	fs::write(m.value_of_os("out").unwrap(), &data)?;
	Ok(())
}

fn main() {
	if let Err(e) = run() {
		eprintln!("error: {e}");
		std::process::exit(1);
	}
}
