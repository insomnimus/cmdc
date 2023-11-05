param (
	[string] $FasmInclude,
	[switch] $noAutoInclude,

	[ValidateSet("x86_64-pc-windows-msvc", "i686-pc-windows-msvc")]
	[string[]] $targets = @("x86_64-pc-windows-msvc", "i686-pc-windows-msvc"),
	[string[]] $cargoFlags
)

if(!$noAutoInclude -and !$FasmInclude) {
	$p = scoop which fasm.exe
	if($LastExitCode -ne 0) {
		"failed to locate the fasm executable"
		exit 1
	}
	$FasmInclude = join-path (split-path $p) "INCLUDE"
	if(-not (test-path $FasmInclude)) {
		"failed to locate the flat assembler include directory"
		exit 1
	}
}

$inc = $env:INCLUDE
if(!$noAutoInclude) {
	$env:INCLUDE = $FasmInclude
}

pushd -lp $PSScriptRoot

foreach($f in get-item src/*.asm) {
	"assembling $($f.name)"
	$target = join-path (split-path $f) "$($f.basename).exe"
	fasm $f.fullname $target
	if($LastExitCode -ne 0) {
		"failed to assemble $($_.name)"
		$env:INCLUDE = $inc
		popd
		exit 1
	}
	"successfully assembled $($f.name)"
}

$env:INCLUDE = $inc

foreach($t in $targets) {
	"building the rust binary for $t"
	cargo build --target $t $cargoFlags
	if($LastExitCode -ne 0) {
		"error: cargo exited with exit code $lastExitCode"
		popd
		exit 1
	}
	"successfully built the rust binary for $t"
}

popd
