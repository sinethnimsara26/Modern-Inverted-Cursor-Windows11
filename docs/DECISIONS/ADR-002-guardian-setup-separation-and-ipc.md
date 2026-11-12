# ADR-002: Separating Setup & Guardian + Verified IPC Handshake & Targeted Shutdown

- **Status:** Accepted (Batch 01)
- **Date:** 2026-11-12

## Context
In `v1.0.2`:
- `setup.rs` launched Guardian (`Command::new().spawn()`), polled `FindWindowW(TRAY_WND_CLASS)` blindly for 2 seconds, and returned `Ok` even if Guardian crashed immediately or never applied cursors.
- `stop_any_running_guardian()` called `taskkill.exe /IM ModernInvertedCursorGuardian.exe /F` indiscriminately across all processes matching that image name.
- `guardian.rs` contained uninstall confirmation + file deletion (`--uninstall`), and `setup.rs` ran raw registry cleanup (`--restore`), mixing responsibilities.

## Decision
1. **Scoped Per-Session Control & Readiness Protocol (`src/lib.rs` / IPC module):**
   - Pass explicit `--ready-event <named-or-inheritable-event>` (or deterministic per-install session readiness event + shared status / exit-code check via `WaitForMultipleObjects` on `[process_handle, ready_event]`!) when Setup starts Guardian:
     - **Immediate crash detection:** If `process_handle` signals BEFORE `ready_event`, Setup reads the child's exit code & `guardian.log` tail and fails immediately instead of waiting out a timeout or returning false `Ok`!
     - **Readiness verification:** Guardian signals `ready_event` ONLY after: (a) acquiring single-instance mutex, (b) registering tray window & creating tray icon, (c) completing initial `register_and_apply()` cleanly, AND (d) writing a verified status/version tag (`HKCU\Software\ModernInvertedCursors\RuntimeStatus` or readiness payload) matching the expected `APP_VERSION` and PID!
     - **Timeout failure & rollback:** If timeout (`10s`) expires without readiness, terminate that exact child `HANDLE`, roll back newly changed `Run`/cursor state cleanly, and present an actionable error.
2. **Targeted Shutdown Replacing `taskkill /IM`:**
   - Primary: Find tray HWND, verify its owning PID (`GetWindowThreadProcessId`), verify its image path (`QueryFullProcessImageNameW` / Win7-compatible `GetModuleFileNameExW` or `QueryFullProcessImageNameW` in Kernel32 Vista+) belongs to our install root or known legacy guardian name, post `WM_CLOSE`, and `WaitForSingleObject(proc_handle, 3000)`.
   - Secondary (if window-less hang or legacy mutex exists): Enumerate processes safely via `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` + verify exact executable path matches `%LOCALAPPDATA%\ModernInvertedCursors\ModernInverted*.exe` (never another path/directory!) before calling `TerminateProcess(h_proc, 1)` ONLY on that exact verified PID and waiting for process exit (`WAIT_OBJECT_0`).

## Consequences
- Zero chance of killing unrelated binaries elsewhere on the machine.
- Installer never lies about Guardian health.
- Clean zero-dependency implementation using built-in Vista/Win7+ `kernel32.dll` & `user32.dll`.
