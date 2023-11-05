# CMDC - Command Compiler
This program saves a command along with its arguments into a 32 or 64-bit PE executable (Windows only).

It receives the command and an output file, writes the serialized command into a pre-determined location in an executable template and saves the modified template into the output file.

For that, the executable template needs to be compiled first; for full control and minimal size, the template is written in assembly.

## But this is what a shell script should do! Why did you write this?!
No particular reason, take it or leave it :^).

There is actually one use case that I know of, though: `cargo` style subcommand plugins sometimes need `.exe` extensions.
So instead of writing a small program that basically calls a shell script with an interpreter, you can use `cmdc` to do it for you.

## Install
Grab a pre-built binary from the [releases page](https://github.com/insomnimus/cmdc/releases) ([here's the latest release](https://github.com/insomnimus/cmdc/releases/latest)).

Or build from source:

## Building
Since the templates are written in assembly and make use of macros provided by [flat-assembler](https://flatassembler.net), you must have the `fasm.exe` installed on your system.
You will also need a recent enough version of rust (tested with 1.73.0).

1. Install flat-assembler from [here](https://flatassembler.net) or optionally from [scoop](https://github.com/ScoopInstaller/scoop).
2. If flat-assembler's `INCLUDE` directory is not in your `$INCLUDE` env variable, put it there. Or optionally set the `FASM_INCLUDE` env variable to the directory.
3. If `fasm.exe` is not in `$PATH`, set the `FASM` environment variable to the full path where `fasm.exe` is located; e.g `D:\fasm\fasm.exe`.
4. Build like the usual: `cargo build --release`. The build script will take care of assembling the assembly files.

## Usage
Provide a command, an output file and optional arguments.
You can optionally specify a different arch than the one the program was compiled on with `--arch=x32|x64`.

```powershell
# cmd style ls
cmdc -o dir.exe -- cmd.exe /c dir
# Inspect the generated executable:
cmdc -i dir.exe
# prints: "cmd.exe" /c dir
# ls using wsl
# note that wsl.exe only works on 64 bit
cmdc -a x64 -o ls.exe -- wsl.exe ls
# Inspect the executable:
cmdc --inspect ls.exe
# prints: "wsl.exe" ls
```
