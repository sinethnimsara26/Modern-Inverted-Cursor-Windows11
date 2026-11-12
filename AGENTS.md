# AGENTS.md — Engineering Rules, Architecture Constraints & Verification Protocol

## 1. Mission & Non-Negotiable Constraints
1. **Zero external crates (`[dependencies]` must remain empty)** unless a security defect cannot be mitigated responsibly in pure Rust + Win32 FFI and is formally approved via a new ADR in `docs/DECISIONS/`.
2. **Preserve the user's live Windows desktop & cursor state:** Never run destructive registry/cursor tests against live `HKCU\Control Panel\Cursors`, live `HKCU\Software\Microsoft\Accessibility`, live `HKCU\...\Run`, or `%LOCALAPPDATA%\ModernInvertedCursors\backup-original.reg`. Use isolated temporary directories, isolated temporary subkeys (e.g., sandbox test prefixes), or dedicated CI runners.
3. **Never touch `legacy-v5.1` history or delete published releases (`v1.0.0`, `v1.0.1`, `v1.0.2`).**
4. **Never merge to `main` or publish a non-RC stable release** without explicit user approval. Work only on the active hardening branch (`hardening/v1.1.0-audit`) and produce release candidates (`v1.1.0-rc.1`, etc.).
5. **Never fake measurements, screenshots, CI runs, or OS runtime claims.** Distinguish compile/PE/sandbox checks on Windows 11 runners from true Windows 7 / Windows 8 runtime execution if Windows 7/8 runners are absent.
6. **Never silently disable Defender, SmartScreen, UAC, or elevate to Administrator** unless technically unavoidable and explicitly documented.

## 2. Architecture Boundaries
- **`src/lib.rs` (or modular `src/core/*`):** Pure shared core only — cursor pack manifest/SHA-1 verification, WinHTTP/curl networking helpers, Win32 registry helpers, backup export/validation/restore, balanced DPI/cursor application, IPC protocol, SDF/vector icon renderer. **No setup GUI code, no tray popup menu code.**
- **Application A — `ModernInvertedSetup` (`src/bin/setup.rs` / UI modules):**
  - Responsible for Dark-Mode GUI wizard & installed maintenance UX (preset selection, repair, modify, upgrade, restore/uninstall), downloading & SHA-1 verifying all 32 cursor assets, backing up and validating `backup-original.reg`, migrating legacy `v1.0.0`–`v1.0.2` installs, deploying both itself (maintenance copy) and Guardian into `%LOCALAPPDATA%\ModernInvertedCursors`, registering Windows Installed Apps (`ModifyPath`, `UninstallString`, `QuietUninstallString`), launching Guardian, and verifying bounded startup readiness + initial cursor apply.
  - **Must fail closed:** `build.rs` must panic on release builds if the target Guardian PE payload is missing or invalid (`< 10 KB` or non-PE). Silent mode (`--silent`) must never display blocking `MessageBoxW` dialogs and must always propagate errors with non-zero exit codes and written logs.
- **Application B — `ModernInvertedCursorGuardian` (`src/bin/guardian.rs`):**
  - Lightweight `#![windows_subsystem = "windows"]` tray daemon with minimal idle memory and event-driven wait (`MsgWaitForMultipleObjectsEx` + `RegNotifyChangeKeyValue`).
  - Owns tray icon lifecycle, context menu, pause/resume, live preset switching, Explorer `TaskbarCreated` recovery, unlock/display/power/theme recovery, single-instance lock, and targeted IPC shutdown / readiness acknowledgment.
  - **Must not** contain setup wizard UI, download orchestration screens, or indiscriminate `taskkill /IM` calls.

## 3. Coding, Safety & Static Analysis Standards
- Every `unsafe` block or `unsafe fn` requires a clear `// SAFETY:` comment explaining pointer lifetimes, handle ownership, ABI alignment, and Win32 reentrancy invariants.
- Check Win32 return codes explicitly; avoid `let _ = ...` on state-mutating APIs (`SetSystemCursor`, `RegSetValueExW`, `WaitForSingleObject`, `CreateEventW`, `DeleteFileW`) without deliberate documented handling.
- All code must pass before any batch commit claiming green verification:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings` (or documented toolchain exceptions for Rust `1.75.0`)
  - `cargo test` (plus `cargo +1.75.0 test --target x86_64-pc-windows-msvc` and `i686-pc-windows-msvc` on Windows release builds)
  - PE header + `dumpbin /IMPORTS` audit on legacy `win7-win10-*` builds
  - Automated GUI interaction + multi-DPI (`100%`, `125%`, `150%`, `200%`) screenshot verification

## 4. Mandatory Persistent Documentation Rules
Every completed batch MUST update:
- `AI_HANDOFF.md` (exact commit SHA, batch status, test outcomes, next batch actions)
- `docs/ENGINEERING_JOURNAL.md` (problem, evidence, root cause, alternatives, rationale, files changed, test outcomes)
- Relevant audit/test/performance/security docs (`docs/BUG_AUDIT.md`, `docs/TEST_EVIDENCE.md`, `docs/PERFORMANCE_REPORT.md`, `docs/SECURITY_AUDIT.md`, `docs/KNOWN_ISSUES.md`, `docs/RELEASE_CHECKLIST.md`)
