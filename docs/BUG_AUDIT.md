# Bug & Engineering Audit (`v1.0.2` Baseline `398f09b`)

Every bug listed below was directly verified by inspecting the baseline source code, running compilers/linters/tests, or inspecting live/sandbox execution on `398f09b0b2c89c633d336edbf097789aad0e8256`.

---

| ID | Severity | Area | Source Location | Summary | Planned Batch |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **BUG-01** | **High** | Setup -> Guardian Readiness | `src/bin/setup.rs:1839-1852` | Launches guardian, polls `FindWindowW` max 2s, and returns `Ok` even if process crashed immediately or timed out | Batch 02 |
| **BUG-02** | **High** | Restore / Uninstall Safety | `src/bin/setup.rs:1924-1946`, `2483-2495`; `src/bin/guardian.rs:536-544`, `775-810` | Wipes `Run`, `Preset`, `Uninstall`, and `ModernInvertedCursorGuardian.exe` BEFORE validating or importing `backup-original.reg` | Batch 02 |
| **BUG-03** | **High** | Silent Install Error Propagation | `src/bin/setup.rs:2512`, `2544-2551` | `--silent` ignores `register_and_apply()` result (`let _ = ...`) and `main()` blocks automated sessions with `MessageBoxW` on error | Batch 02 |
| **BUG-04** | **High** | Indiscriminate Process Kill | `src/lib.rs:1447-1456` | `stop_any_running_guardian()` unconditionally runs `taskkill.exe /IM ModernInvertedCursorGuardian.exe /F` without PID/path checks | Batch 02 |
| **BUG-05** | **High** | Build Fail-Open Payload | `build.rs:35-37` | Falls back to 0-byte `embedded_guardian.exe` if `dist/` lacks guardian EXE, producing broken Setup EXE via `cargo build` | Batch 04 |
| **BUG-06** | **High** | Partial Live Cursor Apply | `src/lib.rs:1300-1373` | Writes registry and mutates cursor slots 1..17 one-by-one; mid-loop `LoadImageW`/`SetSystemCursor` failure leaves half-applied cursors | Batch 04 |
| **BUG-07** | **Medium-High** | Hash Bypass Cache Check | `src/lib.rs:856-863`, `1136` | `assets_quick_ok()` validates byte size only, skipping `git_blob_sha1()` on existing files during guardian start/reapply | Batch 04 |
| **BUG-08** | **Medium-High** | WinHTTP Timeouts, Caps & Leaks | `src/lib.rs:924-1067`, `1181` | New `WinHttpOpen`+`LoadLibraryW("winhttp.dll")` per asset (32× refcount leak & 30.9s cold variance), no timeouts/max-bytes/cancellation | Batch 04 / 06 |
| **BUG-09** | **Medium** | `cmd.exe /C` Path Interpolation | `src/lib.rs:1476-1486` | `schedule_self_delete()` passes unescaped `path.display()` inside `cmd.exe /C "... & del ..."` | Batch 04 |
| **BUG-10** | **Medium** | GDI HBITMAP / Dangling Worker HWND / Setup Mutex | `src/bin/setup.rs:717-725`, `1886`, `2285`, `2516` | `Backbuffer::destroy()` calls `DeleteObject(hbm)` while selected in `hdc`; worker posts to unchecked `hwnd`; no Setup single-instance guard; `state_ptr` leaks on early `CreateWindowExW` err | Batch 04 / 06 |
| **BUG-11** | **Medium** | Setup/Guardian Architecture Coupling | `src/bin/setup.rs`, `src/bin/guardian.rs` | Guardian owns `--uninstall` and `UninstallString`; Setup not installed as maintenance app; `NoModify=1` | Batch 03 |
| **BUG-12** | **Medium** | UI Idle Animation Timer & Full-Frame Redraws | `src/bin/setup.rs:822`, `2182-2190` | `TIMER_ANIM` never stops when `diff <= 0.002`; hover/expose repaints entire 4×4 SDF surface & 4 cursor previews | Batch 06 |
| **BUG-13** | **Medium** | Incomplete Functional & UI Tests | `tests`, `ci-and-release.yml` | No backup/restore failure tests, no IPC crash/timeout tests, no `--silent` failure exit test, no UI Interaction / multi-DPI (`100–200%`) check | Batch 07 |
| **BUG-14** | **Low-Medium** | Doc Drift & Lint Hygiene | `docs/TESTS_AND_BENCHMARKS.md`, `README.md`, `cargo fmt/clippy` | Claims `8/8` tests when `9` exist; `cargo fmt --check` fails; 21 Clippy warnings; outdated `v1.0.0` CLI examples | Batch 01–08 |

---

### Verification & Fix Evidence Log
*(Updated per batch as each defect is fixed and regression-tested.)*
