# Modern Inverted Mouse Cursors v3.6.2 - community installer (FIXED PowerShell 5.1 syntax)

# Source project: https://github.com/emvaized/modern_inverted_mouse_cursors

# Cursor artwork/code: Copyright (c) 2024 emvaized (Max Tsyba), MIT License

# This installer script is an unofficial helper generated from the public source repository.

# It does NOT contain or redistribute the author's paid Patreon archive.



[CmdletBinding()]

param(

    [ValidateSet('Regular','NoTail','TailDetached','Compact')]

    [string]$Style,

    [switch]$InstallOnly

)



$ErrorActionPreference = 'Stop'

$Version = '3.6.2'

$Repo = 'emvaized/modern_inverted_mouse_cursors'

$ArchiveUrl = "https://github.com/$Repo/archive/refs/tags/$Version.zip"

$InstallRoot = Join-Path $env:LOCALAPPDATA "ModernInvertedCursors\v$Version"

$CursorDir = Join-Path $InstallRoot 'cursors'

$BackupFile = Join-Path $InstallRoot 'previous-cursor-backup.json'

$TempRoot = Join-Path $env:TEMP ("ModernInvertedCursors-" + [guid]::NewGuid().ToString('N'))

$ZipPath = Join-Path $TempRoot 'source.zip'



# Git object IDs and exact byte sizes reported by GitHub for tag 3.6.2.

# A Git blob SHA-1 is SHA1("blob <length>\0" + file bytes); this lets us verify

# that the downloaded files are exactly the public source assets from the tag.

$Expected = @{

 'arrow.cur' = @('2246','325739ebbc8a589aa757b7a7f19194bd0e6afd3a')

 'arrow_no_tail.cur' = @('2384','8a503406723a840324b966c46074ca7257b8b145')

 'arrow_no_tail_smaller.cur' = @('2246','daf30b0336f061a1a7961e1dd0dea9d1ce7ccd40')

 'arrow_no_tail_smaller_v2.cur' = @('2246','5e7990e298d85a4d8eb205e0b64a2b7d06b3f2b1')

 'arrow_tail_detached.cur' = @('2246','aa8082e301909e7bbf0bee1fdeb47db961f6d34a')

 'beam.cur' = @('2246','87c55d6e1a8d819ac80e4edd1d8afe4e35846f5a')

 'beam_v2.cur' = @('2246','b092bcf3b77e8cd9413c52e71d9a0a64afabaa69')

 'busy.ani' = @('5474','fa36f833b346a3d6310458d0a6bc30ec1c8657ad')

 'cross.cur' = @('2246','247947ea4293cb80fe60fe7143ae5c8d9633b7ad')

 'diagonal_1.cur' = @('2246','fe4205fd226707e165e1c9bc6ebf040419df33d2')

 'diagonal_2.cur' = @('2246','2244e646b88022bd3119a16ae7d17ee88aca88a9')

 'help.cur' = @('2246','d807f02924caa159d3bebd138af570bd6dd6ae3e')

 'help_no_tail.cur' = @('2246','fc3985098751b3d6e38f32ae43a9e736cafbdad1')

 'help_tail_detached.cur' = @('326','b3a1e4dcba4e4b2f24ade81dda0eece44e224d4d')

 'horizontal.cur' = @('2246','a38b02b149045421513ca7954d07581a6ae61745')

 'horizontal_v2.cur' = @('2246','8c7888da14a7a13441c56db830877df91024fd7d')

 'link.cur' = @('2246','86dae854fc5479674cca5ea25986b87c0ac2f3e0')

 'link_v2.cur' = @('2246','01eab41a23bbdaaf9f84e449f846a4f233777b68')

 'move.cur' = @('2246','6fdb0c4e3812c1cb896e5c45d4f691de53d80e84')

 'move_v2.cur' = @('326','6df1bcd2bea99c47a025d9f4dd03db9a3a6b3a98')

 'pen.cur' = @('2246','e94d2d6a7182abad2cda2b75434ed37ea7d9ebbc')

 'special.cur' = @('2246','dbb6b1fac453fb52ae1cfc87d09ee907373b19d3')

 'unavailable.cur' = @('8774','70496b18014075e2598f61c87ebd2c790174eb1c')

 'unavailable_v2.cur' = @('2246','4baceb98c400b4b6836a35d4e77b3fa7e76ad68d')

 'unavailable_v3.cur' = @('2246','d3e0901c59ec8494d8fc0a12caca418bac8b65ad')

 'vertical.cur' = @('2246','f6b8c7d085550f01b23a911d617b817bbac87d7d')

 'vertical_v2.cur' = @('2246','57b4a71f8fb6ffc70a169c65d3a561887d193aac')

 'working.ani' = @('5474','fcced8db8402d254a3f2e1e635bdb50dd334016c')

 'working_no_tail.ani' = @('5474','0c368f9a121a14c7c581ccfb60c427bf5093f77f')

 'working_tail_detached.ani' = @('5474','dec0cc444841031357b0b1a1b7189bd61340018a')

 'working_tail_detached_v2.ani' = @('5474','9f3656efe73c669aead22627e1f0bc854bab8b1a')

 'working_tail_detached_v3.ani' = @('2072','c47132bee21ac2941c6db45a72590f1333876907')

}



