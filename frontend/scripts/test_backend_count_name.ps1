$baseUrl = "http://localhost:3000"

# 1. Execute SQL COUNT on 'username' (String column)
$sql = "SELECT COUNT(username) as cnt FROM users"
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
