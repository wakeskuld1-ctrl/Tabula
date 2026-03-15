
try {
    $port3000 = Get-NetTCPConnection -LocalPort 3000 -ErrorAction SilentlyContinue
    if ($port3000) {
        Write-Host "Killing process on port 3000..."
        Stop-Process -Id $port3000.OwningProcess -Force -ErrorAction SilentlyContinue
    }
} catch {
    Write-Host "No process on 3000 or failed to kill."
}

try {
    $port5174 = Get-NetTCPConnection -LocalPort 5174 -ErrorAction SilentlyContinue
    if ($port5174) {
         Write-Host "Killing process on port 5174..."
         Stop-Process -Id $port5174.OwningProcess -Force -ErrorAction SilentlyContinue
    }
} catch {
    Write-Host "No process on 5174 or failed to kill."
}

Write-Host "Starting Backend..."
Start-Process -FilePath "cargo" -ArgumentList "run --bin tabula-server" -WorkingDirectory "d:\Rust\metadata\federated_query_engine"
Start-Sleep -Seconds 5
Write-Host "Starting Frontend..."
Start-Process -FilePath "npm" -ArgumentList "run dev" -WorkingDirectory "d:\Rust\metadata\frontend"
