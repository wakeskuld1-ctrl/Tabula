# E2E Simulation Script
$BaseUrl = "http://localhost:3000"

function Invoke-Api {
    param(
        [string]$Method,
        [string]$Endpoint,
        [hashtable]$Body = $null
    )
    $Url = "$BaseUrl$Endpoint"
    try {
        if ($Body) {
            $JsonBody = $Body | ConvertTo-Json -Depth 10
            $Response = Invoke-RestMethod -Method $Method -Uri $Url -Body $JsonBody -ContentType "application/json"
        } else {
            $Response = Invoke-RestMethod -Method $Method -Uri $Url -ContentType "application/json"
        }
        return $Response
    } catch {
        # Write-Host "Error calling $Endpoint : $_" -ForegroundColor Red
        # Return the error response if available
        if ($_.Exception.Response) {
             $Stream = $_.Exception.Response.GetResponseStream()
             $Reader = New-Object System.IO.StreamReader($Stream)
             $ErrorBody = $Reader.ReadToEnd()
             # Write-Host "Error Body: $ErrorBody" -ForegroundColor Red
             return ($ErrorBody | ConvertFrom-Json)
        }
        return $null
    }
}

Write-Host "--- 1. Testing Connectivity ---"
$Health = Invoke-Api -Method Get -Endpoint "/api/health"
Write-Host "Health: $($Health.status)"

Write-Host "`n--- 2. Cleanup: Unregistering Table (if exists) ---"
$UnregisterPayload = @{
    name = "yashan_tpcc_bmsql_district"
}
$UnregResult = Invoke-Api -Method Post -Endpoint "/api/datasources/unregister" -Body $UnregisterPayload
Write-Host "Unregister Result: $($UnregResult | ConvertTo-Json -Depth 2)"

Write-Host "`n--- 3. Registering YashanDB Table (BMSQL_DISTRICT) ---"
$RegisterPayload = @{
    user = "tpcc"
    pass = "tpcc"
    host = "192.168.23.4"
    port = 1843
    service = "yashandb"
    table_name = "BMSQL_DISTRICT"
    schema = "tpcc"
}
$RegResult = Invoke-Api -Method Post -Endpoint "/api/datasources/yashandb/register" -Body $RegisterPayload
Write-Host "Register Result: $($RegResult | ConvertTo-Json -Depth 2)"

Write-Host "`n--- 4. Executing Query (1st Run - Cache Miss) ---"
$QueryPayload = @{
    sql = "SELECT * FROM tpcc.BMSQL_DISTRICT LIMIT 10"
}
$QueryResult1 = Invoke-Api -Method Post -Endpoint "/api/execute" -Body $QueryPayload
Write-Host "Status: $($QueryResult1.status)"
if ($QueryResult1.rows) {
    Write-Host "Rows (First 1):"
    $QueryResult1.rows | Select-Object -First 1 | ConvertTo-Json -Depth 5 | Write-Host
    Write-Host "... ($($QueryResult1.rows.Count) rows total)"
} else {
    Write-Host "No rows returned or error."
}

Write-Host "`n--- 5. Waiting for Sidecar (10s) ---"
Start-Sleep -Seconds 10

Write-Host "`n--- 6. Executing Query (2nd Run - Cache Hit?) ---"
$Start = Get-Date
$QueryResult2 = Invoke-Api -Method Post -Endpoint "/api/execute" -Body $QueryPayload
$End = Get-Date
$Duration = ($End - $Start).TotalMilliseconds
Write-Host "Status: $($QueryResult2.status)"
Write-Host "Duration: $Duration ms"
if ($QueryResult2.rows) {
     Write-Host "Rows (First 1):"
     $QueryResult2.rows | Select-Object -First 1 | ConvertTo-Json -Depth 5 | Write-Host
}

Write-Host "`n--- 7. Getting Execution Plan & Cost ---"
$PlanPayload = @{
    sql = "SELECT * FROM tpcc.BMSQL_DISTRICT LIMIT 10"
}
$PlanResult = Invoke-Api -Method Post -Endpoint "/api/plan" -Body $PlanPayload
Write-Host "Plan Response JSON:"
$PlanResult | ConvertTo-Json -Depth 10 | Write-Host

Write-Host "`n--- 8. Checking Logs ---"
$Logs = Invoke-Api -Method Get -Endpoint "/api/logs"
$RecentLogs = $Logs.logs | Select-Object -Last 10
$RecentLogs | ForEach-Object { Write-Host $_ }

Write-Host "`n--- E2E Test Completed ---"
