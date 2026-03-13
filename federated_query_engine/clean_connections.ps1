
$conns = Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/connections" -Method Get
$conns.connections | ForEach-Object {
    $id = $_.id
    Write-Host "Deleting connection: $id"
    Invoke-RestMethod -Uri "http://127.0.0.1:3000/api/connections/$id" -Method Delete
}
Write-Host "All connections deleted."
