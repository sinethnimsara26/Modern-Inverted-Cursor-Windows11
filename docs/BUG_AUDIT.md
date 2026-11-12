# Bug & Engineering Audit (`v1.0.2` Baseline `398f09b`)

Every bug listed below was directly verified by inspecting the baseline source code, running compilers/linters/tests, or inspecting live/sandbox execution on `398f09b0b2c89c633d336edbf097789aad0e8256`.

---

| ID | Severity | Area | Source Location | Summary | Planned Batch |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **BUG-01** | **High** | Setup -> Guardian Readiness | `src/bin/setup.rs`, `src/lib.rs`, `src/bin/guardian.rs` | Launches guardian, polls `FindWindowW` max 2s, and returns `Ok` even if process crashed immediately or timed out | **FIXED (Batch 02)** |
| **BUG-02** | **High** | Restore / Uninstall Safety | `src/bin/setup.rs`, `src/bin/guardian.rs`, `src/lib.rs` | Wipes `Run`, `Preset`, `Uninstall`, and Guardian EXE BEFORE validating or importing `backup-original.reg` | **FIXED (Batch 02)** |
| **BUG-03** | **High** | Silent Install Error Propagation | `src/bin/setup.rs` | `--silent` ignores `register_and_apply()` result (`let _ = ...`) and `main()` blocks automated sessions with `MessageBoxW` on error | **FIXED (Batch 02)** |
| **BUG-04** | **High** | Indiscriminate Process Kill | `src/lib.rs` | `stop_any_running_guardian()` unconditionally runs `taskkill.exe /IM ModernInvertedCursorGuardian.exe /F` without PID/path checks | **FIXED (Batch 02)** |
| **BUG-05** | **High** | Build Fail-Open Payload | `build.rs` | Falls back to 0-byte `embedded_guardian.exe` if `dist/` lacks guardian EXE, producing broken Setup EXE via `cargo build` | **FIXED (Batch 04)** |
| **BUG-06** | **High** | Partial Live Cursor Apply | `src/cursors.rs` | Writes registry and mutates cursor slots 1..17 one-by-one; mid-loop `LoadImageW`/`SetSystemCursor` failure leaves half-applied cursors | **FIXED (Batch 04)** |
| **BUG-07** | **Medium-High** | Hash Bypass Cache Check | `src/assets.rs` | `assets_quick_ok()` validates byte size only, skipping `git_blob_sha1()` on existing files during guardian start/reapply | **FIXED (Batch 04)** |
| **BUG-08** | **Medium-High** | WinHTTP Timeouts, Caps & Leaks | `src/assets.rs` | New `WinHttpOpen`+`LoadLibraryW("winhttp.dll")` per asset (32× refcount leak & 30.9s cold variance), no timeouts/max-bytes/cancellation | **FIXED (Batch 04)** |
| **BUG-09** | **Medium** | `cmd.exe /C` Path Interpolation | `src/ipc.rs` | `schedule_self_delete()` passes unescaped `path.display()` inside `cmd.exe /C "... & del ..."` | **FIXED (Batch 04)** |
| **BUG-10** | **Medium** | GDI HBITMAP / Dangling Worker HWND / Setup Mutex | `src/bin/setup/ui.rs`, `src/bin/setup/main.rs` | `Backbuffer::destroy()` calls `DeleteObject(hbm)` while selected in `hdc`; worker posts to unchecked `hwnd`; no Setup single-instance guard; `state_ptr` leaks on early `CreateWindowExW` err | **FIXED (Batch 04)** |
| **BUG-11** | **Medium** | Setup/Guardian Architecture Coupling | `src/bin/setup/`, `src/bin/guardian.rs`, `src/registry.rs` | Guardian owns `--uninstall` and `UninstallString`; Setup not installed as maintenance app; `NoModify=1` | **FIXED (Batch 03)** |
| **BUG-12** | **Medium** | UI Idle Animation Timer & Full-Frame Redraws | `src/bin/setup/ui.rs` | `TIMER_ANIM` never stops when `diff <= 0.002`; hover/expose repaints entire 4×4 SDF surface & 4 cursor previews | **FIXED (Batch 06)** |
| **BUG-13** | **Medium** | Incomplete Functional & UI Tests | `tests/`, `ci-and-release.yml` | No backup/restore failure tests, no IPC crash/timeout tests, no `--silent` failure exit test, no UI interaction / multi-DPI (`100–200%`) check | **FIXED (Batch 07)** |
| **BUG-14** | **Low-Medium** | Doc Drift & Lint Hygiene | `docs/TESTS_AND_BENCHMARKS.md`, `README.md`, `cargo fmt/clippy` | Claims `8/8` tests when `9` exist; `cargo fmt --check` fails; 21 Clippy warnings; outdated `v1.0.0` CLI examples | Batch 01–08 |
| **BUG-15** | **Medium** | Silent CLI Flag Fall-Through & Inherited Child Handles | `src/registry.rs`, `src/bin/setup/install.rs` | Discovered in Batch 07: unknown flags fell through to default preset in `--silent`; spawned Guardian inherited caller handles without `DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP` | **FIXED (Batch 07)** |

---

### Verification & Fix Evidence Log
*(Updated per batch as each defect is fixed and regression-tested.)*
