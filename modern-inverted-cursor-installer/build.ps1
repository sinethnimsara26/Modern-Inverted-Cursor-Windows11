$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$Source = Join-Path $ProjectRoot "final\modern_inverted_installer_v5_1_balanced.rs"
$Dist = Join-Path $ProjectRoot "dist"
$Exe = Join-Path $Dist "ModernInvertedInstallerV5_1.exe"

if (-not (Test-Path $Source)) { throw "Final Rust source not found: $Source" }

$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
            [System.Environment]::GetEnvironmentVariable("Path","User")

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    winget install -e --id Rustlang.Rustup
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
                [System.Environment]::GetEnvironmentVariable("Path","User")
}

rustup toolchain install stable-gnu
New-Item -ItemType Directory -Path $Dist -Force | Out-Null

rustc +stable-gnu -O $Source -o $Exe
if ($LASTEXITCODE -ne 0 -or -not (Test-Path $Exe)) { throw "Rust compilation failed." }

Get-ChildItem $Dist -Filter "ModernInvertedInstallerV*.exe" |
    Where-Object Name -ne "ModernInvertedInstallerV5_1.exe" |
    Remove-Item -Force

$Hash = Get-FileHash $Exe -Algorithm SHA256
"$($Hash.Hash.ToLower())  ModernInvertedInstallerV5_1.exe" |
    Set-Content (Join-Path $Dist "SHA256SUMS.txt") -Encoding ascii

Write-Host "Build succeeded:" -ForegroundColor Green
Get-Item $Exe | Format-List FullName,Length,LastWriteTime
Write-Host "SHA-256: $($Hash.Hash.ToLower())"
