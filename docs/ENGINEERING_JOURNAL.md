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
