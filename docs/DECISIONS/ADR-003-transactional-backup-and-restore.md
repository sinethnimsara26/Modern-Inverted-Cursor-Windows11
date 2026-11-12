# ADR-003: Failure-Safe Transactional Backup, Validation & Restore Ordering

- **Status:** Accepted (Batch 01)
- **Date:** 2026-11-12

## Context
In `v1.0.2`, EVERY restore/uninstall path (`setup.rs` GUI Restore, `setup.rs --restore`, `guardian.rs` menu Restore, `--uninstall`, `--restore`) deleted `HKCU\...\Run`, `HKCU\Software\ModernInvertedCursors`, `Uninstall`, and `ModernInvertedCursorGuardian.exe` **before** validating or importing `backup-original.reg`. Moreover, `export_backup()` treated any existing file at `backup-original.reg` as valid even if 0-byte or corrupt.

## Decision
1. **Strict Pre-Flight Backup Validation:**
   - Implement `validate_backup_file(path: &Path) -> Result<BackupKind, String>` verifying: file exists, non-empty, bounded size (`<= 256 KB`), valid UTF-16LE BOM / UTF-8 `.reg` header (`Windows Registry Editor Version 5.00` or `REGEDIT4`), and contains `[HKEY_CURRENT_USER\Control Panel\Cursors]` with valid cursor entries.
   - During `export_backup`: if `backup-original.reg` already exists and passes validation, **preserve it** untouched. If missing (or corrupt/0-byte AND current cursors are NOT already pointing at `ModernInvertedCursors`), write atomically via `.tmp` -> validate -> rename. Never overwrite a valid original backup during upgrades!
2. **Failure-Safe Restore / Uninstall Order:**
   - **Step 1:** Validate `backup-original.reg` BEFORE stopping Guardian or modifying any registry keys! If corrupt or missing, stop immediately with exact diagnostics AND offer safe explicit recovery (e.g. `--restore-system-defaults` using Windows' default `Control Panel\Cursors` reset / `SPI_SETCURSORS` without leaving the user stranded).
   - **Step 2:** Snapshot current state (so if `reg.exe import` fails unexpectedly, previous guardian startup/config isn't blindly wiped first).
   - **Step 3:** Pause/stop running Guardian cleanly.
   - **Step 4:** Execute `restore_backup` (capture exit code + stderr!), verify `HKCU\Control Panel\Cursors` no longer points into `%LOCALAPPDATA%\ModernInvertedCursors\v3.6.2` (or live reload succeeded).
   - **Step 5:** ONLY after backup restoration is confirmed, remove `HKCU\...\Run`, `CONFIG_SUBKEY`, `UNINSTALL_SUBKEY`, installed executables, and `backup-original.reg` (only on full uninstall if confirmed; or optionally keep `backup-original.reg` on `--restore`? Let's define: `--restore` keeps install folder or cleans startup while preserving assets; `--uninstall` cleans all installed artifacts after confirmed restoration!).

## Consequences
- A missing, locked, or corrupted `backup-original.reg` never leaves the system stripped of its tray controls or metadata silently.
