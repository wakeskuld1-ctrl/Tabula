# verify_yashandb.ps1
Write-Host "Ensure E:\YDC\YDC\yashandb_client\lib is in your PATH before running the backend!" -ForegroundColor Yellow
$baseUrl = "http://localhost:3000"

# 1. Test Connectivity
Write-Host "1. Testing YashanDB Connection..." -ForegroundColor Cyan
$connectPayload = @{
    user = "sys"
    pass = "sys"
    host = "127.0.0.1"
    port = 1688
    service = "yashandb"
}
$connectJson = $connectPayload | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/debug/yashandb" -Method Post -Body $connectJson -ContentType "application/json"
    if ($response.status -eq "ok") {
        Write-Host "Connection Successful!" -ForegroundColor Green
        Write-Host "Tables found: $($response.tables -join ', ')" -ForegroundColor Gray
    } else {
        Write-Host "Connection Failed: $($response.message)" -ForegroundColor Red
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# 2. Register Table
Write-Host "`n2. Registering YashanDB Table..." -ForegroundColor Cyan
$registerPayload = @{
    user = "sys"
    pass = "sys"
    host = "127.0.0.1"
    port = 1688
    service = "yashandb"
    table_name = "DUAL" # DUAL is standard
    alias = "yas_dual"
}
$registerJson = $registerPayload | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/datasources/yashandb/register" -Method Post -Body $registerJson -ContentType "application/json"
    if ($response.status -eq "ok") {
        Write-Host "Registration Successful!" -ForegroundColor Green
    } else {
        Write-Host "Registration Failed: $($response.message)" -ForegroundColor Red
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}

# 3. Query Table
Write-Host "`n3. Querying Registered Table..." -ForegroundColor Cyan
$sqlPayload = @{
    sql = "SELECT * FROM yas_dual"
}
$sqlJson = $sqlPayload | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "$baseUrl/api/execute" -Method Post -Body $sqlJson -ContentType "application/json"
    if ($response.status -eq "ok") {
        Write-Host "Query Successful!" -ForegroundColor Green
        $response.data | Format-Table
    } else {
        Write-Host "Query Failed: $($response.message)" -ForegroundColor Red
    }
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}
