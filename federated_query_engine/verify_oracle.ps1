# verify_oracle.ps1
$env:PATH = "E:\YDC\YDC\oracle_client\instantclient_19_29;" + $env:PATH
$body = @{
    user = "tpcc"
    pass = "tpcc"
    host = "192.168.23.3"
    port = 1521
    service = "cyccbdata"
} | ConvertTo-Json

try {
    $response = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/debug/oracle" -Method Post -Body $body -ContentType "application/json" -ErrorAction Stop
    Write-Host "Status: $($response.status)"
    if ($response.status -eq "ok") {
        Write-Host "Tables found:"
        $response.tables | ForEach-Object { Write-Host " - $_" }
    } else {
        Write-Host "Error: $($response.message)"
    }
} catch {
    Write-Host "Request failed: $_"
    Write-Host "Response Body: $($_.Exception.Response.GetResponseStream() | %{ (New-Object System.IO.StreamReader $_).ReadToEnd() })"
}
