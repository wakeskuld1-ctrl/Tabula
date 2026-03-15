# verify_unmerge.ps1
# PowerShell script to verify Merge and Unmerge functionality end-to-end
# 1. Creates a test CSV
# 2. Registers it as a table
# 3. Creates a session
# 4. Merges cells
# 5. Unmerges cells
# 6. Verifies metadata at each step

$ErrorActionPreference = "Stop"
$baseUrl = "http://localhost:3000"
$tableName = "test_unmerge_v1"
$csvPath = Join-Path (Get-Location) "data\test_unmerge_v1.csv"

# Ensure data directory exists
if (!(Test-Path "data")) {
    New-Item -ItemType Directory -Path "data" | Out-Null
}

# 0. Create Test Data
Write-Host "Creating test data at $csvPath..."
@"
id,name,value
1,A,10
2,B,20
3,C,30
4,D,40
"@ | Set-Content -Path $csvPath

# 1. Register Table (Critical for create_session to work)
Write-Host "Registering table..."
try {
    $regRes = Invoke-RestMethod -Uri "$baseUrl/api/register_table" -Method Post -Body (@{
        file_path = $csvPath
        table_name = $tableName
        source_type = "csv"
    } | ConvertTo-Json) -ContentType "application/json"
    Write-Host "Register Response: $($regRes | ConvertTo-Json -Depth 2)"
} catch {
    Write-Host "Error registering table: $_"
    exit 1
}

# 2. Create Session
Write-Host "Creating session..."
try {
    $sessionRes = Invoke-RestMethod -Uri "$baseUrl/api/create_session" -Method Post -Body (@{
        table_name = $tableName
        is_default = $true
    } | ConvertTo-Json) -ContentType "application/json"
    
    # Extract session_id from nested 'session' object
    $sessionId = $sessionRes.session.session_id
    
    if ([string]::IsNullOrWhiteSpace($sessionId)) {
        Write-Error "Failed to get session ID. Response: $($sessionRes | ConvertTo-Json -Depth 5)"
    }
    Write-Host "Session Created: $sessionId"
} catch {
    Write-Host "Error creating session: $_"
    exit 1
}

# 3. Merge Cells (A2:B3 -> rows 0-1, cols 0-1 0-indexed)
# Backend expects MergeRange object, not string
$mergeRangeObj = @{
    start_row = 0
    start_col = 0
    end_row = 1
    end_col = 1
}

Write-Host "Merging cells A2:B3 (0,0 -> 1,1)..."
try {
    $mergeRes = Invoke-RestMethod -Uri "$baseUrl/api/update_merge" -Method Post -Body (@{
        table_name = $tableName
        range = $mergeRangeObj
        session_id = $sessionId
    } | ConvertTo-Json) -ContentType "application/json"
    Write-Host "Merge Response: $($mergeRes | ConvertTo-Json)"
    
    if ($mergeRes.message -ne "Merged") {
        Write-Warning "Expected 'Merged' but got '$($mergeRes.message)'"
    }
} catch {
    Write-Host "Error merging: $_"
    exit 1
}

# 4. Verify Metadata (Should have merge)
Write-Host "Verifying Merge..."
$gridRes = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?table_name=$tableName&page=1&page_size=10&session_id=$sessionId" -Method Get
$merges = $gridRes.metadata.merges
Write-Host "Current Merges: $($merges | ConvertTo-Json)"

# Check if our range exists in the returned array
$found = $false
foreach ($m in $merges) {
    if ($m.start_row -eq 0 -and $m.start_col -eq 0 -and $m.end_row -eq 1 -and $m.end_col -eq 1) {
        $found = $true
        break
    }
}

if (-not $found) {
    Write-Error "Merge range not found in metadata!"
} else {
    Write-Host "Merge verified."
}

# 5. Unmerge (Call update_merge again on same range)
Write-Host "Unmerging cells..."
try {
    $unmergeRes = Invoke-RestMethod -Uri "$baseUrl/api/update_merge" -Method Post -Body (@{
        table_name = $tableName
        range = $mergeRangeObj
        session_id = $sessionId
    } | ConvertTo-Json) -ContentType "application/json"
    Write-Host "Unmerge Response: $($unmergeRes | ConvertTo-Json)"
    
    if ($unmergeRes.message -ne "Unmerged") {
         Write-Warning "Expected 'Unmerged' but got '$($unmergeRes.message)'"
    }
} catch {
    Write-Host "Error unmerging: $_"
    exit 1
}

# 6. Verify Metadata (Should be empty)
Write-Host "Verifying Unmerge..."
$gridRes2 = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?table_name=$tableName&page=1&page_size=10&session_id=$sessionId" -Method Get
$merges2 = $gridRes2.metadata.merges
Write-Host "Current Merges: $($merges2 | ConvertTo-Json)"

$found2 = $false
if ($merges2) {
    foreach ($m in $merges2) {
        if ($m.start_row -eq 0 -and $m.start_col -eq 0 -and $m.end_row -eq 1 -and $m.end_col -eq 1) {
            $found2 = $true
            break
        }
    }
}

if ($found2) {
    Write-Error "Merge range still exists in metadata!"
} else {
    Write-Host "Unmerge verified successfully!"
}

# Cleanup
Remove-Item $csvPath -ErrorAction SilentlyContinue
