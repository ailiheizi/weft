$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path

Push-Location $root
try {
  python -m pytest tests -q
} finally {
  Pop-Location
}
