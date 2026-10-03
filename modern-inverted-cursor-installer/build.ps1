$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$Source = Join-Path $ProjectRoot "final\modern_inverted_installer_v3_force_live.rs"
$Dist = Join-Path $ProjectRoot "dist"
$Exe = Join-Path $Dist "ModernInvertedInstallerV3.exe"

if (-not (Test-Path $Source)) {
    throw "Final Rust source not found: $Source"
}

# Refresh this shell's PATH in case Rustup was installed after PowerShell was opened.
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
            [System.Environment]::GetEnvironmentVariable("Path","User")

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host "Rustup is not installed. Installing it with winget..."
    winget install -e --id Rustlang.Rustup
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
                [System.Environment]::GetEnvironmentVariable("Path","User")
}

Write-Host "Ensuring the stable GNU Windows Rust toolchain is installed..."
rustup toolchain install stable-gnu

New-Item -ItemType Directory -Path $Dist -Force | Out-Null

Write-Host "Compiling final installer..."
rustc +stable-gnu -O $Source -o $Exe

if ($LASTEXITCODE -ne 0 -or -not (Test-Path $Exe)) {
    throw "Rust compilation failed."
}

$Hash = Get-FileHash $Exe -Algorithm SHA256
"$($Hash.Hash.ToLower())  ModernInvertedInstallerV3.exe" |
    Set-Content (Join-Path $Dist "SHA256SUMS.txt") -Encoding ascii

Write-Host ""
Write-Host "Build succeeded:" -ForegroundColor Green
Get-Item $Exe | Format-List FullName,Length,LastWriteTime
Write-Host "SHA-256: $($Hash.Hash.ToLower())"
