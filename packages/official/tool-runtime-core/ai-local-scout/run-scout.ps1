param(
  [string]$Output = "output/ai-local-scout-report.json",
  [switch]$SteamPublicProfile
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$argsList = @(
  "-m", "ai_local_scout.cli",
  "--root", "C:\Users\Admin",
  "--root", "D:\weft",
  "--home", "C:\Users\Admin",
  "--output", (Join-Path $root $Output),
  "--max-depth", "6",
  "--max-sqlite-parse", "16"
)

if ($SteamPublicProfile) {
  $argsList += "--enable-steam-public-profile"
}

Push-Location $root
try {
  python @argsList
  Write-Host "Report written to $(Join-Path $root $Output)"
} finally {
  Pop-Location
}
