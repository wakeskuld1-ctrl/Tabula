
$baseUrl = "http://127.0.0.1:3000"
$tableName = "orders"
$newValue = "777"

Write-Host "1. Fetching tables..."
try {
    $tablesRes = Invoke-RestMethod -Uri "$baseUrl/api/tables" -Method Get
    if ($tablesRes.tables.table_name -notcontains $tableName) {
        Write-Error "Table '$tableName' not found."
        exit 1
    }
    Write-Host "   Table '$tableName' found."
} catch {
    Write-Error "Failed to fetch tables: $_"
    exit 1
}

Write-Host "2. Creating Session A (Base)..."
$createBodyA = @{
    table_name = $tableName
    session_name = "SessionA"
} | ConvertTo-Json

try {
    $createResA = Invoke-RestMethod -Uri "$baseUrl/api/create_session" -Method Post -Body $createBodyA -ContentType "application/json"
    $sessionAId = $createResA.session.session_id
    Write-Host "   Session A created: $sessionAId"
} catch {
    Write-Error "Failed to create Session A: $_"
    exit 1
}

Write-Host "3. Creating Session B (Target)..."
$createBodyB = @{
    table_name = $tableName
    session_name = "SessionB"
} | ConvertTo-Json

try {
    $createResB = Invoke-RestMethod -Uri "$baseUrl/api/create_session" -Method Post -Body $createBodyB -ContentType "application/json"
    $sessionBId = $createResB.session.session_id
    Write-Host "   Session B created: $sessionBId"
} catch {
    Write-Error "Failed to create Session B: $_"
    exit 1
}

Write-Host "4. Reading initial data (Session B)..."
$gridUrlB = "$baseUrl/api/grid-data?session_id=$sessionBId&table_name=$tableName&page=1&page_size=100"
try {
    $gridRes = Invoke-RestMethod -Uri $gridUrlB -Method Get
    $columns = $gridRes.columns
    $rows = $gridRes.data
    
    $amountIdx = $columns.IndexOf("amount")
    if ($amountIdx -eq -1) {
        Write-Error "Column 'amount' not found."
        exit 1
    }
    
    $rowIdx = 0
    $oldValue = $rows[$rowIdx][$amountIdx]
    Write-Host "   Initial value at Row $rowIdx, Col 'amount': $oldValue"
} catch {
    Write-Error "Failed to read grid data: $_"
    exit 1
}

Write-Host "5. Updating Session B..."
$updateBody = @{
    session_id = $sessionBId
    table_name = $tableName
    row_idx = $rowIdx
    col_idx = $amountIdx
    col_name = "amount"
    old_value = [string]$oldValue
    new_value = $newValue
} | ConvertTo-Json

try {
    $updateRes = Invoke-RestMethod -Uri "$baseUrl/api/update_cell" -Method Post -Body $updateBody -ContentType "application/json"
    if ($updateRes.status -ne "ok") {
        Write-Error "Update failed: $($updateRes.message)"
        exit 1
    }
    Write-Host "   Update success."
} catch {
    Write-Error "Failed to update cell: $_"
    exit 1
}

Write-Host "6. Verifying Session B (Read after Write)..."
try {
    $verifyRes = Invoke-RestMethod -Uri $gridUrlB -Method Get
    $updatedValue = $verifyRes.data[$rowIdx][$amountIdx]
    
    Write-Host "   Session B value: $updatedValue"
    
    if ([string]$updatedValue -eq $newValue -or [string]$updatedValue -eq "$newValue.0") {
        Write-Host "   SUCCESS: Session B updated correctly."
    } else {
        Write-Error "   FAILURE: Session B mismatch. Expected '$newValue', got '$updatedValue'."
        exit 1
    }
} catch {
    Write-Error "Failed to verify update: $_"
    exit 1
}

Write-Host "7. Verifying Isolation (Session A)..."
$gridUrlA = "$baseUrl/api/grid-data?session_id=$sessionAId&table_name=$tableName&page=1&page_size=100"
try {
    $verifyResA = Invoke-RestMethod -Uri $gridUrlA -Method Get
    $valueA = $verifyResA.data[$rowIdx][$amountIdx]
    
    Write-Host "   Session A value: $valueA"
    
    # Session A should still have the old value (100.0)
    # assuming $oldValue was 100.0
    
    if ([string]$valueA -eq [string]$oldValue) {
        Write-Host "   SUCCESS: Session A is isolated (value unchanged)."
    } else {
        Write-Error "   FAILURE: Session A was modified! Expected '$oldValue', got '$valueA'."
        exit 1
    }
} catch {
    Write-Error "Failed to verify isolation: $_"
    exit 1
}

Write-Host "Test Complete."
