$baseUrl = "http://localhost:3000"

# 1. Execute SQL COUNT on 'age' (which is now likely Utf8)
$sql = "SELECT COUNT(age) as cnt FROM users"
$body = @{ sql = $sql } | ConvertTo-Json

Write-Host "Executing: $sql"
try {
    $res = Invoke-RestMethod -Uri "$baseUrl/api/execute" -Method Post -Body $body -ContentType "application/json"
    if ($res.error) {
        Write-Error "SQL Error: $($res.error)"
    } else {
        Write-Host "Result: $($res.rows | ConvertTo-Json -Depth 1)"
    }
} catch {
    Write-Error "Request failed: $_"
}

# 2. Execute SQL AVG on 'age' (Should fail if String)
$sqlAvg = "SELECT AVG(age) as avg_age FROM users"
$bodyAvg = @{ sql = $sqlAvg } | ConvertTo-Json

Write-Host "Executing: $sqlAvg"
try {
    $resAvg = Invoke-RestMethod -Uri "$baseUrl/api/execute" -Method Post -Body $bodyAvg -ContentType "application/json"
    if ($resAvg.error) {
        Write-Host "Expected Error for AVG: $($resAvg.error)"
    } else {
        Write-Host "Result AVG: $($resAvg.rows | ConvertTo-Json -Depth 1)"
    }
} catch {
    Write-Error "Request failed: $_"
}
