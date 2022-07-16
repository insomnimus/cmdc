use std::{
	ffi::OsStr,
	fs,
	os::windows::ffi::OsStrExt,
};

use clap::arg;

const CURRENT_ARCH: &str = if cfg!(target_arch = "x86_64") {
	"x64"
} else {
	"x32"
};
static TEMPLATE32: &[u8] = include_bytes!("template32.exe");
static TEMPLATE64: &[u8] = include_bytes!("template64.exe");
const REPLACEME: &[u8] = &[
	0, 114, 0, 101, 0, 112, 0, 108, 0, 97, 0, 99, 0, 101, 0, 109, 0, 101,
];
const CMD_SIZE: usize = 32767 * 2 + REPLACEME.len();

fn replace(template: &[u8], with: &[u8]) -> Vec<u8> {
	debug_assert_eq!(CMD_SIZE, with.len());
	debug_assert!(
		template == TEMPLATE32 || template == TEMPLATE64,
		"template provided to `replace` is neither TEMPLATE32 nor TEMPLATE64"
	);
	let mut buf = Vec::with_capacity(template.len());

	for i in 0..template.len() - CMD_SIZE {
		if REPLACEME == &template[i..i + REPLACEME.len()] {
			buf.extend(&template[..i]);
			buf.extend(with);
			buf.extend(&template[i + CMD_SIZE..]);
			debug_assert_eq!(buf.len(), template.len());
			return buf;
		}
	}
	panic!("could not find the utf-16 text `replaceme` in the template executable");
}

fn make_command_line<I>(argv0: &OsStr, args: I) -> Result<Vec<u16>, &'static str>
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
	cmd.push(b'"' as u16);
	cmd.extend(argv0.encode_wide());
	cmd.push(b'"' as u16);

	for arg in args {
		cmd.push(' ' as u16);
		append_arg(&mut cmd, arg.as_ref())?;
	}
	Ok(cmd)
}

fn ensure_no_nulls(s: &OsStr) -> Result<(), &'static str> {
	if s.encode_wide().any(|b| b == 0) {
		Err("input contains null bytes")
	} else {
		Ok(())
	}
}

fn append_arg(cmd: &mut Vec<u16>, arg: &OsStr) -> Result<(), &'static str> {
	// If an argument has 0 characters then we need to quote it to ensure
	// that it actually gets passed through on the command line or otherwise
	// it will be dropped entirely when parsed on the other end.
	ensure_no_nulls(arg)?;
	let arg_bytes = arg
		.encode_wide()
		.flat_map(|w| w.to_ne_bytes())
		.collect::<Vec<_>>();
	let quote = arg_bytes.iter().any(|c| *c == b' ' || *c == b'\t') || arg_bytes.is_empty();

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
	Ok(())
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
	let m = clap::Command::new("cmdc")
		.about("Compile a command into a standalone executable")
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
	)?;

	let mut cmd = cmd
		.into_iter()
		.flat_map(|w| w.to_be_bytes())
		.collect::<Vec<_>>();

	if cmd.len() >= 32765 * 2 {
		return Err("the command is too long for windows to handle".into());
	}
	// Pad rest of it with zeroes.
	cmd.extend((0..CMD_SIZE - cmd.len()).map(|_| 0));

	let template = match m.value_of("arch").unwrap() {
		"x64" | "X64" => TEMPLATE64,
		"x32" | "X32" => TEMPLATE32,
		_ => unreachable!(),
	};

	let data = replace(template, &cmd);
	fs::write(m.value_of_os("out").unwrap(), &data)?;
	Ok(())
}

fn main() {
	if let Err(e) = run() {
		eprintln!("error: {e}");
		std::process::exit(1);
	}
}
