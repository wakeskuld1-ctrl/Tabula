$baseUrl = "http://localhost:3000"
$tableName = "users"

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

# 1. Ensure Session is Active (Hydrate)
# We can trigger this by just accessing grid data or update cell.
# Let's try to update a known cell.

Test-Step "Update Age (Int64) with Number" {
    $body = @{
        table_name = $tableName
        row_idx = 0
        col_idx = 2  # 'age' is likely column 2 (0:user_id, 1:username, 2:age, 3:is_active)
        col_name = "age"
        old_value = ""
        new_value = "30"
    } | ConvertTo-Json

    $res = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $body -ContentType "application/json"
    if ($res.status -ne "ok") { 
        Write-Host "Response: $($res | ConvertTo-Json -Depth 5)"
        throw "Failed to set age to 30" 
    }
}

Test-Step "Update Age (Int64) with Formula (Trigger Type Promotion)" {
    # This tests the fix where Int64 column accepts a formula string by promoting to Utf8
    $formula = "=SUM(10,20)"
    $body = @{
        table_name = $tableName
        row_idx = 0
        col_idx = 2
        col_name = "age"
        old_value = "30"
        new_value = $formula
    } | ConvertTo-Json

    $res = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $body -ContentType "application/json"
    if ($res.status -ne "ok") { 
        Write-Host "Response: $($res | ConvertTo-Json -Depth 5)"
        throw "Failed to save formula in numeric column" 
    }
}

Test-Step "Update Username (String) with COUNT Formula" {
    # username is col 1
    $formula = "=COUNT(A1:A5)"
    $body = @{
        table_name = $tableName
        row_idx = 0
        col_idx = 1
        col_name = "username"
        old_value = ""
        new_value = $formula
    } | ConvertTo-Json

    $res = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $body -ContentType "application/json"
    if ($res.status -ne "ok") { 
        Write-Host "Response: $($res | ConvertTo-Json -Depth 5)"
        throw "Failed to save formula in string column" 
    }
}

Test-Step "Verify Persistence" {
    # Get grid data to confirm values
    # We need to use the session_id if possible, but let's assume default session for table if session_id is omitted or handle it.
    # To be safe, we'll list sessions or just use the table_name which defaults to latest session in get_grid_data logic? 
    # Actually get_grid_data switches session if provided. If not, it uses active?
    # Let's try without session_id first.
    
    $gridUrl = "$baseUrl/api/grid-data?table_name=$tableName&page=1&page_size=10"
    $grid = Invoke-RestMethod -Uri $gridUrl -Method Get
    
    # Check Row 0, Col 2 (Age)
    $row0 = $grid.data[0]
    $ageVal = $row0[2]
    if ($ageVal -ne "=SUM(10,20)") {
        throw "Age Expected '=SUM(10,20)' but got '$ageVal'"
    }
    
    # Check Row 0, Col 1 (Username)
    $userVal = $row0[1]
    if ($userVal -ne "=COUNT(A1:A5)") {
        throw "Username Expected '=COUNT(A1:A5)' but got '$userVal'"
    }
    
    Write-Host "Formulas persisted and types promoted!" -ForegroundColor Green
}
