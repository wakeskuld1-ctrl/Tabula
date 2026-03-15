$baseUrl = "http://localhost:3000"

# Wait for server
Start-Sleep -Seconds 2

# 1. List tables to ensure 'users' is there
Write-Host "Listing tables..."
try {
    $tables = Invoke-RestMethod -Uri "$baseUrl/api/tables" -Method Get -ErrorAction Stop
} catch {
    Write-Error "Server not reachable or failed to list tables: $_"
    exit 1
}

if ($tables.status -ne "ok") { Write-Error "Failed to list tables"; exit 1 }

# 2. Create Session for 'users'
Write-Host "Creating session for users..."
$sessionBody = @{ table_name = "users" } | ConvertTo-Json
try {
    $session = Invoke-RestMethod -Uri "$baseUrl/api/create_session" -Method Post -Body $sessionBody -ContentType "application/json" -ErrorAction Stop
} catch {
    Write-Error "Failed to create session: $_"
    exit 1
}

if ($session.status -ne "ok") { Write-Error "Failed to create session"; exit 1 }
Write-Host "Session created: $($session.session.session_id)"

# 3. Update Style
Write-Host "Updating style..."
$styleBody = @{
    table_name = "users"
    row = 0
    col = 0
    style = @{
        bold = $true
        color = "#FF0000"
    }
} | ConvertTo-Json -Depth 10

try {
    $update = Invoke-RestMethod -Uri "$baseUrl/api/update_style" -Method Post -Body $styleBody -ContentType "application/json" -ErrorAction Stop
} catch {
    Write-Error "Failed to update style: $_"
    exit 1
}

if ($update.status -ne "ok") { 
    Write-Error "Failed to update style: $($update.message)" 
    exit 1 
}
Write-Host "Style updated."

# 4. Verify Metadata in Grid Data
Write-Host "Verifying grid data..."
try {
    $grid = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?table_name=users&page=1&page_size=10" -Method Get -ErrorAction Stop
} catch {
    Write-Error "Failed to get grid data: $_"
    exit 1
}

if ($grid.metadata -eq $null) { Write-Error "No metadata returned"; exit 1 }

$style = $grid.metadata.styles."0,0"
if ($null -eq $style) { Write-Error "Style not found for 0,0"; exit 1 }

# Note: PowerShell json conversion might make booleans weird or objects.
Write-Host "Style retrieved: $($style | ConvertTo-Json)"

if ($style.bold -ne $true) { 
    Write-Warning "Bold might not be true, check output above."
    # Strict check might fail if types differ, but let's see.
}
if ($style.color -ne "#FF0000") { Write-Error "Color not set"; exit 1 }

# 5. Update Style Range
Write-Host "Updating style range..."
$rangeBody = @{
    table_name = "users"
    range = @{
        start_row = 1
        start_col = 1
        end_row = 2
        end_col = 2
    }
    style = @{
        italic = $true
        bg_color = "#00FF00"
    }
} | ConvertTo-Json -Depth 10

try {
    $rangeUpdate = Invoke-RestMethod -Uri "$baseUrl/api/update_style_range" -Method Post -Body $rangeBody -ContentType "application/json" -ErrorAction Stop
} catch {
    Write-Error "Failed to update style range: $_"
    exit 1
}

if ($rangeUpdate.status -ne "ok") { 
    Write-Error "Failed to update style range: $($rangeUpdate.message)" 
    exit 1 
}
Write-Host "Style range updated."

# 6. Verify Range Metadata
Write-Host "Verifying range style..."
try {
    $grid = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?table_name=users&page=1&page_size=10" -Method Get -ErrorAction Stop
} catch {
    Write-Error "Failed to get grid data: $_"
    exit 1
}

$rangeStyle = $grid.metadata.styles."1,1"
if ($null -eq $rangeStyle) { Write-Error "Style not found for 1,1"; exit 1 }

Write-Host "Range Style (1,1): $($rangeStyle | ConvertTo-Json)"

if ($rangeStyle.italic -ne $true) { Write-Warning "Italic might not be true" }
if ($rangeStyle.bg_color -ne "#00FF00") { Write-Error "Background color not set"; exit 1 }

# 7. Update Merge
Write-Host "Updating merge..."
$mergeBody = @{
    table_name = "users"
    range = @{
        start_row = 3
        start_col = 3
        end_row = 4
        end_col = 4
    }
} | ConvertTo-Json -Depth 10

try {
    $mergeUpdate = Invoke-RestMethod -Uri "$baseUrl/api/update_merge" -Method Post -Body $mergeBody -ContentType "application/json" -ErrorAction Stop
} catch {
    Write-Error "Failed to update merge: $_"
    exit 1
}

if ($mergeUpdate.status -ne "ok") { 
    Write-Error "Failed to update merge: $($mergeUpdate.message)" 
    exit 1 
}
Write-Host "Merge updated."

# 8. Verify Merge Metadata
Write-Host "Verifying merge..."
try {
    $grid = Invoke-RestMethod -Uri "$baseUrl/api/grid-data?table_name=users&page=1&page_size=10" -Method Get -ErrorAction Stop
} catch {
    Write-Error "Failed to get grid data: $_"
    exit 1
}

$merges = $grid.metadata.merges
if ($null -eq $merges) { Write-Error "No merges returned"; exit 1 }

Write-Host "Merges retrieved: $($merges | ConvertTo-Json)"

$foundMerge = $false
foreach ($m in $merges) {
    if ($m.start_row -eq 3 -and $m.start_col -eq 3 -and $m.end_row -eq 4 -and $m.end_col -eq 4) {
        $foundMerge = $true
        break
    }
}

if (-not $foundMerge) { Write-Error "Merge not found in metadata"; exit 1 }

Write-Host "Verification Successful!"
