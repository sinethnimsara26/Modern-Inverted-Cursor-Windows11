param(
    [string]$Arch = "",
    [string]$Version = "",
    [string]$Toolchain = "",
    [string]$Target = "",
    [string]$Suffix = "",
    [switch]$StaticCrt
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$TargetRoot = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root "target" }
$Dist = Join-Path $Root "dist"
New-Item -ItemType Directory -Path $Dist -Force | Out-Null

if ([string]::IsNullOrWhiteSpace($Arch)) {
    if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
        $Arch = "arm64"
    } else {
        $Arch = "x64"
    }
}
if ([string]::IsNullOrWhiteSpace($Suffix)) {
    $Suffix = $Arch
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $toml = Get-Content (Join-Path $Root "Cargo.toml") -Raw
    if ($toml -match 'version\s*=\s*"([^"]+)"') {
        $Version = $Matches[1]
    } else {
        $Version = "1.0.2"
    }
}

$CargoPrefix = @()
if (-not [string]::IsNullOrWhiteSpace($Toolchain)) {
    $CargoPrefix += "+$Toolchain"
}

$TargetArgs = @()
$TargetSubdir = "release"
if (-not [string]::IsNullOrWhiteSpace($Target)) {
    $TargetArgs += @("--target", $Target)
    $TargetSubdir = "$Target\release"
}

$PrevRustFlags = $env:RUSTFLAGS
if ($StaticCrt) {
    $env:RUSTFLAGS = if ([string]::IsNullOrWhiteSpace($PrevRustFlags)) {
        "-C target-feature=+crt-static"
    } else {
        "$PrevRustFlags -C target-feature=+crt-static"
    }
}

$GuardianName = if ($Suffix -eq "x64") { "ModernInvertedCursorGuardian.exe" } else { "ModernInvertedCursorGuardian-$Suffix.exe" }
$GuardianTagged = "ModernInvertedCursorGuardian-$Suffix.exe"
$SetupOut = Join-Path $Dist "ModernInvertedSetup-v$Version-$Suffix.exe"
$GuardianOut = Join-Path $Dist $GuardianName
$GuardianArchCopy = Join-Path $Dist $GuardianTagged

Push-Location $Root
try {
    Write-Host "[1/4] Building ModernInvertedCursorGuardian ($Suffix, release)..." -ForegroundColor Cyan
    & cargo @CargoPrefix build --release @TargetArgs --bin ModernInvertedCursorGuardian
    if ($LASTEXITCODE -ne 0) { throw "Guardian build failed." }
    $BuiltGuardian = Join-Path $TargetRoot "$TargetSubdir\ModernInvertedCursorGuardian.exe"
    Copy-Item $BuiltGuardian $GuardianOut -Force
    if ($GuardianArchCopy -ne $GuardianOut) {
        Copy-Item $BuiltGuardian $GuardianArchCopy -Force
    }

    $env:GUARDIAN_EMBED_PATH = $GuardianOut

    Write-Host "[2/4] Building ModernInvertedSetup v$Version ($Suffix, release)..." -ForegroundColor Cyan
    & cargo @CargoPrefix build --release @TargetArgs --bin ModernInvertedSetup
    if ($LASTEXITCODE -ne 0) { throw "Setup build failed." }
    Copy-Item (Join-Path $TargetRoot "$TargetSubdir\ModernInvertedSetup.exe") $SetupOut -Force

    # Keep dist clean of superseded setup installers for this exact suffix (and untagged v1.0.0 filenames)
    Get-ChildItem $Dist -Filter "ModernInvertedSetup-v*-$Suffix.exe" |
        Where-Object FullName -ne $SetupOut |
        Remove-Item -Force
    Get-ChildItem $Dist -Filter "ModernInvertedSetup-v*.exe" |
        Where-Object { $_.Name -match "^ModernInvertedSetup-v[0-9.]+\.exe$" -and $_.FullName -ne $SetupOut } |
        Remove-Item -Force

    Write-Host "[3/4] Running unit tests ($Suffix)..." -ForegroundColor Cyan
    & cargo @CargoPrefix test @TargetArgs
    if ($LASTEXITCODE -ne 0) { throw "Tests failed." }

    Write-Host "[4/4] Generating SHA256SUMS.txt..." -ForegroundColor Cyan
    Get-ChildItem $Dist -Filter "*.pdb" | Remove-Item -Force
    $Sums = Get-ChildItem $Dist -Filter "*.exe" | Sort-Object Name | ForEach-Object {
        "$((Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower())  $($_.Name)"
    }
    $Sums | Set-Content (Join-Path $Dist "SHA256SUMS.txt") -Encoding ascii

    Write-Host "`nBuild & tests succeeded ($Suffix):" -ForegroundColor Green
    Get-Item $GuardianOut, $SetupOut | Format-List Name, Length, LastWriteTime
    Get-Content (Join-Path $Dist "SHA256SUMS.txt")
} finally {
    Remove-Item Env:GUARDIAN_EMBED_PATH -ErrorAction SilentlyContinue
    if ($null -eq $PrevRustFlags) {
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
    } else {
        $env:RUSTFLAGS = $PrevRustFlags
    }
    Pop-Location
}
