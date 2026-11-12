# Engineering Journal

Chronological technical log of the audit, design decisions, experiments, measurements, failures, and fixes on `hardening/v1.1.0-audit`.

---

## 2026-11-12 — Batch 01: Baseline Investigation, Audits, Online Research & Baseline Benchmarks

### 1. Repository & CI Baseline State
- **Baseline commit (`main`):** `398f09b0b2c89c633d336edbf097789aad0e8256` (tag `v1.0.2`).
- **Latest GitHub Release:** `v1.0.2` (`9` assets: `x64`, `arm64`, `win7-win10-x64`, `win7-win10-x86`, `SHA256SUMS.txt`).
- **CI History:**
  - Passing: `#43160826875` (`v1.0.0`), `#43162511198` (`v1.0.1`), `#43177846627` (`v1.0.2` across `arm64`, `win7-win10-x64`, `win7-win10-x86`, `x64`, and `publish-release`).
  - Prior CI Failures Analyzed:
    - `#43162190599` (`v1.0.1` initial): `publish-release` ran on `ubuntu-latest` without `actions/checkout`, causing `gh release create` to fail with `fatal: not a git repository: .git`. Fixed in `a025f6a` (`--repo "$GH_REPO"`).
    - `#43177594470` (`v1.0.2` initial): PowerShell `@argsList` splatted an array positionally instead of named parameters (`error: toolchain '-Suffix' is not installed`). Fixed in `398f09b` using hashtable `@params`.

### 2. Static Checks on Baseline (`398f09b`)
- `cargo test`: `9/9` unit tests passed (`6` lib + `1` guardian + `2` setup). *(Note: `docs/TESTS_AND_BENCHMARKS.md` Section 1 still erroneously said `8/8`.)*
- `cargo fmt --check`: Failed — formatting drift across `build.rs`, `src/bin/guardian.rs`, `src/bin/setup.rs`, `src/lib.rs`.
- `cargo clippy --all-targets`: Exited 0 with **21 warnings**:
  - `src/lib.rs`: 6 warnings (`manual_div_ceil` ×2, `chunks_exact_to_as_chunks`, `needless_range_loop`, `type_complexity`, `too_many_arguments`, plus `manual_contains` in test)
  - `src/bin/guardian.rs`: 2 warnings (`manual_c_str_literals` ×2)
  - `src/bin/setup.rs`: 13 warnings (`manual_c_str_literals` ×5, `too_many_arguments` ×7, `same_item_push` ×1)

### 3. Baseline Resource & Performance Measurements (`v1.0.2`, 3 trials)
- **Cold sandbox install (`--sandbox`, 32 assets from GitHub):** `30,899.2 ms`, `12,656.7 ms`, `1,029.0 ms` (median **`12,656.7 ms`**). High tail variance traces directly to calling `WinHttpOpen` + `WinHttpConnect` + `LoadLibraryW("winhttp.dll")` cleanly from scratch on every single one of the 32 downloads (`WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY` WPAD/proxy probe + 32 TLS handshakes).
- **Warm sandbox install (cached verify + extract):** `73.6 ms`, `70.7 ms`, `58.8 ms` (median **`70.7 ms`**).
- **Idle Setup GUI (`v1.0.2-x64`):** `29.56 MB` Working Set (`30,998,528 B`), `7.00 MB` Private Memory (`7,340,032 B`), `351` Handles, `0.234s` startup CPU, `0.000s` measured delta over 5s (note: Windows `TotalProcessorTime` quantizes in `15.625 ms` ticks; also `TIMER_ANIM` at 30ms never stops cleanly if `diff <= 0.002` until `finished == true`, and hover/expose always re-runs full-window SDF rasterization).
- **Idle Live V6 Guardian (`PID 9072`):** `2.01 MB` WS (`2,109,440 B`), `1.36 MB` PM (`1,429,504 B`), `122` Handles, `0.000s` 5s delta CPU.
- **Disk Footprint (Sandbox):** `377,674 B` total (`90,664 B` cursors + `286,720 B` guardian + `290 B` log/report).

