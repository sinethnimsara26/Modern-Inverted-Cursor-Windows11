# Modern Inverted Cursor Installer for Windows 11

This folder preserves the complete troubleshooting project that produced a working installer for the **Modern Inverted Mouse Cursors v3.6.2** project.

The goal was to replace Windows' old-looking inverted cursor set with a more modern design while keeping **true dynamic inversion**: the cursor pixels invert whatever is underneath them rather than simply switching between black and white.

## Final product

The final source is:

`modern-inverted-cursor-installer/final/modern_inverted_installer_v3_force_live.rs`

The reproducibly built Windows executable is:

`modern-inverted-cursor-installer/dist/ModernInvertedInstallerV3.exe`

A GitHub Actions workflow rebuilds that EXE from the final Rust source and commits the binary back into this repository, so the working final product is preserved alongside the source.

## What the final installer does

- Downloads the exact public v3.6.2 cursor assets from:
  https://github.com/emvaized/modern_inverted_mouse_cursors
- Verifies every downloaded cursor using its expected size and Git blob SHA-1.
- Backs up the current user's cursor registry configuration.
- Registers four schemes: Regular, No Tail, Tail Detached, and Compact.
- Writes only to the current user's cursor settings under HKCU.
- Calls `SystemParametersInfoW(SPI_SETCURSORS)` to reload the scheme.
- Uses `LoadCursorFromFileW` + `SetSystemCursor` as a force-live fallback when Windows writes the scheme but does not visibly apply it.
- Preserves Windows' native inverted Pin and Person cursors where available.
- Supports restoring the original cursor configuration with `--restore`.

## Run

From PowerShell:

```powershell
.\ModernInvertedInstallerV3.exe
```

The Compact preset was the one tested successfully during development.

To restore the original cursor setup:

```powershell
.\ModernInvertedInstallerV3.exe --restore
```

The backup is stored under:

```text
%LOCALAPPDATA%\ModernInvertedCursors\backup-original.reg
```

## Local build

The development machine used Rust's GNU Windows toolchain because the default MSVC Rust target initially failed with `link.exe not found`.

```powershell
rustup toolchain install stable-gnu
rustc +stable-gnu -O .\final\modern_inverted_installer_v3_force_live.rs -o .\dist\ModernInvertedInstallerV3.exe
```

Or run:

```powershell
.\build.ps1
```

## Project history

See [docs/DEBUGGING_HISTORY.md](docs/DEBUGGING_HISTORY.md) for the full sequence of failures, fixes, commands, and the reason the final force-live Rust version exists.

Earlier source attempts are kept in `archive/` for reference.

## Upstream attribution

Cursor artwork/assets are from **emvaized/modern_inverted_mouse_cursors v3.6.2**. This repository does not contain or redistribute the author's Patreon package. The installer downloads the public repository assets at runtime and verifies them before use.

Upstream project:
https://github.com/emvaized/modern_inverted_mouse_cursors

Development date: **2026-10-03**
