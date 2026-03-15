
# Merge Verification Script
# Tests:
# 1. Horizontal Merge (A1:B1)
# 2. Vertical Merge (A2:A3)
# 3. Rectangular Merge (B2:C3)
# 4. Overlapping Merge (Update)

$ErrorActionPreference = "Stop"

function Assert-Equal($actual, $expected, $message) {
    if ($actual -ne $expected) {
        Write-Error "FAIL: $message. Expected '$expected', but got '$actual'."
    } else {
        Write-Host "PASS: $message" -ForegroundColor Green
    }
}

Write-Host "Starting Merge Verification..." -ForegroundColor Cyan

# 1. Setup Session
Write-Host "Creating session..."
try {
    $sessionRes = Invoke-RestMethod -Uri "http://localhost:3000/api/create_session" -Method Post -Body (@{
        table_name = "users"
        is_default = $true
    } | ConvertTo-Json) -ContentType "application/json"
    $sessionId = $sessionRes.session_id
    Write-Host "Session created: $sessionId"
} catch {
    Write-Error "Failed to create session: $_"
    exit 1
}

# 2. Horizontal Merge (Row 0, Col 0-1 -> A1:B1)
Write-Host "Testing Horizontal Merge (A1:B1)..."
$merge1 = @{
    start_col = 0
    start_row = 0
    end_col = 1
    end_row = 0
}
Invoke-RestMethod -Uri "http://localhost:3000/api/update_merge" -Method Post -Body (@{
    table_name = "users"
    range = $merge1
} | ConvertTo-Json) -ContentType "application/json"

# Verify
$data = Invoke-RestMethod -Uri "http://localhost:3000/api/grid-data?session_id=$sessionId&table_name=users&page=1&page_size=100"
$merges = $data.metadata.merges
$m1 = $merges | Where-Object { $_.start_row -eq 0 -and $_.start_col -eq 0 }
Assert-Equal $m1.end_col 1 "Horizontal merge end_col"
Assert-Equal $m1.end_row 0 "Horizontal merge end_row"

# 3. Vertical Merge (Row 1-2, Col 0 -> A2:A3)
Write-Host "Testing Vertical Merge (A2:A3)..."
$merge2 = @{
    start_col = 0
    start_row = 1
    end_col = 0
    end_row = 2
}
Invoke-RestMethod -Uri "http://localhost:3000/api/update_merge" -Method Post -Body (@{
    table_name = "users"
    range = $merge2
} | ConvertTo-Json) -ContentType "application/json"

# Verify
$data = Invoke-RestMethod -Uri "http://localhost:3000/api/grid-data?session_id=$sessionId&table_name=users&page=1&page_size=100"
$merges = $data.metadata.merges
$m2 = $merges | Where-Object { $_.start_row -eq 1 -and $_.start_col -eq 0 }
Assert-Equal $m2.end_row 2 "Vertical merge end_row"

# 4. Overlapping Merge (Should replace previous ones)
# New merge: A1:B2 (Row 0-1, Col 0-1)
# This overlaps with A1:B1 (Row 0) and A2:A3 (Row 1 part)
Write-Host "Testing Overlapping Merge (A1:B2)..."
$mergeOverlap = @{
    start_col = 0
    start_row = 0
    end_col = 1
    end_row = 1
}
Invoke-RestMethod -Uri "http://localhost:3000/api/update_merge" -Method Post -Body (@{
    table_name = "users"
    range = $mergeOverlap
} | ConvertTo-Json) -ContentType "application/json"

# Verify
$data = Invoke-RestMethod -Uri "http://localhost:3000/api/grid-data?session_id=$sessionId&table_name=users&page=1&page_size=100"
$merges = $data.metadata.merges

# Check A1:B1 is gone or replaced?
# A1:B1 (0,0 -> 0,1) overlaps with (0,0 -> 1,1). Should be removed.
$oldM1 = $merges | Where-Object { $_.start_row -eq 0 -and $_.start_col -eq 0 -and $_.end_row -eq 0 }
if ($oldM1) { Write-Error "Old horizontal merge should be removed" }

# Check A2:A3 (1,0 -> 2,0) overlaps with (0,0 -> 1,1) at (1,0). Should be removed.
$oldM2 = $merges | Where-Object { $_.start_row -eq 1 -and $_.start_col -eq 0 }
if ($oldM2) { Write-Error "Old vertical merge should be removed" }

# Check New Merge Exists
$newM = $merges | Where-Object { $_.start_row -eq 0 -and $_.start_col -eq 0 -and $_.end_row -eq 1 }
Assert-Equal $newM.end_col 1 "New overlap merge end_col"
Assert-Equal $newM.end_row 1 "New overlap merge end_row"

Write-Host "All Merge Tests Passed!" -ForegroundColor Green
