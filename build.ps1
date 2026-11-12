$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$TargetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root "target" }
$Dist = Join-Path $Root "dist"
New-Item -ItemType Directory -Path $Dist -Force | Out-Null

$GuardianOut = Join-Path $Dist "ModernInvertedCursorGuardian.exe"
$SetupOut = Join-Path $Dist "ModernInvertedSetup-v1.0.0.exe"

Push-Location $Root
try {
    Write-Host "[1/4] Building ModernInvertedCursorGuardian (release)..." -ForegroundColor Cyan
    cargo build --release --bin ModernInvertedCursorGuardian
    if ($LASTEXITCODE -ne 0) { throw "Guardian build failed." }
    Copy-Item (Join-Path $TargetRoot "release\ModernInvertedCursorGuardian.exe") $GuardianOut -Force

    Write-Host "[2/4] Building ModernInvertedSetup (release)..." -ForegroundColor Cyan
    cargo build --release --bin ModernInvertedSetup
    if ($LASTEXITCODE -ne 0) { throw "Setup build failed." }
    Copy-Item (Join-Path $TargetRoot "release\ModernInvertedSetup.exe") $SetupOut -Force

    Write-Host "[3/4] Running unit tests..." -ForegroundColor Cyan
    cargo test
    if ($LASTEXITCODE -ne 0) { throw "Tests failed." }

    Write-Host "[4/4] Generating SHA256SUMS.txt..." -ForegroundColor Cyan
    Get-ChildItem $Dist -Filter "*.pdb" | Remove-Item -Force
    $Sums = @(
        "$((Get-FileHash $GuardianOut -Algorithm SHA256).Hash.ToLower())  ModernInvertedCursorGuardian.exe",
        "$((Get-FileHash $SetupOut -Algorithm SHA256).Hash.ToLower())  ModernInvertedSetup-v1.0.0.exe"
    )
    $Sums | Set-Content (Join-Path $Dist "SHA256SUMS.txt") -Encoding ascii

    Write-Host "`nBuild & tests succeeded:" -ForegroundColor Green
    Get-Item $GuardianOut, $SetupOut | Format-List Name, Length, LastWriteTime
    Get-Content (Join-Path $Dist "SHA256SUMS.txt")
} finally {
    Pop-Location
}
