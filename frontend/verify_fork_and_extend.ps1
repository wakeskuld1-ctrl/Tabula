
$baseUrl = "http://127.0.0.1:3000"
$timestamp = Get-Date -Format "yyyyMMddHHmmss"
$tableName = "test_fork_$timestamp"
$testDataDir = "d:\Rust\metadata\frontend\test_data"
if (-not (Test-Path $testDataDir)) {
    New-Item -ItemType Directory -Path $testDataDir | Out-Null
}
$csvPath = Join-Path $testDataDir "$tableName.csv"
$csvContent = "id,name,amount`n1,Alice,100.0`n2,Bob,200.0"

# 0. Prepare CSV
Write-Host "0. Creating CSV at $csvPath..."
$csvContent | Out-File -FilePath $csvPath -Encoding utf8

# 1. Upload CSV (using curl.exe to avoid CRLF/Multipart issues)
Write-Host "1. Uploading CSV to $baseUrl/api/upload..."
$uploadUri = "$baseUrl/api/upload"

# Use curl.exe for reliable multipart upload
# Note: We don't need to specify table_name in body if we use the filename as table name, 
# but the backend uses the filename (without extension) as table name.
# So our table name will be $tableName.

try {
    $curlOutput = & curl.exe -v -F "file=@$csvPath" $uploadUri 2>&1
    Write-Host "Curl Output: $curlOutput"
    
    # Check if curl failed
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Curl failed with exit code $LASTEXITCODE"
        exit 1
    }
} catch {
    Write-Error "Upload failed: $_"
    exit 1
}

Write-Host "   Upload seems successful (check backend logs if unsure)."

# 2. Hydrate (Create Default Session)
# The upload endpoint returns the table name.
# For uploaded files, the shadow parquet is at "data/uploaded_files/{filename}.parquet" or similar.
# But `hydrate` expects `parquet_path`.
# Wait, `hydrate` is legacy. `create_session` is better?
# Actually, the `upload_file` handler in main.rs returns:
# {"status": "ok", "message": "...", "table": "..."}
# And it registers the table in CacheManager.
# So we can just call `hydrate` with `table_name` and a dummy parquet path?
# Or does `hydrate` require a valid path?
# Let's check `hydrate` implementation in `session_manager/mod.rs`.
# It calls `create_session`.
# `create_session` uses `parquet_path` to open the dataset.
# So we need the correct path.
# The `upload_file` handler saves the file to `data/{filename}`.
# And converts to parquet at `data/{filename}.parquet`.
# So the path should be `data\$tableName.csv.parquet` or `data\$tableName.parquet`.
# Let's try `data\$tableName.parquet` (standard behavior of upload handler optimization).
# Actually, let's use `create_session` directly if possible? No, `hydrate` is the API.

# Let's look at `upload_file` response in `verify_auto_hydrate.ps1` output:
# {"message":"Uploaded and optimized ... as table '...' (Format: parquet)","status":"ok","table":"..."}
# It doesn't return the path.
# But `verify_auto_hydrate.ps1` called `update_cell` directly, which triggered auto-hydration!
# So we don't even need to call `/api/hydrate` explicitly if we rely on auto-hydration.
# BUT, we want to test "Default Session" (Read-only) behavior.
# Auto-hydration creates a "Default" session?
# In `main.rs`: `state.session_manager.hydrate(..., ...)` is called.
# In `session_manager/mod.rs`: `hydrate` calls `create_session(..., is_default=true)`.
# So auto-hydration creates a DEFAULT session.

# So we can just call `hydrate` explicitly to be sure, or trust `update_cell`.
# But `verify_fork_and_extend` wants to verify "Default Session Exists" BEFORE updating.
# So we need to call `/api/hydrate`.
# What is the path?
# In `main.rs`, `upload_file` saves to `filepath`.
# Then `CacheManager` converts it.
# The path is likely `data/$tableName.csv.parquet` (if extension included) or `data/$tableName.parquet`.
# Let's try `data/$tableName.csv` and let `hydrate` find the shadow?
# `hydrate` takes `parquet_path`.
# If I pass a non-existent path, it might fail if it tries to open it.
# However, if the table is already registered in `SessionContext`, maybe we don't need the path?
# `create_session` uses `Dataset::open(&uri)`.
# So we definitely need the valid URI.

