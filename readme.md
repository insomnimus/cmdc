# CMDC - Command Compiler
This program saves a command along with its arguments into a 32 or 64-bit PE executable (Windows only).

It receives the command and an output file, writes the serialized command into a pre-determined location in an executable template and saves the modified template into the output file.

For that, the executable template needs to be compiled first; for full control and minimal size, the template is written in assembly.

## But this is what a shell script should do! Why did you write this?!
No particular reason, take it or leave it :^).

There is actually one use case that I know of, though: `cargo` style subcommand plugins sometimes need `.exe` extensions.
So instead of writing a small program that basically calls a shell script with an interpreter, you can use `cmdc` to do it for you.

## Building
Since the templates are written in assembly and make use of macros provided by [flat-assembler](https://flatassembler.net), you should assemble it before running cargo.

1. Install flat-assembler from [here](https://flatassembler.net) or optionally from [scoop](https://github.com/ScoopInstaller/scoop).
2. Make sure the `$INCLUDE` environment variable contains the flat-assemblers include directory (it ships with a directory called `include`). For example on powershell: `$env:INCLUDE += ";D:\programs\flat-assembler\include"`.
3. Assemble the templates:
	```powershell
	cd cmdc/src
	fasm template32.asm template32.exe
	fasm template64.asm template64.exe
	```
4. Finally, compile the program with `cargo build --release`.

## Usage
Provide a command, an output file and optional arguments.
You can optionally specify a different arch than the one the program was compiled on with `--arch=x32|x64`.

```powershell
# cmd style ls
cmdc -o dir.exe -- cmd.exe /c dir
# ls using wsl
# note that wsl.exe only works on 64 bit
cmdc -a x64 -o ls.exe -- wsl.exe ls
```