function Get-GitBlobSha1([string]$Path) {

    $bytes = [IO.File]::ReadAllBytes($Path)

    $header = [Text.Encoding]::ASCII.GetBytes("blob $($bytes.Length)`0")

    $all = New-Object byte[] ($header.Length + $bytes.Length)

    [Array]::Copy($header,0,$all,0,$header.Length)

    [Array]::Copy($bytes,0,$all,$header.Length,$bytes.Length)

    $sha1 = [Security.Cryptography.SHA1]::Create()

    try { return (($sha1.ComputeHash($all) | ForEach-Object { $_.ToString('x2') }) -join '') }

    finally { $sha1.Dispose() }

}



function Reload-Cursors {

    if (-not ('CursorNative' -as [type])) {

        Add-Type @"

using System;

using System.Runtime.InteropServices;

public static class CursorNative {

    [DllImport("user32.dll", SetLastError=true)]

    public static extern bool SystemParametersInfo(uint uiAction, uint uiParam, IntPtr pvParam, uint fWinIni);

}

"@

    }

    # SPI_SETCURSORS = 0x57; SPIF_UPDATEINIFILE | SPIF_SENDCHANGE = 0x03

    [void][CursorNative]::SystemParametersInfo(0x57, 0, [IntPtr]::Zero, 0x03)

}



