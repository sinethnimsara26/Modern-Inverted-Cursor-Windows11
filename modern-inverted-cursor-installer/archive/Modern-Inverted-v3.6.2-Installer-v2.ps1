#requires -Version 5.1
<#
Modern Inverted Mouse Cursors v3.6.2 - robust community installer
Source: https://github.com/emvaized/modern_inverted_mouse_cursors
Cursor assets: Copyright (c) emvaized (Max Tsyba), MIT License

This script uses only the public GitHub source tag. It is not the Patreon package.
Designed for Windows PowerShell 5.1 and PowerShell 7+.
#>

[CmdletBinding()]
param(
    [ValidateSet('Regular','NoTail','TailDetached','Compact')]
    [string]$Style,
    [switch]$InstallOnly,
    [switch]$Restore,
    [switch]$NoPause
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$Version = '3.6.2'
$Repo = 'emvaized/modern_inverted_mouse_cursors'
$ArchiveUrl = "https://codeload.github.com/$Repo/zip/refs/tags/$Version"
$InstallRoot = Join-Path $env:LOCALAPPDATA "ModernInvertedCursors\v$Version"
$CursorDir = Join-Path $InstallRoot 'cursors'
$BackupFile = Join-Path $InstallRoot 'previous-cursor-backup.json'
$TempRoot = Join-Path $env:TEMP ("ModernInvertedCursors-" + [Guid]::NewGuid().ToString('N'))
$ZipPath = Join-Path $TempRoot 'source.zip'
$LogPath = Join-Path $env:TEMP ("Modern-Inverted-Installer-" + (Get-Date -Format 'yyyyMMdd-HHmmss') + '.log')
$TranscriptStarted = $false

$Expected = @{
    'arrow.cur'                    = @('2246','325739ebbc8a589aa757b7a7f19194bd0e6afd3a')
    'arrow_no_tail.cur'            = @('2384','8a503406723a840324b966c46074ca7257b8b145')
    'arrow_no_tail_smaller.cur'    = @('2246','daf30b0336f061a1a7961e1dd0dea9d1ce7ccd40')
    'arrow_no_tail_smaller_v2.cur' = @('2246','5e7990e298d85a4d8eb205e0b64a2b7d06b3f2b1')
    'arrow_tail_detached.cur'      = @('2246','aa8082e301909e7bbf0bee1fdeb47db961f6d34a')
    'beam.cur'                     = @('2246','87c55d6e1a8d819ac80e4edd1d8afe4e35846f5a')
    'beam_v2.cur'                  = @('2246','b092bcf3b77e8cd9413c52e71d9a0a64afabaa69')
    'busy.ani'                     = @('5474','fa36f833b346a3d6310458d0a6bc30ec1c8657ad')
    'cross.cur'                    = @('2246','247947ea4293cb80fe60fe7143ae5c8d9633b7ad')
    'diagonal_1.cur'               = @('2246','fe4205fd226707e165e1c9bc6ebf040419df33d2')
    'diagonal_2.cur'               = @('2246','2244e646b88022bd3119a16ae7d17ee88aca88a9')
    'help.cur'                     = @('2246','d807f02924caa159d3bebd138af570bd6dd6ae3e')
    'help_no_tail.cur'             = @('2246','fc3985098751b3d6e38f32ae43a9e736cafbdad1')
    'help_tail_detached.cur'       = @('326','b3a1e4dcba4e4b2f24ade81dda0eece44e224d4d')
    'horizontal.cur'               = @('2246','a38b02b149045421513ca7954d07581a6ae61745')
    'horizontal_v2.cur'            = @('2246','8c7888da14a7a13441c56db830877df91024fd7d')
    'link.cur'                     = @('2246','86dae854fc5479674cca5ea25986b87c0ac2f3e0')
    'link_v2.cur'                  = @('2246','01eab41a23bbdaaf9f84e449f846a4f233777b68')
    'move.cur'                     = @('2246','6fdb0c4e3812c1cb896e5c45d4f691de53d80e84')
    'move_v2.cur'                  = @('326','6df1bcd2bea99c47a025d9f4dd03db9a3a6b3a98')
    'pen.cur'                      = @('2246','e94d2d6a7182abad2cda2b75434ed37ea7d9ebbc')
    'special.cur'                  = @('2246','dbb6b1fac453fb52ae1cfc87d09ee907373b19d3')
    'unavailable.cur'              = @('8774','70496b18014075e2598f61c87ebd2c790174eb1c')
    'unavailable_v2.cur'           = @('2246','4baceb98c400b4b6836a35d4e77b3fa7e76ad68d')
    'unavailable_v3.cur'           = @('2246','d3e0901c59ec8494d8fc0a12caca418bac8b65ad')
    'vertical.cur'                 = @('2246','f6b8c7d085550f01b23a911d617b817bbac87d7d')
    'vertical_v2.cur'              = @('2246','57b4a71f8fb6ffc70a169c65d3a561887d193aac')
    'working.ani'                  = @('5474','fcced8db8402d254a3f2e1e635bdb50dd334016c')
    'working_no_tail.ani'          = @('5474','0c368f9a121a14c7c581ccfb60c427bf5093f77f')
    'working_tail_detached.ani'    = @('5474','dec0cc444841031357b0b1a1b7189bd61340018a')
    'working_tail_detached_v2.ani' = @('5474','9f3656efe73c669aead22627e1f0bc854bab8b1a')
    'working_tail_detached_v3.ani' = @('2072','c47132bee21ac2941c6db45a72590f1333876907')
}

function Wait-BeforeExit {
    if (-not $NoPause) {
        Write-Host ''
        [void](Read-Host 'Press ENTER to close this window')
    }
}

function Get-GitBlobSha1 {
    param([Parameter(Mandatory=$true)][string]$Path)

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $header = [System.Text.Encoding]::ASCII.GetBytes("blob $($bytes.Length)`0")
    $buffer = New-Object byte[] ($header.Length + $bytes.Length)
    [System.Array]::Copy($header, 0, $buffer, 0, $header.Length)
    [System.Array]::Copy($bytes, 0, $buffer, $header.Length, $bytes.Length)

    $sha1 = [System.Security.Cryptography.SHA1]::Create()
    try {
        $hash = $sha1.ComputeHash($buffer)
        return (($hash | ForEach-Object { $_.ToString('x2') }) -join '')
    }
    finally {
        $sha1.Dispose()
    }
}

function Set-RegistryDefaultValue {
    param([string]$SubKey, [string]$Value)
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($SubKey)
    try {
        $key.SetValue('', $Value, [Microsoft.Win32.RegistryValueKind]::String)
    }
    finally {
        $key.Close()
    }
}

function Get-RegistryDefaultValue {
    param([string]$SubKey)
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($SubKey)
    if ($null -eq $key) { return '' }
    try {
        return [string]$key.GetValue('', '')
    }
    finally {
        $key.Close()
    }
}

function Get-RegistryNamedValue {
    param([string]$Path, [string]$Name)
    try {
        $item = Get-ItemProperty -Path $Path -Name $Name -ErrorAction Stop
        return [string]$item.$Name
    }
    catch {
        return ''
    }
}

function Set-RegistryNamedValue {
    param([string]$Path, [string]$Name, [string]$Value)
    New-ItemProperty -Path $Path -Name $Name -Value $Value -PropertyType String -Force | Out-Null
}

function Reload-Cursors {
    if (-not ('CursorNative' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class CursorNative
{
    [DllImport("user32.dll", SetLastError=true)]
    public static extern bool SystemParametersInfo(uint uiAction, uint uiParam, IntPtr pvParam, uint fWinIni);
}
'@
    }

    # SPI_SETCURSORS = 0x57; SPIF_UPDATEINIFILE | SPIF_SENDCHANGE = 0x03
    [void][CursorNative]::SystemParametersInfo(0x57, 0, [IntPtr]::Zero, 0x03)
}

function Get-CursorPath {
    param([string]$Name)
    return (Join-Path $CursorDir $Name)
}

function Restore-PreviousCursorSetup {
    if (-not (Test-Path -LiteralPath $BackupFile)) {
        throw "No backup exists at: $BackupFile"
    }

    $backup = Get-Content -LiteralPath $BackupFile -Raw | ConvertFrom-Json
    $cursorKey = 'HKCU:\Control Panel\Cursors'
    New-Item -Path $cursorKey -Force | Out-Null

    foreach ($property in $backup.Roles.PSObject.Properties) {
        Set-RegistryNamedValue -Path $cursorKey -Name $property.Name -Value ([string]$property.Value)
    }

    Set-RegistryDefaultValue -SubKey 'Control Panel\Cursors' -Value ([string]$backup.DefaultScheme)
    if ($null -ne $backup.SchemeSource) {
        Set-RegistryNamedValue -Path $cursorKey -Name 'Scheme Source' -Value ([string]$backup.SchemeSource)
    }

    Reload-Cursors
    Write-Host 'Previous cursor configuration restored.' -ForegroundColor Green
}

try {
    try {
        Start-Transcript -Path $LogPath -Force | Out-Null
        $TranscriptStarted = $true
    }
    catch {
        # Logging is helpful but not required for installation.
    }

    Clear-Host
    Write-Host '============================================================' -ForegroundColor DarkCyan
    Write-Host " Modern Inverted Mouse Cursors v$Version - Installer v2" -ForegroundColor Cyan
    Write-Host '============================================================' -ForegroundColor DarkCyan
    Write-Host "PowerShell: $($PSVersionTable.PSVersion)" -ForegroundColor DarkGray
    Write-Host "Log: $LogPath" -ForegroundColor DarkGray
    Write-Host ''

    if ($env:OS -ne 'Windows_NT') {
        throw 'This installer must be run on Windows.'
    }

    if ([string]::IsNullOrWhiteSpace($env:LOCALAPPDATA)) {
        throw 'LOCALAPPDATA is unavailable. Run this from your normal Windows user account.'
    }

    if ($Restore) {
        Restore-PreviousCursorSetup
        Wait-BeforeExit
        return
    }

    # GitHub requires modern TLS. This is especially important in Windows PowerShell 5.1.
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    }
    catch {
        # PowerShell 7 does not need this, so failure here is harmless.
    }

    Write-Host '[1/7] Preparing folders...'
    New-Item -ItemType Directory -Path $TempRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $InstallRoot -Force | Out-Null
    New-Item -ItemType Directory -Path $CursorDir -Force | Out-Null

    Write-Host '[2/7] Backing up your current Windows cursor setup...'
    if (-not (Test-Path -LiteralPath $BackupFile)) {
        $cursorKey = 'HKCU:\Control Panel\Cursors'
        $roles = @('Arrow','Help','AppStarting','Wait','Crosshair','IBeam','NWPen','No','SizeNS','SizeWE','SizeNWSE','SizeNESW','SizeAll','UpArrow','Hand','Pin','Person')
        $values = [ordered]@{}

        foreach ($role in $roles) {
            $values[$role] = Get-RegistryNamedValue -Path $cursorKey -Name $role
        }

        $backup = [ordered]@{
            DefaultScheme = Get-RegistryDefaultValue -SubKey 'Control Panel\Cursors'
            SchemeSource  = Get-RegistryNamedValue -Path $cursorKey -Name 'Scheme Source'
            Roles         = $values
        }

        $backup | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $BackupFile -Encoding UTF8
        Write-Host "      Backup saved: $BackupFile" -ForegroundColor DarkGray
    }
    else {
        Write-Host '      Existing backup kept (not overwritten).' -ForegroundColor DarkGray
    }

    Write-Host '[3/7] Downloading the exact public GitHub v3.6.2 source archive...'
    $webClient = New-Object System.Net.WebClient
    $webClient.Headers.Add('User-Agent', 'Modern-Inverted-Community-Installer/2.0')
    try {
        $webClient.DownloadFile($ArchiveUrl, $ZipPath)
    }
    finally {
        $webClient.Dispose()
    }

    if (-not (Test-Path -LiteralPath $ZipPath)) {
        throw 'GitHub download did not produce the expected ZIP file.'
    }
    if ((Get-Item -LiteralPath $ZipPath).Length -lt 1000) {
        throw 'The downloaded GitHub archive is unexpectedly small.'
    }

    Write-Host '[4/7] Extracting source archive...'
    Expand-Archive -LiteralPath $ZipPath -DestinationPath $TempRoot -Force

    $sourceRoot = Get-ChildItem -LiteralPath $TempRoot -Directory |
        Where-Object { Test-Path -LiteralPath (Join-Path $_.FullName 'src') } |
        Select-Object -First 1

    if ($null -eq $sourceRoot) {
        throw 'Could not locate the repository source folder after extraction.'
    }

    $Src = Join-Path $sourceRoot.FullName 'src'

    Write-Host '[5/7] Verifying and installing all 32 public cursor assets...'
    foreach ($name in ($Expected.Keys | Sort-Object)) {
        $sourceFile = Join-Path $Src $name
        if (-not (Test-Path -LiteralPath $sourceFile)) {
            throw "Missing expected source file: $name"
        }

        $expectedSize = [Int64]$Expected[$name][0]
        $actualSize = (Get-Item -LiteralPath $sourceFile).Length
        if ($actualSize -ne $expectedSize) {
            throw "Integrity check failed for $name : expected $expectedSize bytes, got $actualSize bytes."
        }

        $expectedBlob = [string]$Expected[$name][1]
        $actualBlob = Get-GitBlobSha1 -Path $sourceFile
        if ($actualBlob -ne $expectedBlob) {
            throw "Integrity check failed for $name : Git blob SHA-1 does not match v$Version."
        }

        Copy-Item -LiteralPath $sourceFile -Destination (Join-Path $CursorDir $name) -Force
    }
    Write-Host '      All 32 files verified successfully.' -ForegroundColor Green

    $licenseSource = Join-Path $sourceRoot.FullName 'LICENSE'
    if (Test-Path -LiteralPath $licenseSource) {
        Copy-Item -LiteralPath $licenseSource -Destination (Join-Path $InstallRoot 'LICENSE.txt') -Force
    }

    Write-Host '[6/7] Registering Windows cursor schemes...'
    $windowsCursorDir = Join-Path $env:SystemRoot 'Cursors'

    $pin = Join-Path $windowsCursorDir 'pin_i.cur'
    if (-not (Test-Path -LiteralPath $pin)) { $pin = Join-Path $windowsCursorDir 'pin.cur' }
    if (-not (Test-Path -LiteralPath $pin)) { $pin = '' }

    $person = Join-Path $windowsCursorDir 'person_i.cur'
    if (-not (Test-Path -LiteralPath $person)) { $person = Join-Path $windowsCursorDir 'person.cur' }
    if (-not (Test-Path -LiteralPath $person)) { $person = '' }

    $common = [ordered]@{
        Wait       = Get-CursorPath 'busy.ani'
        Crosshair  = Get-CursorPath 'cross.cur'
        IBeam      = Get-CursorPath 'beam.cur'
        NWPen      = Get-CursorPath 'pen.cur'
        No         = Get-CursorPath 'unavailable.cur'
        SizeNS     = Get-CursorPath 'vertical.cur'
        SizeWE     = Get-CursorPath 'horizontal.cur'
        SizeNWSE   = Get-CursorPath 'diagonal_1.cur'
        SizeNESW   = Get-CursorPath 'diagonal_2.cur'
        SizeAll    = Get-CursorPath 'move.cur'
        UpArrow    = Get-CursorPath 'special.cur'
        Hand       = Get-CursorPath 'link.cur'
        Pin        = $pin
        Person     = $person
    }

    $styles = [ordered]@{
        Regular = [ordered]@{
            Name = "Modern Inverted v$Version - Regular"
            Arrow = Get-CursorPath 'arrow.cur'
            Help = Get-CursorPath 'help.cur'
            AppStarting = Get-CursorPath 'working.ani'
        }
        NoTail = [ordered]@{
            Name = "Modern Inverted v$Version - No Tail"
            Arrow = Get-CursorPath 'arrow_no_tail.cur'
            Help = Get-CursorPath 'help_no_tail.cur'
            AppStarting = Get-CursorPath 'working_no_tail.ani'
        }
        TailDetached = [ordered]@{
            Name = "Modern Inverted v$Version - Tail Detached"
            Arrow = Get-CursorPath 'arrow_tail_detached.cur'
            Help = Get-CursorPath 'help_tail_detached.cur'
            AppStarting = Get-CursorPath 'working_tail_detached_v3.ani'
        }
        Compact = [ordered]@{
            Name = "Modern Inverted v$Version - Compact"
            Arrow = Get-CursorPath 'arrow_no_tail_smaller_v2.cur'
            Help = Get-CursorPath 'help_no_tail.cur'
            AppStarting = Get-CursorPath 'working_tail_detached_v3.ani'
        }
    }

    $orderedRoles = @('Arrow','Help','AppStarting','Wait','Crosshair','IBeam','NWPen','No','SizeNS','SizeWE','SizeNWSE','SizeNESW','SizeAll','UpArrow','Hand','Pin','Person')
    $schemeKey = 'HKCU:\Control Panel\Cursors\Schemes'
    New-Item -Path $schemeKey -Force | Out-Null

    foreach ($styleKey in $styles.Keys) {
        $currentStyle = $styles[$styleKey]
        $map = [ordered]@{}
        $map['Arrow'] = [string]$currentStyle['Arrow']
        $map['Help'] = [string]$currentStyle['Help']
        $map['AppStarting'] = [string]$currentStyle['AppStarting']

        foreach ($commonKey in $common.Keys) {
            $map[$commonKey] = [string]$common[$commonKey]
        }

        # Important: a foreach statement is NOT embedded in parentheses here.
        # This is valid in Windows PowerShell 5.1.
        $schemeValues = @()
        foreach ($role in $orderedRoles) {
            $schemeValues += [string]$map[$role]
        }
        $schemeString = $schemeValues -join ','

        New-ItemProperty -Path $schemeKey -Name ([string]$currentStyle['Name']) -Value $schemeString -PropertyType String -Force | Out-Null
    }

    if ($InstallOnly) {
        Write-Host '[7/7] Installation complete. Nothing was applied because -InstallOnly was used.' -ForegroundColor Green
        Write-Host '      Open Control Panel > Mouse > Pointers and choose one of the Modern Inverted schemes.'
        Wait-BeforeExit
        return
    }

    if ([string]::IsNullOrWhiteSpace($Style)) {
        Write-Host ''
        Write-Host 'Choose the cursor style to apply now:' -ForegroundColor Cyan
        Write-Host '  [1] Regular'
        Write-Host '  [2] No Tail'
        Write-Host '  [3] Tail Detached'
        Write-Host '  [4] Compact  <-- recommended first try'
        Write-Host ''
        $choice = Read-Host 'Enter 1, 2, 3, or 4 [default = 4]'

        switch ($choice) {
            '1' { $Style = 'Regular' }
            '2' { $Style = 'NoTail' }
            '3' { $Style = 'TailDetached' }
            default { $Style = 'Compact' }
        }
    }

    Write-Host "[7/7] Applying style: $Style ..."
    $selected = $styles[$Style]
    if ($null -eq $selected) {
        throw "Unknown style: $Style"
    }

    $apply = [ordered]@{}
    $apply['Arrow'] = [string]$selected['Arrow']
    $apply['Help'] = [string]$selected['Help']
    $apply['AppStarting'] = [string]$selected['AppStarting']
    foreach ($commonKey in $common.Keys) {
        $apply[$commonKey] = [string]$common[$commonKey]
    }

    $cursorKey = 'HKCU:\Control Panel\Cursors'
    New-Item -Path $cursorKey -Force | Out-Null

    foreach ($role in $orderedRoles) {
        Set-RegistryNamedValue -Path $cursorKey -Name $role -Value ([string]$apply[$role])
    }

    Set-RegistryDefaultValue -SubKey 'Control Panel\Cursors' -Value ([string]$selected['Name'])
    Set-RegistryNamedValue -Path $cursorKey -Name 'Scheme Source' -Value '1'
    Reload-Cursors

    Write-Host ''
    Write-Host 'SUCCESS.' -ForegroundColor Green
    Write-Host "Applied: $($selected['Name'])" -ForegroundColor Green
    Write-Host "Cursor files: $CursorDir"
    Write-Host "Backup: $BackupFile"
    Write-Host "Log: $LogPath"
    Write-Host ''
    Write-Host 'To restore your previous cursor setup later, run:' -ForegroundColor Cyan
    Write-Host "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Restore"
    Write-Host ''
    Write-Host 'Tip: while judging sharpness, keep Settings > Accessibility > Mouse pointer size near its default.' -ForegroundColor DarkGray

    Wait-BeforeExit
}
catch {
    Write-Host ''
    Write-Host 'INSTALLER ERROR' -ForegroundColor Red
    Write-Host '---------------' -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    Write-Host ''

    if ($null -ne $_.InvocationInfo) {
        if ($_.InvocationInfo.PositionMessage) {
            Write-Host $_.InvocationInfo.PositionMessage -ForegroundColor Yellow
        }
    }

    Write-Host "Log file: $LogPath" -ForegroundColor Yellow
    Write-Host ''
    Write-Host 'The window will NOT auto-close. Copy the red/yellow error text back to ChatGPT if it still fails.' -ForegroundColor Yellow

    Wait-BeforeExit
}
finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
    }

    if ($TranscriptStarted) {
        try { Stop-Transcript | Out-Null } catch { }
    }
}