### 4. Critical Audit Highlights (Recorded in `docs/BUG_AUDIT.md`)
Confirmed 14 distinct defects across baseline (`BUG-01` through `BUG-14`), including all 6 pre-identified issues (A–F) plus `build.rs` silent empty-payload fallback, `assets_quick_ok` hash bypass, WinHTTP session/timeout/cap omissions + `LoadLibraryW` refcount leaks, partial cursor apply leaving mixed system handles, `cmd.exe /C` path interpolation, `Backbuffer::destroy()` deleting a selected `HBITMAP`, non-terminating `TIMER_ANIM`, stale `HWND` worker messages, and `Setup`/`Guardian` coupling.

---

## Batch 02 — Critical Installation Readiness, Safe Restore & Targeted Shutdown

1. **Observed Problems (`BUG-01`..`BUG-04`):** Silent guardian startup failures reported false `Ok`, restore wiped startup/uninstall metadata before verifying `backup-original.reg`, `--silent` discarded cursor-apply errors and blocked on `MessageBoxW`, and `taskkill /IM` indiscriminately targeted any process matching the filename.
2. **Design & Implementation:**
   - Replaced `stop_any_running_guardian()` (`taskkill /IM`) with `stop_targeted_guardians()` (`src/lib.rs`): verifies tray HWND PID + `CreateToolhelp32Snapshot` process list against `QueryFullProcessImageNameW` path inside the install root before posting `WM_CLOSE` and bounded `WaitForSingleObject`, with targeted `TerminateProcess` fallback only on verified PIDs.
   - Added `NamedReadyEvent` (`CreateEventW`/`WaitForMultipleObjects`), `RuntimeVersion`/`RuntimePid` verification, tray PID match, and `InstallSnapshot::rollback()` (`src/bin/setup.rs` & `src/bin/guardian.rs`): crashes or timeouts fail closed cleanly and roll back newly written state.
   - Added `validate_cursor_backup_bytes()` + `perform_validated_restore()` + `--restore-system-defaults`: every restore/uninstall validates `.reg` encoding, header, and strictly scoped `[HKEY_CURRENT_USER\Control Panel\Cursors]` sections BEFORE touching guardian/registry/files.
   - Fixed `--silent` error propagation and headless `main()` error handling (writes `setup-error.log` + stderr, zero `MessageBoxW` blocking).
3. **Tests Executed:**
   - Added 5 new unit/integration tests covering `ProcessEntry32W` ABI (x64 `568` B / x86 `556` B), `.reg` validation (UTF-16LE/UTF-8/corrupt/out-of-scope), transactional refusal on corrupt backup, guardian image-path verification, real child-process crash & timeout readiness detection, and headless detection.
   - `cargo test` (`1.99.0` x64): **`16/16 PASSED`** (`11` lib + `1` guardian + `4` setup).
   - `cargo +1.75.0 test --target x86_64-pc-windows-msvc`: **`16/16 PASSED`**.
   - `cargo +1.75.0 test --target i686-pc-windows-msvc`: **`16/16 PASSED`**.

---

## Batch 03 — Installer/Guardian Architecture Separation, Refactor & Packaging (`v1.1.0-rc.1`)

1. **Observed Problem (`BUG-11` & monolithic maintainability):** `src/lib.rs` (2,400+ lines) and `src/bin/setup.rs` (2,800+ lines) mixed unrelated responsibilities; Guardian owned `--uninstall` and `UninstallString` instead of Setup; `NoModify=1` disabled Windows Installed Apps maintenance; release/installed artifact names weren't uniformly standardized.
2. **Architectural Changes (ADR-001, ADR-002):**
   - Decomposed `src/lib.rs` into 6 focused modules: `registry.rs`, `assets.rs`, `cursors.rs`, `backup.rs`, `ipc.rs`, `render.rs`.
   - Split `src/bin/setup.rs` into `src/bin/setup/{main.rs, install.rs, ui.rs}` cleanly.
   - Bumped `Cargo.toml` to `1.1.0-rc.1` with binaries `ModernInvertedCursors-Setup` and `ModernInvertedCursors-Guardian` producing `ModernInvertedCursors-Setup-v1.1.0-rc.1-<arch>.exe` and `ModernInvertedCursors-Guardian-v1.1.0-rc.1-<arch>.exe`.
   - Setup now deploys BOTH persistent `ModernInvertedCursors-Setup.exe` (`UninstallString`, `QuietUninstallString`, `ModifyPath`, `NoModify=0`, `NoRepair=0`) and `ModernInvertedCursors-Guardian.exe`, dynamically computes `EstimatedSize` KB, and migrates from `v1.0.0–v1.0.2` (`cleanup_legacy_guardian_binary`, legacy `Run` key cleanup) while preserving valid `backup-original.reg`.
   - Guardian tray menu delegates **"Open Setup & Maintenance..."** (`--modify`) and **"Restore Original Cursors..."** (`--uninstall`) to installed `ModernInvertedCursors-Setup.exe`, retaining standalone safe fallback only when Setup is absent.
   - Updated `.github/workflows/ci-and-release.yml` to run CI on `hardening/**` branches and mark `-rc` tags `--prerelease` automatically.
