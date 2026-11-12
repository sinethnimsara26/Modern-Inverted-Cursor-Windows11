# Test Evidence & Execution Log

Records exact commands, commit SHAs, tool outputs, and CI links. Zero fabricated results.

---

## Baseline Checkpoint (`398f09b0b2c89c633d336edbf097789aad0e8256` — `main` / `v1.0.2`)

### 1. Local Static Checks (`2026-11-12`)
- `cargo fmt --check`: **FAILED** (whitespace/wrapping diffs in `build.rs`, `src/bin/guardian.rs`, `src/bin/setup.rs`).
- `cargo clippy --all-targets`: **EXITED 0 WITH 21 WARNINGS** (`6` in `src/lib.rs` + `1` test, `2` in `src/bin/guardian.rs`, `13` in `src/bin/setup.rs`).
- `cargo test`: **9/9 PASSED** (`6` lib + `1` guardian + `2` setup).

### 2. Local Baseline Benchmarks (`ModernInvertedSetup-v1.0.2-x64.exe` `--sandbox`)
- Cold trials (3 runs): `30,899.2 ms`, `12,656.7 ms`, `1,029.0 ms` (median `12,656.7 ms`)
- Warm trials (3 runs): `73.6 ms`, `70.7 ms`, `58.8 ms` (median `70.7 ms`)
- Setup GUI idle (`3s -> 8s`): `29.56 MB` WS (`30,998,528 B`), `7.00 MB` Private (`7,340,032 B`), `351` Handles, `0.234375s` initial CPU.
- Live Guardian (`PID 9072`, `ModernInvertedCursorGuardian.exe`): `2.01 MB` WS (`2,109,440 B`), `1.36 MB` Private (`1,429,504 B`), `122` Handles.

### 3. Upstream GitHub Actions Evidence on Baseline
- `v1.0.2` passing run [`#43177846627`](https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11/actions/runs/43177846627): all 5 jobs passed (`arm64`, `win7-win10-x64`, `win7-win10-x86`, `x64`, `publish-release`).
- Prior failure [`#43177594470`](https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11/actions/runs/43177594470): PowerShell positional array splat bug (`@argsList`), fixed in `398f09b`.
- Prior failure [`#43162190599`](https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11/actions/runs/43162190599): `gh release create` failed without `--repo` because `publish-release` lacked `.git`, fixed in `a025f6a`.

---

## Batch 02 Test Evidence

- `cargo test` (`1.99.0`, `x86_64-pc-windows-msvc`): **`16/16 PASSED`** (`11` lib + `1` guardian + `4` setup)
- `cargo +1.75.0 test --target x86_64-pc-windows-msvc`: **`16/16 PASSED`**
- `cargo +1.75.0 test --target i686-pc-windows-msvc`: **`16/16 PASSED`**
- Verified behavior covers `BUG-01` (`test_verify_child_readiness_detects_crash_and_timeout`, `test_named_ready_event_detects_early_exit_and_timeout`), `BUG-02` (`test_validate_cursor_backup_bytes_*`, `test_perform_validated_restore_refuses_corrupt_or_missing_backup_before_touching_state`), `BUG-03` (`test_headless_detection_covers_silent_and_automated_flags`), and `BUG-04` (`test_is_guardian_image_path_enforces_install_root_and_binary_name`, `test_process_entry_32w_matches_windows_sdk_size`).

---

## Batch 03 Test Evidence (`v1.1.0-rc.1`)

- `build.ps1 -Arch x64 -Suffix x64`: Release build + **`18/18 PASSED`** (`12` lib + `1` guardian + `5` setup), `0` warnings.
- `build.ps1 -Arch x64 -Suffix win7-win10-x64 -Toolchain 1.75.0 -Target x86_64-pc-windows-msvc -StaticCrt`: Release build + **`18/18 PASSED`**.
- `build.ps1 -Arch x86 -Suffix win7-win10-x86 -Toolchain 1.75.0 -Target i686-pc-windows-msvc -StaticCrt`: Release build + **`18/18 PASSED`**.
- Generated artifacts in `dist/`:
  - `ModernInvertedCursors-Setup-v1.1.0-rc.1-x64.exe` (`675,840 B`), `ModernInvertedCursors-Guardian-v1.1.0-rc.1-x64.exe` (`298,496 B`)
  - `ModernInvertedCursors-Setup-v1.1.0-rc.1-win7-win10-x64.exe`, `ModernInvertedCursors-Guardian-v1.1.0-rc.1-win7-win10-x64.exe`
  - `ModernInvertedCursors-Setup-v1.1.0-rc.1-win7-win10-x86.exe` (`798,208 B`), `ModernInvertedCursors-Guardian-v1.1.0-rc.1-win7-win10-x86.exe` (`363,008 B`)
