$body = @{ sql = "SELECT * FROM BMSQL_CONFIG" } | ConvertTo-Json
Write-Host "--- BMSQL_CONFIG ---"
try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/execute" -Method Post -Body $body -ContentType "application/json"
    $response | ConvertTo-Json -Depth 5
} catch {
    Write-Host "Error: $_"
    $_.Exception.Response
}

$body3 = @{ sql = "SELECT cfg_name, CAST(cfg_value AS VARCHAR) as cfg_value FROM BMSQL_CONFIG EXCEPT SELECT cfg_name, CAST(cfg_value AS VARCHAR) as cfg_value FROM yashan_mock" } | ConvertTo-Json
Write-Host "--- EXCEPT Query ---"
try {
    $response3 = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/execute" -Method Post -Body $body3 -ContentType "application/json"
    $response3 | ConvertTo-Json -Depth 5
} catch {
    Write-Host "Error: $_"
    $_.Exception.Response
}
