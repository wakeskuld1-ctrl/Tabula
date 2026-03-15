# verify_auto_hydrate.ps1
# Verifies that update_cell automatically hydrates a session if one is missing

$baseUrl = "http://localhost:3000"
$csvPath = "d:\Rust\metadata\frontend\test_data\auto_hydrate_test.csv"

# 1. Create a fresh CSV
$csvContent = @"
id,name,score
1,Alice,100
2,Bob,90
"@
$csvContent | Out-File -FilePath $csvPath -Encoding utf8
Write-Host "Created test CSV at $csvPath"

# 2. Upload CSV (this creates table metadata but NO session)
$tableName = "auto_hydrate_test_" + (Get-Date -Format "yyyyMMddHHmmss")
$tempCsvPath = "d:\Rust\metadata\frontend\test_data\$tableName.csv"
Copy-Item -Path $csvPath -Destination $tempCsvPath

Write-Host "Uploading CSV as table: $tableName (File: $tempCsvPath)"

$uploadUri = "$baseUrl/api/upload"
# Use curl.exe for reliable multipart upload
Write-Host "Uploading using curl.exe..."
& curl.exe -v -F "file=@$tempCsvPath" $uploadUri
Write-Host "`nUpload complete."

# Cleanup temp file
Remove-Item -Path $tempCsvPath

# 3. DIRECTLY Call update_cell WITHOUT Hydrate
# This should trigger the auto-hydrate logic in backend
Write-Host "Attempting Update Cell (should trigger Auto-Hydrate)..."
$updateBody = @{
    table_name = $tableName
    row_idx = 0
    col_idx = 2
    col_name = "score"
    old_value = "100"
    new_value = "999"
} | ConvertTo-Json

try {
    $res = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $updateBody -ContentType "application/json" -ErrorAction Stop
    Write-Host "Update Response: $($res | ConvertTo-Json -Depth 5)"
    
    if ($res.status -eq "ok") {
        Write-Host "SUCCESS: Auto-hydration and update worked!"
    } else {
        Write-Host "FAILURE: Update returned error status."
        
        # Fetch Logs
        Write-Host "Fetching Backend Logs..."
        try {
            $logs = Invoke-RestMethod -Uri "$baseUrl/api/logs" -Method Get
            $logs.logs | Select-Object -Last 20 | ForEach-Object { Write-Host "LOG: $_" }
        } catch {
            Write-Host "Could not fetch logs."
        }
        exit 1
    }
} catch {
    Write-Host "FAILURE: Request failed. $($_.Exception.Message)"
    # Fetch Logs
    Write-Host "Fetching Backend Logs..."
    try {
        $logs = Invoke-RestMethod -Uri "$baseUrl/api/logs" -Method Get
        $logs.logs | Select-Object -Last 20 | ForEach-Object { Write-Host "LOG: $_" }
    } catch {
        Write-Host "Could not fetch logs."
    }
    exit 1
}

# 4. Verify Data via SQL
Write-Host "Verifying data..."
$sqlBody = @{
    sql = "SELECT * FROM $tableName WHERE id = 1"
} | ConvertTo-Json

$queryRes = Invoke-RestMethod -Uri "$baseUrl/api/execute" -Method Post -Body $sqlBody -ContentType "application/json"
Write-Host "Query Response: $($queryRes | ConvertTo-Json -Depth 5)"
$row = $queryRes.rows[0]
Write-Host "Row Data: $($row | ConvertTo-Json)"

# score is index 2 (id, name, score)
if ($row[2] -eq 999 -or $row[2] -eq "999") {
    Write-Host "SUCCESS: Data verified as 999."
} else {
    Write-Host "FAILURE: Data mismatch. Expected 999, got $($row[2])"
    exit 1
}
