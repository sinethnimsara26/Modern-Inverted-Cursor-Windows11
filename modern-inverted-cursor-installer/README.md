# Modern Inverted Cursor Installer for Windows 11

This folder preserves the complete troubleshooting project that produced a working installer for the **Modern Inverted Mouse Cursors v3.6.2** project.

The goal was to replace Windows' old-looking inverted cursor set with a more modern design while keeping **true dynamic inversion**: the cursor pixels invert whatever is underneath them rather than simply switching between black and white.

## Final tested product

The current final source is:

`modern-inverted-cursor-installer/final/modern_inverted_installer_v4_no_reapply.rs`

The final Windows executable built by CI is:

`modern-inverted-cursor-installer/dist/ModernInvertedInstallerV4.exe`

**V4 is the version confirmed working on the target Windows 11 machine on 2026-10-03.** The Compact preset visibly changed the active mouse pointer immediately.

## Why V4 exists

V3 introduced the essential `LoadCursorFromFileW` + `SetSystemCursor` force-live path, and that direct application worked. However, V3 then automatically opened classic **Mouse Properties -> Pointers**. Re-selecting the custom scheme there and pressing Apply/OK caused Windows to reload the normal cursor scheme path and could undo the live `SetSystemCursor` state, which was especially noticeable with Busy/Wait animations.

V4 keeps the successful force-live behavior but **does not open Mouse Properties afterward**. It explicitly warns the user not to re-select/apply the scheme immediately after installation.

## What the final installer does

- Downloads the exact public v3.6.2 cursor assets from:
  https://github.com/emvaized/modern_inverted_mouse_cursors
- Verifies every downloaded cursor using its expected size and Git blob SHA-1.
- Backs up the current user's cursor registry configuration.
- Registers four schemes: Regular, No Tail, Tail Detached, and Compact.
- Writes only to the current user's cursor settings under HKCU.
- Calls `SystemParametersInfoW(SPI_SETCURSORS)` to persist/reload the scheme.
- Uses `LoadCursorFromFileW` + `SetSystemCursor` to directly force-apply the active Windows cursor handles.
- Preserves Windows' native inverted Pin and Person cursors where available.
- Supports restoring the original cursor configuration with `--restore`.
- Does **not** reopen Mouse Properties after applying, preventing the successful force-live state from being accidentally overwritten.

## Run

From PowerShell:

```powershell
.\ModernInvertedInstallerV4.exe --preset compact
```

The **Compact** preset is the one confirmed working during the final test.

**Important:** after V4 reports `SUCCESS`, do not immediately re-select/apply the scheme in classic Mouse Properties. V4 has already applied the live cursor handles directly.

To restore the original cursor setup:

```powershell
.\ModernInvertedInstallerV4.exe --restore
```

The backup is stored under:

```text
%LOCALAPPDATA%\ModernInvertedCursors\backup-original.reg
```

## Local build

The development machine used Rust's GNU Windows toolchain because the default MSVC Rust target initially failed with `link.exe not found`.

```powershell
rustup toolchain install stable-gnu
rustc +stable-gnu -O .\final\modern_inverted_installer_v4_no_reapply.rs -o .\dist\ModernInvertedInstallerV4.exe
```

Or run:

```powershell
.\build.ps1
```

## Project history

See [docs/DEBUGGING_HISTORY.md](docs/DEBUGGING_HISTORY.md) for the complete sequence of failures, fixes, commands, false starts, and the final V4 correction.

Earlier source attempts are kept in `archive/` for reference.

## Upstream attribution

Cursor artwork/assets are from **emvaized/modern_inverted_mouse_cursors v3.6.2**. This repository does not contain or redistribute the author's Patreon package. The installer downloads the public repository assets at runtime and verifies them before use.

Upstream project:
https://github.com/emvaized/modern_inverted_mouse_cursors

Development/final validation date: **2026-10-03**