3. **Tests Executed:**
   - Added 3 new tests (`ASSETS.len() == 32`, `test_detect_and_migrate_legacy_v1_preserves_valid_backup`, `test_sync_setup_maintenance_copy_skips_same_path_and_copies_external`).
   - `build.ps1` release build + `18/18` tests passed across `x64` (`1.99.0`), `win7-win10-x64` (`1.75.0`), and `win7-win10-x86` (`1.75.0`).

---

## Batch 04 — Security, Integrity, FFI & Resource-Lifetime Hardening

1. **Changes Implemented (`BUG-05`..`BUG-10`, `SEC-01`..`SEC-06`):**
   - `build.rs`: fails closed (`panic!`) on release builds or explicit `GUARDIAN_EMBED_PATH` if the target Guardian PE payload is missing, `<10 KB`, or mismatches target `Machine`.
   - `src/registry.rs`: added `load_system_dll()` (`GetModuleHandleW` -> `LOAD_LIBRARY_SEARCH_SYSTEM32` -> `GetSystemDirectoryW` explicit fallback) used by WinHTTP, DWM, DPI, and WTS loaders cleanly.
   - `src/assets.rs`: removed `assets_quick_ok()` size-only hash bypass; cached `winhttp.dll` exports once via `OnceLock`; added `WinHttpHostSession` per-worker TLS reuse + `WinHttpSetTimeouts(10s, 20s, 15s, 20s)` + `MAX_ASSET_BYTES (256 KB)` cap + `AtomicBool` sibling cancellation on failure. Note: initial 10s connect timeout test hit a `70.9s` cold retry storm when host ISP `curl.exe -4` took `11.6s` on a single request; adjusting to `WINHTTP_ACCESS_TYPE_DEFAULT_PROXY` + `20s` connect timeout eliminated false connect-timeout retries cleanly (`39.2s, 1.42s, 15.1s`).
   - `src/cursors.rs`: pre-loads and validates all required cursor handles via `LoadImageW` before modifying `HKCU\Control Panel\Cursors` or calling `SetSystemCursor`, rolling back via `DestroyCursor` + `reload_cursors()` on mid-apply errors.
   - `src/ipc.rs`: replaced raw path string interpolation in `schedule_self_delete()` with `MIC_DEL_TARGET` env-var passing + `/D /Q` + `is_safe_cleanup_target()`.
   - `src/bin/setup/`: added `WorkerTarget` (`AtomicBool` + `AtomicUsize` cleared in `WM_DESTROY`), `Backbuffer::destroy()` `old_bm` restoration before `DeleteObject`/`DeleteDC`, Setup single-instance mutex (`Local\ModernInvertedCursors_Setup_V1_Mutex`), and early `state_ptr` cleanup on window creation failure.
2. **Tests & Verification:**
   - Added 4 new tests (`test_verify_asset_file_rejects_same_size_tampered_bytes`, `test_load_system_dll_resolves_system32_and_blocks_fake`, `test_is_safe_cleanup_target_allows_only_expected_executables`, `test_register_and_apply_missing_dir_fails_atomically_before_mutating`).
   - `build.rs` fail-closed test confirmed panic on invalid `GUARDIAN_EMBED_PATH`.
   - Warm `--sandbox` improved from `70.7 ms` baseline to `52.1 ms` median (`53.8, 52.1, 51.5 ms`).
   - `22/22` unit & integration tests passed across modern `x64` (`1.99.0`), legacy `x64` (`1.75.0`), and legacy `x86` (`1.75.0`).
