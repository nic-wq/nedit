# NEdit Windows Installation Script
# This script downloads the latest nedit_windows.exe and installs it as 'nedit' in the user's PATH.

$Repo = "nic-wq/nedit"
$BinaryPattern = "nedit_windows.exe"
$InstallDir = Join-Path $env:LOCALAPPDATA "nedit\bin"
$InstallPath = Join-Path $InstallDir "nedit.exe"

# 1. Prepare Environment
Write-Host "--- NEdit Installation for Windows ---" -ForegroundColor Cyan
Write-Host "Preparing installation directory..."
if (!(Test-Path $InstallDir)) {
    try {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        Write-Host "Created: $InstallDir" -ForegroundColor Gray
    } catch {
        Write-Host "Error: Could not create installation directory. $_" -ForegroundColor Red
        exit 1
    }
}

# 2. Fetch Latest Version from GitHub
Write-Host "Fetching latest release information..."
try {
    $ApiUrl = "https://api.github.com/repos/$Repo/releases/latest"
    $ReleaseData = Invoke-RestMethod -Uri $ApiUrl -UseBasicParsing
    $Asset = $ReleaseData.assets | Where-Object { $_.name -eq $BinaryPattern } | Select-Object -First 1

    if ($null -eq $Asset) {
        Write-Host "Error: Could not find '$BinaryPattern' in the latest release of $Repo." -ForegroundColor Red
        exit 1
    }
    $DownloadUrl = $Asset.browser_download_url
    $Version = $ReleaseData.tag_name
} catch {
    Write-Host "Error: Failed to connect to GitHub API. Check your internet connection." -ForegroundColor Red
    exit 1
}

# 3. Download and Install
Write-Host "Downloading NEdit $Version..." -ForegroundColor Yellow
try {
    # Using a temporary file for the download
    $TempFile = Join-Path $env:TEMP "nedit_download.exe"
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempFile -UserAgent "Mozilla/5.0"
    
    # Move and rename to final destination
    if (Test-Path $InstallPath) {
        Remove-Item $InstallPath -Force
    }
    Move-Item -Path $TempFile -Destination $InstallPath -Force
    Write-Host "Binary installed successfully to $InstallPath" -ForegroundColor Green
} catch {
    Write-Host "Error during download or file operation: $_" -ForegroundColor Red
    exit 1
}

# 4. Update PATH
Write-Host "Updating user PATH..."
try {
    $UserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        $NewPath = "$UserPath;$InstallDir"
        [System.Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
        Write-Host "Added $InstallDir to User PATH." -ForegroundColor Gray
    } else {
        Write-Host "NEdit path already in PATH." -ForegroundColor Gray
    }
} catch {
    Write-Host "Warning: Could not automatically update PATH. You may need to add $InstallDir manually." -ForegroundColor Yellow
}

# 5. Final Message
Write-Host ""
Write-Host "*****************************************************" -ForegroundColor Green
Write-Host "  Installation completed successfully!" -ForegroundColor Green
Write-Host "*****************************************************" -ForegroundColor Green
Write-Host "To start using NEdit:"
Write-Host "1. RESTART your terminal (close and open again)."
Write-Host "2. Type 'nedit' and press Enter."
Write-Host ""
