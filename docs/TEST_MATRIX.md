# Test Matrix & Coverage Plan

Target platforms: Modern Windows `x64`, Native Windows `ARM64`, Legacy Windows 7 SP1–10 `x64`, Legacy Windows 7 SP1–10 `x86`.

| Category | Test / Verification Case | Target Scope | Environment | Current Baseline (`398f09b`) | Target Batch |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Unit** | Git blob SHA-1, URL parser, balanced size curve, preset parsing | `core` | All 4 targets | Pass (6 tests) | Expand in Batch 04/07 |
| **Unit / ABI** | Win32 `repr(C)` struct sizes (64-bit & 32-bit) | `setup`, `guardian` | All 4 targets | Pass (2 tests) | Expand for IPC/toolhelp |
| **Unit / Backup** | `.reg` backup header/key validation (UTF-16LE, UTF-8, corrupt, empty, missing) | `core` | All 4 targets | **MISSING** | Batch 02 |
| **Unit / Path Security** | Safe path validation & non-shell cleanup args | `core` | All 4 targets | **MISSING** | Batch 04 |
| **Integration** | Guardian startup readiness event + timeout + immediate crash exit detection + rollback | `setup` <-> `guardian` | Isolated sandbox + fake binaries | **MISSING** | Batch 02 / 07 |
| **Integration** | Transactional restore preserving `Run`/metadata when backup is corrupt/missing | `setup` / `guardian` | Isolated sandbox registry + temp dir | **MISSING** | Batch 02 / 07 |
| **Integration** | Targeted shutdown by PID + image path (must NOT kill unrelated process sharing name or wrong path) | `ipc` / process | Local + CI | **MISSING** | Batch 02 / 07 |
| **Integration** | `--silent` error propagation (nonzero exit code + log written + no blocking `MessageBoxW`) | `setup` | Local + CI | **MISSING** | Batch 02 / 07 |
| **Integration** | Upgrade migration (`v1.0.0`/`v1.0.1`/`v1.0.2` files & registry -> `v1.1.0` layout preserving `backup-original.reg`) | `setup` | Isolated sandbox | **MISSING** | Batch 03 / 07 |
| **Integration** | Corrupt/truncated/size-matching asset detection (`git_blob_sha1`) | `core` | Isolated sandbox | **MISSING** | Batch 04 / 07 |
| **GUI / UIA** | Real Setup window launch, mouse click / Tab / Arrow / Enter / Esc preset toggle + Install/Repair/Restore flows + visual screenshots at 100%, 125%, 150%, 200% DPI | `setup` | Local Windows 11 + CI | `--smoke-ui` static render only | Batch 07 |
| **Guardian Endurance** | Tray icon sync, `TaskbarCreated` Explorer restart recovery, double-click/hotkey reapply, idle handle/GDI/memory leak stability | `guardian` | Isolated local session | Manual partial only | Batch 05 / 07 |
| **Build & PE Audit** | Release fail-closed `build.rs`, `dumpbin /IMPORTS` zero UCRT/Win10 DLLs on `win7-win10-*`, `cargo fmt`, `cargo clippy` | All EXEs | Win11 + CI (`windows-latest`, `windows-11-arm`) | `cargo fmt` fails; `build.rs` fails open | Batch 04 / 08 |
| **Legacy Runtime** | True execution on real Windows 7 SP1 / Windows 8.1 VM | `win7-win10-*` | **Unavailable on GH/local** | Build + PE + Win11 compat only | Recorded honestly in `KNOWN_ISSUES.md` |
