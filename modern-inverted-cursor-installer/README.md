# Modern Inverted Cursor Installer for Windows 11

This folder preserves the complete troubleshooting project that produced a Windows-11-integrated installer for **Modern Inverted Mouse Cursors v3.6.2** while keeping true dynamic inversion.

## Current final version: V5.1 Balanced Guardian

Final source:

`modern-inverted-cursor-installer/final/modern_inverted_installer_v5_1_balanced.rs`

Final Windows executable (built by GitHub Actions):

`modern-inverted-cursor-installer/dist/ModernInvertedInstallerV5_1.exe`

V5.1 was tested successfully on Windows 11 on 2026-10-03.

### Why V5.1 exists

V4 solved the immediate application problem with a force-live `SetSystemCursor` path and stopped reopening Mouse Properties, but Windows Accessibility could still replace the custom cursor whenever the pointer-size slider changed.

V5 added a silent event-driven per-user guardian that watches Accessibility cursor-setting changes and reapplies the pack automatically.

V5.1 keeps that integration and normalizes the 15-position Windows size slider to a balanced 32..88 px range:

```text
1  -> 32 px
2  -> 36 px
3  -> 40 px
4  -> 44 px
5  -> 48 px
6  -> 52 px
7  -> 56 px
8  -> 60 px
9  -> 64 px
10 -> 68 px
11 -> 72 px
12 -> 76 px
13 -> 80 px
14 -> 84 px
15 -> 88 px
```

This removes the large first jump and avoids scaling low-bit-depth XOR/inverting cursors to absurd sizes where pixelation becomes severe.

## What the final installer does

- Downloads the exact public v3.6.2 cursor assets from upstream.
- Verifies every asset by expected size and Git blob SHA-1.
- Backs up the current user's cursor registry settings.
- Registers Regular, No Tail, Tail Detached, and Compact schemes.
- Uses `SystemParametersInfoW(SPI_SETCURSORS)` and `SetSystemCursor` for live application.
- Uses `LoadImageW` at the balanced render size for static and animated cursor roles.
- Installs a silent guardian at `%LOCALAPPDATA%\ModernInvertedCursors\ModernInvertedCursorGuardian.exe`.
- Starts the guardian at sign-in through the current-user Run key.
- Watches Accessibility changes with `RegNotifyChangeKeyValue` rather than polling.
- Reapplies the modern inverted pack after the Windows pointer-size slider changes.
- Supports `--restore`, which removes the guardian and restores the original cursor configuration.

## Local build

```powershell
rustup toolchain install stable-gnu
rustc +stable-gnu -O .\final\modern_inverted_installer_v5_1_balanced.rs -o .\dist\ModernInvertedInstallerV5_1.exe
```

Or:

```powershell
.\build.ps1
```

## Run

```powershell
.\ModernInvertedInstallerV5_1.exe --preset compact
```

Compact is the tested preset.

Restore:

```powershell
.\ModernInvertedInstallerV5_1.exe --restore
```

Original backup:

```text
%LOCALAPPDATA%\ModernInvertedCursors\backup-original.reg
```

## History

See [docs/DEBUGGING_HISTORY.md](docs/DEBUGGING_HISTORY.md). Earlier PowerShell/Rust attempts are preserved under `archive/`, including V4 and the first V5 guardian.

## Upstream attribution

Cursor artwork/assets are from **emvaized/modern_inverted_mouse_cursors v3.6.2**:

https://github.com/emvaized/modern_inverted_mouse_cursors

This project does not contain or redistribute the author's Patreon package. The installer downloads the public source assets from the pinned tag and verifies them before use.

Development/testing date: **2026-10-03**
