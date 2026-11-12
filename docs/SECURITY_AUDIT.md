# Security Audit (`v1.0.2` Baseline `398f09b`)

## Scope & Threat Model
Per-user Windows cursor installer (`asInvoker`, no elevation requested) and background system-tray utility fetching pinned binary assets from `raw.githubusercontent.com/emvaized/modern_inverted_mouse_cursors/3.6.2/src/...` over HTTPS.

---

## Findings Matrix

| ID | Risk | Area | Finding (`398f09b`) | Remediation Plan |
| :--- | :--- | :--- | :--- | :--- |
| **SEC-01** | **High** | Network / Response Bounds | **FIXED (Batch 04):** Enforced `MAX_ASSET_BYTES = 262,144` streaming limit, explicit `WinHttpSetTimeouts(10s/20s/15s/20s)`, and sibling `AtomicBool` worker cancellation |
| **SEC-02** | **Medium-High** | Integrity Bypass | **FIXED (Batch 04):** Removed `assets_quick_ok()` size-only bypass; `ensure_all_assets()` runs full Git blob SHA-1 verification (`test_verify_asset_file_rejects_same_size_tampered_bytes`) |
| **SEC-03** | **Medium-High** | Process Termination Scope | **FIXED (Batch 02):** Replaced `taskkill /IM` with `stop_targeted_guardians` verifying tray PID + Toolhelp32 + `QueryFullProcessImageNameW` path match |
| **SEC-04** | **Medium** | Shell Command Injection | **FIXED (Batch 04):** Passed deletion target strictly via `MIC_DEL_TARGET` env var with `/D /Q` and `is_safe_cleanup_target` guard (no shell string interpolation) |
| **SEC-05** | **Medium** | Backup Validation & Restore Integrity | **FIXED (Batch 02):** Added strict UTF-16LE/UTF-8 header + `HKCU\Control Panel\Cursors` section-only allowlist validator before invoking `reg.exe import` |
| **SEC-06** | **Medium** | DLL Hijacking / Search Order | **FIXED (Batch 04):** Implemented `load_system_dll` (`GetModuleHandleW` -> `LOAD_LIBRARY_SEARCH_SYSTEM32` -> explicit `GetSystemDirectoryW` path fallback) across all dynamic DLL loads |
| **SEC-07** | **Low (Accepted Limitation)** | Code Signing & SmartScreen | Release EXEs are unsigned FOSS builds (`no Authenticode certificate` available); Windows Defender SmartScreen displays *"Windows protected your PC"* on fresh downloads and Smart App Control may block unsigned binaries | Batch 08: document explicitly in README, release notes & `KNOWN_ISSUES.md`; publish `SHA256SUMS.txt` |
| **SEC-08** | **Informational** | Legacy OS TLS & EOL | Windows 7 (`2020-01-14` / ESU `2023-01-10`) and Windows 8.1 (`2023-01-10`) are End of Life; Win7 WinHTTP requires KB3140245 for TLS 1.2 against `raw.githubusercontent.com` | Batch 04 / 08: document OS lifecycle & KB3140245 caveat transparently |
