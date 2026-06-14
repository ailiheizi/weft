$ErrorActionPreference = 'Stop'
$scriptPath = Join-Path $PSScriptRoot 'server.py'
if (-not (Test-Path $scriptPath)) {
  throw "memory-runtime server.py not found at $scriptPath"
}

$pythonCommands = @(
  @{ Command = 'python'; Args = @($scriptPath) },
  @{ Command = 'py'; Args = @('-3', $scriptPath) },
  @{ Command = 'python3'; Args = @($scriptPath) }
)

$started = $false
foreach ($candidate in $pythonCommands) {
  if (Get-Command $candidate.Command -ErrorAction SilentlyContinue) {
    & $candidate.Command @($candidate.Args)
    if ($LASTEXITCODE -eq 0) {
      $started = $true
      break
    }
  }
}

if (-not $started) {
  throw "memory-runtime could not find a usable Python launcher (tried: py, python, python3)"
}

exit $LASTEXITCODE