function Set-RegistryDefaultValue([string]$SubKey,[string]$Value) {

    $k=[Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($SubKey)

    try { $k.SetValue('', $Value, [Microsoft.Win32.RegistryValueKind]::String) }

    finally { $k.Close() }

}



function Get-RegistryDefaultValue([string]$SubKey) {

    $k=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($SubKey)

    if ($null -eq $k) { return '' }

    try { return [string]$k.GetValue('', '') }

    finally { $k.Close() }

}



function Cursor([string]$Name) { Join-Path $CursorDir $Name }



try {

    Write-Host "Modern Inverted Mouse Cursors v$Version" -ForegroundColor Cyan

    Write-Host "Unofficial installer built from the project's public MIT-licensed source tag." -ForegroundColor DarkGray

    Write-Host



    New-Item -ItemType Directory -Force -Path $TempRoot,$InstallRoot,$CursorDir | Out-Null



    if (-not (Test-Path $BackupFile)) {

        $cursorKey='HKCU:\Control Panel\Cursors'

        $roles=@('Arrow','Help','AppStarting','Wait','Crosshair','IBeam','NWPen','No','SizeNS','SizeWE','SizeNWSE','SizeNESW','SizeAll','UpArrow','Hand','Pin','Person')

        $p=Get-ItemProperty -Path $cursorKey

        $values=[ordered]@{}

        foreach($r in $roles) { $values[$r]=[string]$p.$r }

        $backup=[ordered]@{

            DefaultScheme=Get-RegistryDefaultValue 'Control Panel\Cursors'

            SchemeSource=[string]$p.'Scheme Source'

            Roles=$values

        }

        $backup | ConvertTo-Json -Depth 4 | Set-Content -Path $BackupFile -Encoding UTF8

        Write-Host "Backed up your current cursor configuration." -ForegroundColor DarkGray

    }



    Write-Host "Downloading the official public source tag $Version from GitHub..."

    Invoke-WebRequest -Uri $ArchiveUrl -OutFile $ZipPath

    Expand-Archive -Path $ZipPath -DestinationPath $TempRoot -Force

    $Src = Join-Path $TempRoot "modern_inverted_mouse_cursors-$Version\src"

    if (-not (Test-Path $Src)) { throw "Expected source directory was not found after extraction." }



    Write-Host "Verifying all 32 cursor files against GitHub's v$Version blob IDs..."

    foreach($name in ($Expected.Keys | Sort-Object)) {

        $f=Join-Path $Src $name

        if (-not (Test-Path $f)) { throw "Missing expected file: $name" }

        $len=(Get-Item $f).Length

        if ($len -ne [int64]$Expected[$name][0]) { throw "Size check failed for $name ($len bytes)." }

        $actual=Get-GitBlobSha1 $f

        if ($actual -ne $Expected[$name][1]) { throw "Git blob SHA-1 check failed for $name." }

        Copy-Item $f (Join-Path $CursorDir $name) -Force

    }

    Write-Host "Integrity checks passed." -ForegroundColor Green



    # Preserve attribution/license alongside installed assets.

    $licenseSrc=Join-Path $TempRoot "modern_inverted_mouse_cursors-$Version\LICENSE"

    if (Test-Path $licenseSrc) { Copy-Item $licenseSrc (Join-Path $InstallRoot 'LICENSE.txt') -Force }



    $sys=Join-Path $env:SystemRoot 'Cursors'

    $pin=Join-Path $sys 'pin_i.cur'

    $person=Join-Path $sys 'person_i.cur'

    if (-not (Test-Path $pin)) { $pin=Join-Path $sys 'pin.cur' }

    if (-not (Test-Path $person)) { $person=Join-Path $sys 'person.cur' }



    $common=[ordered]@{

        Wait=(Cursor 'busy.ani')

        Crosshair=(Cursor 'cross.cur')

        IBeam=(Cursor 'beam.cur')

        NWPen=(Cursor 'pen.cur')

        No=(Cursor 'unavailable.cur')

        SizeNS=(Cursor 'vertical.cur')

        SizeWE=(Cursor 'horizontal.cur')

        SizeNWSE=(Cursor 'diagonal_1.cur')

        SizeNESW=(Cursor 'diagonal_2.cur')

        SizeAll=(Cursor 'move.cur')

        UpArrow=(Cursor 'special.cur')

        Hand=(Cursor 'link.cur')

        Pin=$pin

        Person=$person

    }



    $styles=[ordered]@{

        Regular=[ordered]@{ Name="Modern Inverted v$Version - Regular"; Arrow=(Cursor 'arrow.cur'); Help=(Cursor 'help.cur'); AppStarting=(Cursor 'working.ani') }

        NoTail=[ordered]@{ Name="Modern Inverted v$Version - No Tail"; Arrow=(Cursor 'arrow_no_tail.cur'); Help=(Cursor 'help_no_tail.cur'); AppStarting=(Cursor 'working_no_tail.ani') }

        TailDetached=[ordered]@{ Name="Modern Inverted v$Version - Tail Detached"; Arrow=(Cursor 'arrow_tail_detached.cur'); Help=(Cursor 'help_tail_detached.cur'); AppStarting=(Cursor 'working_tail_detached_v3.ani') }

        Compact=[ordered]@{ Name="Modern Inverted v$Version - Compact"; Arrow=(Cursor 'arrow_no_tail_smaller_v2.cur'); Help=(Cursor 'help_no_tail.cur'); AppStarting=(Cursor 'working_tail_detached_v3.ani') }

    }



    $schemeKey='HKCU:\Control Panel\Cursors\Schemes'

    New-Item -Path $schemeKey -Force | Out-Null

    $orderedRoles=@('Arrow','Help','AppStarting','Wait','Crosshair','IBeam','NWPen','No','SizeNS','SizeWE','SizeNWSE','SizeNESW','SizeAll','UpArrow','Hand','Pin','Person')



    foreach($sKey in $styles.Keys) {

        $s=$styles[$sKey]

        $map=[ordered]@{}

        $map.Arrow=$s.Arrow; $map.Help=$s.Help; $map.AppStarting=$s.AppStarting

        foreach($k in $common.Keys) { $map[$k]=$common[$k] }

        $schemeValues = foreach ($r in $orderedRoles) { $map[$r] }
        $schemeString = $schemeValues -join ','

        New-ItemProperty -Path $schemeKey -Name $s.Name -Value $schemeString -PropertyType String -Force | Out-Null

    }



    Write-Host "Registered four schemes in classic Mouse Properties." -ForegroundColor Green



    if ($InstallOnly) {

        Write-Host "Install-only mode complete. Open Mouse Properties > Pointers to choose a scheme."

        exit 0

    }



    if (-not $Style) {

        Write-Host

        Write-Host 'Choose the look to apply:' -ForegroundColor Cyan

        Write-Host '  1  Regular'

        Write-Host '  2  No Tail'

        Write-Host '  3  Tail Detached (uses the new v3 working animation)'

        Write-Host '  4  Compact (small no-tail arrow + detached working animation)'

        $choice=Read-Host 'Enter 1-4 [default: 4]'

        $Style = switch($choice) { '1'{'Regular'} '2'{'NoTail'} '3'{'TailDetached'} default{'Compact'} }

    }



    $selected=$styles[$Style]

    $apply=[ordered]@{}

    $apply.Arrow=$selected.Arrow; $apply.Help=$selected.Help; $apply.AppStarting=$selected.AppStarting

    foreach($k in $common.Keys) { $apply[$k]=$common[$k] }



    $cursorKey='HKCU:\Control Panel\Cursors'

    foreach($r in $orderedRoles) { Set-ItemProperty -Path $cursorKey -Name $r -Value $apply[$r] -Force }

    Set-RegistryDefaultValue 'Control Panel\Cursors' $selected.Name

    Set-ItemProperty -Path $cursorKey -Name 'Scheme Source' -Value 1 -Force

    Reload-Cursors



    Write-Host

    Write-Host "Applied: $($selected.Name)" -ForegroundColor Green

    Write-Host "Files: $CursorDir"

    Write-Host "Backup: $BackupFile"

    Write-Host 'You can switch among the four registered schemes later in Control Panel > Mouse > Pointers.'

    Write-Host

    Write-Host 'If Windows Accessibility is enlarging the pointer, keep its Size slider near the default while judging sharpness.' -ForegroundColor DarkGray

}

finally {

    if (Test-Path $TempRoot) { Remove-Item $TempRoot -Recurse -Force -ErrorAction SilentlyContinue }

}
