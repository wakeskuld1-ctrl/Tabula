$baseUrl = "http://localhost:3000"
$boundary = "------------------------boundary123"
$fileContent = "id,name,amount`n1,test_user,100"
$fileName = "test_upload_fix.csv"

# Build multipart body manually because PowerShell's Invoke-WebRequest -Form is tricky with specific boundaries
$LF = "`r`n"
$bodyLines = (
    "--$boundary",
    "Content-Disposition: form-data; name=`"file`"; filename=`"$fileName`"",
    "Content-Type: text/csv",
    "",
    $fileContent,
    "--$boundary--"
) -join $LF

try {
    $resp = Invoke-WebRequest -Uri "$baseUrl/api/upload" -Method Post -ContentType "multipart/form-data; boundary=$boundary" -Body $bodyLines
    Write-Host "Upload Response: $($resp.Content)"
} catch {
    Write-Host "Upload Failed: $_"
    exit 1
}

# Verify query
$queryBody = '{"sql": "SELECT * FROM test_upload_fix LIMIT 1"}'
try {
    $query_resp = Invoke-WebRequest -Uri "$baseUrl/api/execute" -Method Post -ContentType "application/json" -Body $queryBody
    Write-Host "Query Response: $($query_resp.Content)"
} catch {
    Write-Host "Query Failed: $_"
    exit 1
}
