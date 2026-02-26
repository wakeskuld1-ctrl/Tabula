# Set source and destination paths
$sourceDir = "D:\Rust\metadata"
$destDir = "D:\Rust\metadata\dist"

# Create destination directory if it doesn't exist
if (-not (Test-Path -Path $destDir)) {
    New-Item -ItemType Directory -Path $destDir | Out-Null
    Write-Host "Created directory: $destDir"
} else {
    Write-Host "Cleaning existing directory: $destDir"
    Remove-Item -Path "$destDir\*" -Recurse -Force
}

# Define files/folders to include
$includeList = @(
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    ".gitignore",
    "generate_report.py",
    "federated_query_engine",
    "metadata_store"
)

# Copy items
foreach ($item in $includeList) {
    $srcPath = Join-Path $sourceDir $item
    $dstPath = Join-Path $destDir $item

    if (Test-Path $srcPath) {
        Write-Host "Copying $item..."
        Copy-Item -Path $srcPath -Destination $dstPath -Recurse -Force
    } else {
        Write-Warning "Item not found: $srcPath"
    }
}

# Create empty data directory
New-Item -ItemType Directory -Path (Join-Path $destDir "data") | Out-Null
New-Item -ItemType File -Path (Join-Path $destDir "data\.keep") | Out-Null

# Clean up build artifacts from the destination (just in case they were copied)
$cleanupItems = @(
    "target",
    "federated_query_engine\target",
    "metadata_store\target",
    "federated_query_engine\cache",
    "federated_query_engine\*.db",
    "federated_query_engine\*.log",
    "federated_query_engine\*.csv",
    "federated_query_engine\*.html",
    "federated_query_engine\*.json"
)

foreach ($pattern in $cleanupItems) {
    $path = Join-Path $destDir $pattern
    if (Test-Path $path) {
        Write-Host "Cleaning up: $path"
        Remove-Item -Path $path -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "Done! Lightweight source package created at: $destDir"