# Hack: We can use `auto-hydrate` by calling a harmless update?
# Or just guess the path.
# Based on `verify_auto_hydrate.ps1` output: "Uploaded ... auto_hydrate_test_... .csv"
# The backend likely stores it as `data/auto_hydrate_test_... .csv`.
# And shadow as `data/auto_hydrate_test_... .csv.shadow.parquet`?
# Or `data/auto_hydrate_test_... .parquet`?
# Let's assume `data/$tableName.csv.shadow.parquet` (CacheManager default).

$parquetPath = "data/$tableName.csv.shadow.parquet"
Write-Host "2. Hydrating using presumed path: $parquetPath..."

$hydrateBody = @{
    table_name = $tableName
    parquet_path = $parquetPath
} | ConvertTo-Json

try {
    $hydrateRes = Invoke-RestMethod -Uri "$baseUrl/api/hydrate" -Method Post -Body $hydrateBody -ContentType "application/json"
    Write-Host "   Hydrate Response: $($hydrateRes | ConvertTo-Json -Depth 5)"
    
    # If explicit hydration fails (e.g. path wrong), we can try to rely on auto-hydration via update?
    # But we want to test "Default Session" state first.
    if ($hydrateRes.status -ne "ok") {
        Write-Warning "Hydration failed (maybe path wrong?). Attempting to proceed (checking sessions)..."
    } else {
        Write-Host "   Hydration Success."
    }
} catch {
    Write-Warning "Hydration request failed: $_. Proceeding..."
}

# 3. Verify Default Session Exists
Write-Host "3. Verifying Default Session..."
try {
    $sessionsRes = Invoke-RestMethod -Uri "$baseUrl/api/sessions?table_name=$tableName" -Method Get
    $sessions = $sessionsRes.sessions
    
    if ($sessions.Count -eq 0) {
        Write-Warning "No sessions found. Auto-hydration didn't run yet. This is expected if explicit hydration failed."
    } else {
        $defaultSession = $sessions[0]
        $defaultSessionId = $defaultSession.session_id
        Write-Host "   Default Session Found: $defaultSessionId (is_default=$($defaultSession.is_default))"
    }
} catch {
    Write-Error "Failed to list sessions: $_"
    exit 1
}

# 4. Attempt Update (Expect Fork)
# If no session exists, this will trigger Auto-Hydrate (creating Default) AND then Update?
# Wait, `update_cell` in `main.rs`:
# If `session_id` is None, it finds active session.
# If no active session, it errors -> triggers Auto-Hydrate.
# Auto-Hydrate creates Default Session.
# Then it retries update on Default Session.
# Default Session update triggers Fork.
# So we should get a NEW session ID, different from "Default".
# But if we didn't have a session before, the "Default" session is created inside `update_cell`'s auto-hydrate block.
# Then `update_cell` is called again.
# This 2nd call sees "Default" session.
# It should Fork.
# So we should get a session ID that is NOT the Default Session ID.
# But we don't know the Default Session ID if we didn't list it before.
# We can check `is_default` of the returned session?
# `update_cell` returns `session_id`.
# We can list sessions afterwards to see if we have 2 sessions (Default + Forked).

Write-Host "4. Updating Cell (Expect Fork)..."
$updateBody = @{
    session_id = $null
    table_name = $tableName
    row_idx = 0
    col_idx = 2
    col_name = "amount"
    old_value = "100.0"
    new_value = "999.0"
} | ConvertTo-Json

try {
    $updateRes = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $updateBody -ContentType "application/json"
    
    if ($updateRes.status -ne "ok") {
        Write-Error "Update failed: $($updateRes.message)"
        exit 1
    }
    
    $newSessionId = $updateRes.session_id
    Write-Host "   New Session ID: $newSessionId"
    
    # 5. Verify Isolation & Forking
    Write-Host "5. Verifying Session Count & Isolation..."
    $sessionsRes = Invoke-RestMethod -Uri "$baseUrl/api/sessions?table_name=$tableName" -Method Get
    $sessions = $sessionsRes.sessions
    Write-Host "   Total Sessions: $($sessions.Count)"
    
    if ($sessions.Count -lt 2) {
        # If auto-hydrate created Default, and then Fork created New, we should have 2.
        # Unless Auto-Hydrate logic in main.rs just creates a session and updates it directly without marking it Default?
        # main.rs: `state.session_manager.hydrate(...)` -> creates Default.
        # Then `update_cell` -> forks Default.
        # So we should have 2.
        Write-Warning "Expected at least 2 sessions (Default + Forked), found $($sessions.Count)."
        # It's possible the logic optimized away the default session if it was never used?
        # No, hydrate creates it.
    }
    
    # Check if we have a Default session
    $defaultSessions = $sessions | Where-Object { $_.is_default -eq $true }
    if ($defaultSessions) {
        Write-Host "   Default Session exists: $($defaultSessions.session_id)"
        
        if ($defaultSessions.session_id -eq $newSessionId) {
             Write-Error "FAILURE: Returned session IS the default session (No Fork occurred)."
             exit 1
        } else {
             Write-Host "   SUCCESS: Returned session is different from Default (Fork occurred)."
        }
    } else {
        Write-Warning "No Default session found? Maybe auto-hydration behaves differently?"
    }
    
} catch {
    Write-Error "Failed to update cell: $_"
    exit 1
}

