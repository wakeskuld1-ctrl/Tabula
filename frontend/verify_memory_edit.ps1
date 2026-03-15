# Verify In-Memory Editing and Schema Evolution
$baseUrl = "http://127.0.0.1:3000"
$timestamp = Get-Date -Format "yyyyMMddHHmmss"
$tableName = "verify_mem_$timestamp"
$csvPath = Join-Path $PSScriptRoot "test_data/verify_memory_edit.csv"
# We will simulate upload by copying file directly to backend data dir
$backendDataDir = "..\federated_query_engine\data"
$serverCsvPath = Join-Path $backendDataDir "verify_memory_edit_$timestamp.csv"
# We use .csv extension for hydrate to test the CSV fallback in create_session

# Cleanup
if (-not (Test-Path $backendDataDir)) {
    New-Item -ItemType Directory -Force -Path $backendDataDir | Out-Null
}

# 1. Create Test CSV
if (-not (Test-Path (Join-Path $PSScriptRoot "test_data"))) {
    New-Item -ItemType Directory -Force -Path (Join-Path $PSScriptRoot "test_data") | Out-Null
}
"id,name,score`n1,Alice,10`n2,Bob,20" | Out-File -FilePath $csvPath -Encoding utf8

# 2. Simulate Upload (Copy file)
Write-Host "Simulating Upload (Copying CSV to server data dir)..."
Copy-Item $csvPath $serverCsvPath -Force
Write-Host "Copied to $serverCsvPath"

# 3. Hydrate (Load into Memory)
Write-Host "Hydrating session..."
# Path relative to federated_query_engine root or absolute?
# The server runs in federated_query_engine.
# So "data/filename.csv" should work.
$hydratePath = "data/verify_memory_edit_$timestamp.csv"

$hydrateBody = @{
    table_name = $tableName
    parquet_path = $hydratePath
} | ConvertTo-Json

try {
    $hydrateRes = Invoke-RestMethod -Uri "$baseUrl/api/hydrate" -Method Post -Body $hydrateBody -ContentType "application/json"
    Write-Host "Hydrate Result: $hydrateRes"
} catch {
    Write-Error "Hydrate failed: $_"
    exit 1
}

# 4. Update Existing Cell (In-Memory)
Write-Host "Updating Cell (In-Memory)..."
$updateBody = @{
    table_name = $tableName
    row_idx = 0
    col_name = "score"
    col_idx = 2
    old_value = "10"
    new_value = "99"
} | ConvertTo-Json
$updateRes = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $updateBody -ContentType "application/json"
Write-Host "Update Result: $($updateRes | ConvertTo-Json -Depth 5)"

if ($updateRes.status -eq "error") {
    Write-Error "Update failed: $($updateRes.message)"
    exit 1
}

# 5. Add New Column (Schema Evolution)
Write-Host "Adding New Column '__new_col_0'..."
$newColBody = @{
    table_name = $tableName
    row_idx = 0
    col_name = "__new_col_0"
    col_idx = 3
    old_value = ""
    new_value = "NewVal"
} | ConvertTo-Json
$newColRes = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $newColBody -ContentType "application/json"
Write-Host "New Column Result: $($newColRes | ConvertTo-Json -Depth 5)"

# 6. Verify Data via Grid API (Should see updates)
Write-Host "Verifying Data..."
$gridRes = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?table_name=$tableName&page=1&page_size=10" -Method Get
Write-Host "Grid Data: $($gridRes | ConvertTo-Json -Depth 5)"

$rows = $gridRes.data
if ($rows[0][2] -ne "99") {
    Write-Error "Update failed: Expected 99, got $($rows[0][2])"
}
if ($rows[0][3] -ne "NewVal") {
    Write-Error "New Column failed: Expected NewVal, got $($rows[0][3])"
}

# 7. Save Session
Write-Host "Saving Session..."
$saveBody = @{
    table_name = $tableName
} | ConvertTo-Json
$saveRes = Invoke-RestMethod -Uri "$baseUrl/api/save_session" -Method Post -Body $saveBody -ContentType "application/json"
Write-Host "Save Result: $($saveRes | ConvertTo-Json -Depth 5)"

Write-Host "Verification Completed Successfully!"
