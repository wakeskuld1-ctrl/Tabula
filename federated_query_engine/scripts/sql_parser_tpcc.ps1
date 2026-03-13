$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path "$PSScriptRoot\..\.."
$LogDir = Join-Path $RepoRoot "logs"
if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir | Out-Null
}
$Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$LogPath = Join-Path $LogDir "sql_parser_tpcc_$Timestamp.log"
Push-Location $RepoRoot
try {
    cargo test -p federated_query_engine sql_parser_tpcc_tests -- --nocapture 2>&1 | Tee-Object -FilePath $LogPath
} finally {
    Pop-Location
}
Write-Output $LogPath
