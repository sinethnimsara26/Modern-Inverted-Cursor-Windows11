# ADR-001: Semantic Versioning (`v1.1.0-rc.1`), Artifact Naming & Upgrade Migration

- **Status:** Accepted (Batch 01)
- **Date:** 2026-11-12

## Context
Through `v1.0.2`, executable filenames mixed conventions (`ModernInvertedSetup-v1.0.2-x64.exe`, `ModernInvertedCursorGuardian-x64.exe`), `UninstallString` pointed to the Guardian executable rather than a persistent Setup/Maintenance executable, and no formal release-candidate separation existed before publishing on `main`.

## Decision
1. Advance the hardened release to **`v1.1.0`** prepared as release candidate **`v1.1.0-rc.1`** on branch `hardening/v1.1.0-audit` (per SemVer: minor version bump for architectural separation, new IPC protocol, new persistent maintenance executable, and standardized artifact names while preserving user migration).
2. Standardize release asset names cleanly:
   - `ModernInvertedCursors-Setup-vX.Y.Z-<arch>.exe`
   - `ModernInvertedCursors-Guardian-vX.Y.Z-<arch>.exe`
   - Across targets: `x64`, `arm64`, `win7-win10-x64`, `win7-win10-x86`.
3. Install both persistent components into `%LOCALAPPDATA%\ModernInvertedCursors\`:
   - `ModernInvertedCursors-Setup.exe` (installed maintenance/repair/modify/uninstall UI, pointed to by `HKCU\...\Uninstall\ModernInvertedCursors\{UninstallString,QuietUninstallString,ModifyPath}`)
   - `ModernInvertedCursors-Guardian.exe` (background system-tray guardian pointed to by `HKCU\...\Run`)
4. **Backward-Compatible Migration from `v1.0.0`–`v1.0.2` (and V5.1):**
   - Detect and preserve existing `backup-original.reg` cleanly (after validating it is non-empty valid `.reg` text) — **never overwrite** a pre-existing valid backup!
   - Gracefully shut down legacy `ModernInvertedCursorGuardian.exe` / V5.1 instances via targeted verification (matching install path / known tray class) before writing the upgraded binaries.
   - Update `HKCU\...\Run` (removing obsolete `ModernInvertedCursorGuardian` value cleanly if renamed or updating it safely), migrate `HKCU\Software\ModernInvertedCursors`, and write clean Installed Apps metadata (`NoModify=0`, `ModifyPath`, `UninstallString`, `QuietUninstallString`).
   - Optionally retain or archive-notice legacy files cleanly without breaking `backup-original.reg` or `v3.6.2\`.

## Consequences
- Users on `v1.0.0`–`v1.0.2` can upgrade in one click while keeping their original `backup-original.reg` and selected preset.
- Installed Apps now offers **Modify** (opens Maintenance UI) as well as **Uninstall**.
