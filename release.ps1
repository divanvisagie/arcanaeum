# Check if ghr is installed
$ghrPath = "C:\tools\ghr.exe"
if (-Not (Test-Path $ghrPath)) {
    # Download and extract ghr
    Invoke-WebRequest -Uri "https://github.com/tcnksm/ghr/releases/download/v0.13.0/ghr_v0.13.0_windows_amd64.zip" -OutFile "ghr.zip"
    Expand-Archive -Path "ghr.zip" -DestinationPath "C:\tools" -Force
    Move-Item -Path "C:\tools\ghr_v0.13.0_windows_amd64\ghr.exe" -Destination $ghrPath -Force
}

# Check if PowerShellGet is installed
if (-Not (Get-Module -ListAvailable -Name PowerShellGet)) {
    # Install PowerShellGet
    Install-Module -Name PowerShellGet -Force -SkipPublisherCheck
}

# GitHub Personal Access Token from environment variable
$token = [System.Environment]::GetEnvironmentVariable('GH_TOKEN', [System.EnvironmentVariableTarget]::User)

if ([string]::IsNullOrWhiteSpace($token)) {
    Write-Host "GitHub token is not set. Please set the GH_TOKEN environment variable."
    Exit
}

# Read the version from Cargo.toml
$cargoContent = Get-Content -Path ".\Cargo.toml"
$versionLine = $cargoContent | Select-String -Pattern 'version = ".*"' | ForEach-Object { $_.Matches[0].Value }
$version = ($versionLine -split '"')[1]

if ([string]::IsNullOrWhiteSpace($version)) {
    Write-Host "Version not found in Cargo.toml."
    Exit
}

# Repository and owner information
$owner = "divanvisagie"
$repo = "Arcanaeum"

# Asset to upload
$assetPath = ".\target\release\arcanaeum.exe"

& $ghrPath -t $token -u $owner -r $repo -c $version -n "Release v$version" -b "Release notes for v$version" -replace "v$version" $assetPath

Write-Host "Release and asset upload completed."
