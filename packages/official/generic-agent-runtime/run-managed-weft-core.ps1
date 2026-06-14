$ErrorActionPreference = 'Stop'

$runtimeRoot = Join-Path $PSScriptRoot 'runtime-root'
$manifestPath = 'D:\weft-workspace\weft\WEFT-core\Cargo.toml'
$cargo = 'D:\weft-workspace\.tooling\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe'

if (-not (Test-Path $runtimeRoot)) {
  throw "runtime root not found: $runtimeRoot"
}
if (-not (Test-Path $manifestPath)) {
  throw "WEFT-core Cargo.toml not found: $manifestPath"
}
if (-not (Test-Path $cargo)) {
  throw "cargo.exe not found: $cargo"
}

$env:CARGO_HOME = 'D:\weft-workspace\.tooling\cargo'
$env:RUSTUP_HOME = 'D:\weft-workspace\.tooling\rustup'
$env:PATH = "D:\weft-workspace\.tooling\cargo\bin;D:\weft-workspace\.tooling\rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;$env:PATH"

Set-Location $runtimeRoot
& $cargo run --release --manifest-path $manifestPath --bin weft-core
exit $LASTEXITCODE