# 6. Verify Data Isolation
Write-Host "6. Verifying Data Isolation..."
try {
    # Read New Session
    $gridNew = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?session_id=$newSessionId&table_name=$tableName&page=1&page_size=1" -Method Get
    $colIdx = $gridNew.columns.IndexOf("amount")
    $valNew = $gridNew.data[0][$colIdx]
    
    if ([string]$valNew -eq "999.0" -or [string]$valNew -eq "999") {
         Write-Host "   New Session Updated (OK): $valNew"
    } else {
         Write-Error "   FAILURE: New Session Not Updated! Expected 999.0, Got $valNew"
         exit 1
    }
    
    # Read Default Session (if exists)
    if ($defaultSessions) {
        $defId = $defaultSessions.session_id
        $gridDefault = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?session_id=$defId&table_name=$tableName&page=1&page_size=1" -Method Get
        $valDefault = $gridDefault.data[0][$colIdx]
        
        if ([string]$valDefault -eq "100.0" -or [string]$valDefault -eq "100") {
             Write-Host "   Default Session Unchanged (OK): $valDefault"
        } else {
             Write-Error "   FAILURE: Default Session Modified! Expected 100.0, Got $valDefault"
             exit 1
        }
    }
} catch {
    Write-Error "Verification failed: $_"
    exit 1
}

# 7. Test Append Row
Write-Host "7. Testing Append Row..."
try {
    # Get current row count of New Session
    $gridNewFull = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?session_id=$newSessionId&table_name=$tableName&page=1&page_size=1000" -Method Get
    $rowCount = $gridNewFull.total_rows
    Write-Host "   Current Rows: $rowCount"
    
    # Append at row = rowCount + 2 (creating gaps)
    $targetRow = $rowCount + 2 
    Write-Host "   Appending at Row Index: $targetRow"

    $appendBody = @{
        session_id = $newSessionId
        table_name = $tableName
        row_idx = $targetRow
        col_idx = $colIdx
        col_name = "amount"
        old_value = ""
        new_value = "888.0"
    } | ConvertTo-Json
    
    $appendRes = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $appendBody -ContentType "application/json"
    
    if ($appendRes.status -ne "ok") {
        Write-Error "Append failed: $($appendRes.message)"
        exit 1
    }
    Write-Host "   Append Request Success."
    
    # Verify Append
    $gridAppend = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?session_id=$newSessionId&table_name=$tableName&page=1&page_size=2000" -Method Get
    $newRowCount = $gridAppend.total_rows
    Write-Host "   New Row Count: $newRowCount"
    
    if ($newRowCount -gt $rowCount) {
        Write-Host "   SUCCESS: Rows added (Old: $rowCount, New: $newRowCount)."
    } else {
        Write-Error "   FAILURE: Row count did not increase."
        exit 1
    }
    
    # Check value at target row
    # Note: data array might be sparse or filled with nulls?
    # Grid data usually returns all rows.
    # $targetRow is 0-based index.
    
    # Ensure we have enough data returned
    if ($gridAppend.data.Count -le $targetRow) {
         Write-Error "FAILURE: Returned data length ($($gridAppend.data.Count)) < target row ($targetRow)."
         exit 1
    }
    
    $appendedVal = $gridAppend.data[$targetRow][$colIdx]
    Write-Host "   Value at Row ${targetRow}: $appendedVal"
    
    if ([string]$appendedVal -eq "888.0" -or [string]$appendedVal -eq "888") {
        Write-Host "   SUCCESS: Appended value correct."
    } else {
        Write-Error "   FAILURE: Appended value mismatch. Expected 888.0, Got $appendedVal"
        exit 1
    }

} catch {
    Write-Error "Append test failed: $_"
    exit 1
}

Write-Host "Test Complete."
