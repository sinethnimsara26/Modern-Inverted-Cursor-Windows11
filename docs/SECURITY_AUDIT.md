# Security Audit (`v1.0.2` Baseline `398f09b`)

## Scope & Threat Model
Per-user Windows cursor installer (`asInvoker`, no elevation requested) and background system-tray utility fetching pinned binary assets from `raw.githubusercontent.com/emvaized/modern_inverted_mouse_cursors/3.6.2/src/...` over HTTPS.

---

## Findings Matrix

| ID | Risk | Area | Finding (`398f09b`) | Remediation Plan |
| :--- | :--- | :--- | :--- | :--- |
| **SEC-01** | **High** | Network / Response Bounds | `winhttp_download_bytes` allocates unbounded `Vec<u8>` up to remote EOF before verifying asset size/SHA-1; no explicit `WinHttpSetTimeouts` (infinite DNS resolve default); no worker cancellation on hash mismatch | Batch 04: enforce `MAX_ASSET_BYTES` (`<= 256 KB` streaming limit), configure `WinHttpSetTimeouts`, cancel siblings on error |
| **SEC-02** | **Medium-High** | Integrity Bypass | `assets_quick_ok()` trusts byte-size match (`m.len() == a.size`) without verifying `git_blob_sha1()` when Guardian checks `ensure_all_assets(..., false)` | Batch 04: verify full `git_blob_sha1()` cleanly on every startup/repair (`~90 KB` total hashes in ~1–2 ms) |
| **SEC-03** | **Medium-High** | Process Termination Scope | `stop_any_running_guardian()` invokes `taskkill.exe /IM ModernInvertedCursorGuardian.exe /F` globally by filename only, risking unrelated process termination | Batch 02: replace with scoped IPC + PID + executable-path verification via `QueryFullProcessImageNameW` / Toolhelp32 |
| **SEC-04** | **Medium** | Shell Command Injection | `schedule_self_delete()` builds `cmd.exe /C "... \"<path>\""` string; special characters (`&`, `|`, `%`, `^`, `"`) in `%LOCALAPPDATA%` path could break quoting or execute unintended syntax | Batch 04: eliminate `cmd.exe /C` path string interpolation cleanly |
| **SEC-05** | **Medium** | Backup Validation & Restore Integrity | `restore_backup()` passes unvalidated `backup-original.reg` straight to `reg.exe import` after wiping uninstall/startup entries; untrusted/corrupt `.reg` could modify unintended keys or leave system stranded | Batch 02: strictly validate `.reg` structure & `Control Panel\Cursors` scope before deleting anything |
| **SEC-06** | **Medium** | DLL Hijacking / Search Order | `LoadLibraryW("winhttp.dll")`, `"dwmapi.dll"`, `"shcore.dll"`, `"wtsapi32.dll"`, `"user32.dll"` pass bare DLL filenames without `LOAD_LIBRARY_SEARCH_SYSTEM32` (`0x00000800`) | Batch 04: prefer `GetModuleHandleW` / System32-pinned loading (with Vista/Win7 fallback if KB2533623 absent) |
| **SEC-07** | **Low (Accepted Limitation)** | Code Signing & SmartScreen | Release EXEs are unsigned FOSS builds (`no Authenticode certificate` available); Windows Defender SmartScreen displays *"Windows protected your PC"* on fresh downloads and Smart App Control may block unsigned binaries | Batch 08: document explicitly in README, release notes & `KNOWN_ISSUES.md`; publish `SHA256SUMS.txt` |
| **SEC-08** | **Informational** | Legacy OS TLS & EOL | Windows 7 (`2020-01-14` / ESU `2023-01-10`) and Windows 8.1 (`2023-01-10`) are End of Life; Win7 WinHTTP requires KB3140245 for TLS 1.2 against `raw.githubusercontent.com` | Batch 04 / 08: document OS lifecycle & KB3140245 caveat transparently |
