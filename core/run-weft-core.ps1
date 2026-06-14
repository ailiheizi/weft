$ErrorActionPreference = 'Stop'

$workspace = 'D:\weft-workspace'
$tooling = Join-Path $workspace '.tooling'
$env:CARGO_HOME = Join-Path $tooling 'cargo'
$env:RUSTUP_HOME = Join-Path $tooling 'rustup'
$env:TEMP = Join-Path $tooling 'tmp'
$env:TMP = Join-Path $tooling 'tmp'
$env:PATH = "$(Join-Path $env:RUSTUP_HOME 'toolchains\stable-x86_64-pc-windows-msvc\bin');$(Join-Path $env:CARGO_HOME 'bin');$env:PATH"
$env:RUSTC = Join-Path $env:RUSTUP_HOME 'toolchains\stable-x86_64-pc-windows-msvc\bin\rustc.exe'

$cargo = "D:\weft-workspace\.tooling\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe"
if (-not (Test-Path $cargo)) {
  throw "cargo.exe not found at $cargo"
}

& $cargo run --release --bin weft-core
exit $LASTEXITCODE
