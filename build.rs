use std::{
	env,
	ffi::OsStr,
	fs,
	path::{
		Path,
		PathBuf,
	},
	process::Command,
};

fn assemble(fasm: &OsStr, source: &Path, dest: &Path, fasm_include: Option<&OsStr>) {
	let mut cmd = Command::new(fasm);
	cmd.args([source, dest]);
	if let Some(inc) = fasm_include {
		cmd.env("INCLUDE", inc);
	}

	match cmd.status() {
		Err(e) => panic!("failed to execute {cmd:?}: {e}"),
		Ok(stat) if stat.success() => (),
		Ok(stat) => panic!(
			"failed to assemble assembly files with fasm: the command {cmd:?} exited with {stat}"
		),
	}
}

fn main() {
	assert!(cfg!(windows), "this program only targets Windows");

	let files = fs::read_dir("src")
		.unwrap()
		.map(|e| e.unwrap().path())
		.filter(|f| f.extension().map_or(false, |ext| ext == "asm"))
		.collect::<Vec<_>>();
	for f in &files {
		println!("cargo:rerun-if-changed={}", f.display());
	}

	let out_dir = env::var_os("OUT_DIR").unwrap();
	let fasm_include = env::var_os("FASM_INCLUDE");
	let fasm = env::var_os("FASM").unwrap_or_else(|| "fasm.exe".into());

	for f in &files {
		let mut out = PathBuf::from(&out_dir);
		out.push(f.file_name().unwrap());
		out.set_extension("exe");
		assemble(&fasm, f, &out, fasm_include.as_deref());
	}
}
