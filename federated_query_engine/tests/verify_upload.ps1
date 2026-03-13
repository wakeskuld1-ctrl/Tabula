
$baseUrl = "http://localhost:3000"
$csvFile = "smoke_test_ps.csv"
$csvContent = "id,name,score`n1,TestA,90`n2,TestB,80"
$csvContent | Out-File -FilePath $csvFile -Encoding ascii

function Wait-For-Server {
    Write-Host "Waiting for server..."
    for ($i=0; $i -lt 30; $i++) {
        try {
            $resp = Invoke-WebRequest -Uri "$baseUrl/health" -Method Get -ErrorAction Stop
            if ($resp.StatusCode -eq 200) {
                Write-Host "Server is up!"
                return $true
            }
        } catch {
            Start-Sleep -Seconds 1
        }
    }
    Write-Host "Server failed to start."
    return $false
}

if (-not (Wait-For-Server)) { exit 1 }

# Upload
Write-Host "Uploading CSV..."
$url = "$baseUrl/api/upload"
$boundary = [System.Guid]::NewGuid().ToString() 
$LF = "`r`n"

$fileBytes = [System.IO.File]::ReadAllBytes((Resolve-Path $csvFile))
$fileEnc = [System.Text.Encoding]::GetEncoding('iso-8859-1').GetString($fileBytes)

$bodyLines = ( 
    "--$boundary",
    "Content-Disposition: form-data; name=`"file`"; filename=`"$csvFile`"",
    "Content-Type: text/csv",
    "",
    $fileEnc,
    "--$boundary--"
) -join $LF

try {
    $resp = Invoke-WebRequest -Uri $url -Method Post -ContentType "multipart/form-data; boundary=$boundary" -Body $bodyLines
    Write-Host "Upload Response: $($resp.Content)"
} catch {
    Write-Error "Upload failed: $_"
    exit 1
}

# Query
Write-Host "Querying..."
$tableName = "smoke_test_ps"
$queryBody = @{
    sql = "SELECT * FROM $tableName"
} | ConvertTo-Json

try {
    $resp = Invoke-WebRequest -Uri "$baseUrl/api/execute" -Method Post -ContentType "application/json" -Body $queryBody
    Write-Host "Query Response: $($resp.Content)"
    
    $json = $resp.Content | ConvertFrom-Json
    if ($json.rows.Count -eq 2) {
        Write-Host "SUCCESS: Got 2 rows as expected."
    } else {
        Write-Error "FAILURE: Expected 2 rows, got $($json.rows.Count)"
        exit 1
    }
} catch {
    Write-Error "Query failed: $_"
    exit 1
}
