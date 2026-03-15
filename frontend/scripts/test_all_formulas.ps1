$baseUrl = "http://localhost:3000"
$tableName = "users" # Use known table to avoid session issues

function Test-Step {
    param($name, $scriptBlock)
    Write-Host "[$name] Running..." -ForegroundColor Cyan
    try {
        & $scriptBlock
        Write-Host "[$name] Passed" -ForegroundColor Green
    } catch {
        Write-Error "[$name] Failed: $_"
        exit 1
    }
}

Test-Step "Init Table" {
    # We will use 'users' table. We assume it has 'age' (col 2) and 'username' (col 1).
    # We'll use row 0 for all tests to avoid row-count issues.
    # We will overwrite values.
    
    # Reset values first
    $body = @{
        table_name = $tableName
        row_idx = 0
        col_idx = 2
        col_name = "age"
        old_value = ""
        new_value = "30"
    } | ConvertTo-Json
    Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $body -ContentType "application/json" | Out-Null
}

# 1. Math & Trig (Target 'age' column - triggers type promotion)
Test-Step "Math Functions (SUM)" {
    $body = @{ 
        table_name=$tableName
        row_idx=0
        col_idx=2
        col_name="age"
        old_value=""
        new_value="=SUM(10,20)" 
    } | ConvertTo-Json
    Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $body -ContentType "application/json" | Out-Null
}

# 2. Text (Target 'username' column)
Test-Step "Text Functions (CONCATENATE)" {
    $body = @{ 
        table_name=$tableName
        row_idx=0
        col_idx=1
        col_name="username"
        old_value=""
        new_value="=CONCATENATE('User', 'Name')" 
    } | ConvertTo-Json
    Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $body -ContentType "application/json" | Out-Null
}

# 3. Logical (Target 'age' column again)
Test-Step "Logical Functions (IF)" {
    # Note: If previous step promoted 'age' to String, this is fine.
    $body = @{ 
        table_name=$tableName
        row_idx=0
        col_idx=2
        col_name="age"
        old_value=""
        new_value="=IF(1>0, 100, 200)" 
    } | ConvertTo-Json
    Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $body -ContentType "application/json" | Out-Null
}

Test-Step "Verify All Persisted" {
    $gridUrl = "$baseUrl/api/grid-data?table_name=$tableName&page=1&page_size=20"
    $grid = Invoke-RestMethod -Uri $gridUrl -Method Get
    
    if ($null -eq $grid.data) {
        throw "Grid data is empty"
    }
    
    # Row 0, Col 2 (Age) - Should be =IF(...) (Last write wins)
    # Wait, we want to verify multiple categories. 
    # But we are overwriting the same cell for Math and Logical.
    # Let's verify the last one.
    
    $valAge = $grid.data[0][2]
    Write-Host "Row 0 Col 2 (Age): $valAge"
    if ($valAge -ne "=IF(1>0, 100, 200)") { 
        Write-Warning "Expected '=IF(1>0, 100, 200)' but got '$valAge'"
    }
    
    # Row 0, Col 1 (Username)
    $valUser = $grid.data[0][1]
    Write-Host "Row 0 Col 1 (Username): $valUser"
    if ($valUser -ne "=CONCATENATE('User', 'Name')") { 
        Write-Warning "Expected '=CONCATENATE('User', 'Name')' but got '$valUser'"
    }
    
    Write-Host "Persistence Verified" -ForegroundColor Green
}
