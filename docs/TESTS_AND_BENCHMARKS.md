# Test Results & Benchmarks

All tests and benchmarks below were executed directly on **Windows 11 (`x86_64-pc-windows-msvc`, 125% display DPI / `dpi=120`)** using Rust `1.99.0` (`rustc 1.99.0`, `cargo 1.99.0`).

---

## 1. Automated Unit Tests (`cargo test`)

**Result: `8/8 PASSED` (`0` failed, `0` warnings)** across all three crate targets:

| Target | Test | What It Verifies | Status |
| :--- | :--- | :--- | :--- |
| `src/lib.rs` | `test_git_blob_sha1_matches_git_spec` | Pure-Rust Git blob SHA-1 hash (`blob 6\0hello\n`) matches Git CLI digest | PASS |
| `src/lib.rs` | `test_balanced_cursor_size_progression` | Clamping & 4 px balanced steps (`0->32`, `1->32`, `2->36`, `15->88`, `99->88`) | PASS |
| `src/lib.rs` | `test_preset_cli_and_roles_match` | All 4 CLI presets round-trip and generate all 17 Windows cursor roles | PASS |
| `src/lib.rs` | `test_split_cursor_glyph_renders_distinct_presets` | 4×4 supersampled glyph rasterizer renders distinct non-empty output per preset | PASS |
| `src/lib.rs` | `test_create_split_cursor_hicon_succeeds` | In-memory `CreateDIBSection` + `CreateIconIndirect` produces valid Win32 `HICON` | PASS |
| `src/bin/guardian.rs` | `test_win32_tray_struct_sizes_match_sdk` | 64-bit Win32 ABI sizes match SDK headers (`NOTIFYICONDATAW=976`, `MSG=48`) | PASS |
| `src/bin/setup.rs` | `test_win32_setup_structs_match_sdk` | 64-bit Win32 GUI struct sizes (`TRACKMOUSEEVENT=24`, `MONITORINFO=40`, `PAINTSTRUCT=72`, `BITMAPINFOHEADER=40`) | PASS |
| `src/bin/setup.rs` | `test_embedded_guardian_is_nonempty_pe_binary` | Embedded `ModernInvertedCursorGuardian.exe` slice is non-empty valid `MZ` PE binary | PASS |

---

## 2. Binary & PE Subsystem Verification (x64 & Native ARM64)

| Artifact | Architecture | File Size | SHA-256 | PE Machine / Subsystem |
| :--- | :--- | :--- | :--- | :--- |
| `ModernInvertedCursorGuardian-x64.exe` (`v1.0.1`) | `x86_64-pc-windows-msvc` | **`276,480` bytes (~270 KB)** | `8def6f2e336342b4a2e34f8c3d4cc4ebd0c33b4eab8a7dd2a2187c62d763600d` | `0x8664` / **`2` (Windows GUI)** |
| `ModernInvertedSetup-v1.0.1-x64.exe` | `x86_64-pc-windows-msvc` | **`604,672` bytes (~590 KB)** | `0da19df371dd078935f9e81f41a1112511b86ed24ac36fb340940eea72f8244c` | `0x8664` / **`2` (Windows GUI)** |
| `ModernInvertedCursorGuardian-arm64.exe` (`v1.0.1`) | `aarch64-pc-windows-msvc` | **`257,024` bytes (~251 KB)** | `c448d246792017842c347badd1b5f44e8485ceb932777d9f4f93bc6175b1512d` | `0xAA64` / **`2` (Windows GUI)** |
| `ModernInvertedSetup-v1.0.1-arm64.exe` | `aarch64-pc-windows-msvc` | **`560,640` bytes (~547.5 KB)** | `133315c1aa0aaf9c775fb427ac2a6a7c6c3f9862f0c0c89e9d07b532bee8f0b1` | `0xAA64` / **`2` (Windows GUI)** |
| `ModernInvertedSetup-v1.0.0.exe` (`v1.0.0` initial) | `x86_64-pc-windows-msvc` | `604,672` bytes (~590 KB) | `11975328bfffd07e1cbe1e4ad1b9d3b382d78800117ea28a352b8a306806ece5` | `0x8664` / `2` (Windows GUI) |
| *Baseline V5.1 (for comparison)* | `x86_64` | `5,161,090` bytes (~4.92 MB) | *(legacy-v5.1)* | `3` (Console — opens Windows Terminal) |

*(Native ARM64 binaries built, unit-tested, PE-checked, and UI-smoke-verified directly on GitHub Actions `windows-11-arm` runner in run `#43162511198`.)*

---

## 3. UI Dark-Mode Smoke Verification (`--smoke-ui`)

Executed `ModernInvertedSetup-v1.0.0.exe --smoke-ui`:

```text
SMOKE_UI_OK dpi=120 dwm_dark=true hits_cleared=2 size=775x675
```

- Confirmed `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)` succeeded (`dwm_dark=true`).
- Confirmed Per-Monitor DPI V2 scaling automatically scaled `620×540` logical DIPs to `775×675` physical pixels at 125% scaling (`dpi=120`).
- Visually inspected all three generated screens (`docs/images/01_setup_dark.png`, `02_installing_dark.png`, `03_done_dark.png`).

---

## 4. Live Functional & Integration Tests (Guardian)

1. **Tray Window & Graceful Shutdown (`--quit`):**
   - Verified `FindWindowW("ModernInvertedCursorGuardianTrayWnd", "Modern Inverted Cursor Guardian")` returned a live `HWND` (`459866`, `procs=1`) when active.
   - Invoked `--quit` -> returned `HWND=0` and `procs=0` cleanly (zero leaked processes, zero ghost tray icons).
2. **Single-Instance Mutex (`--tray` re-launch):**
   - Launching a second guardian instance detected the named mutex (`ERROR_ALREADY_EXISTS`), posted `WM_APP_REAPPLY` to the active window (`Applied Compact at 36px (Accessibility 2/15)` logged), and exited with code `0`.
3. **Live Accessibility Change Watcher (`CursorSize` `2 → 3 → 2`):**
   - Updating `HKCU\Software\Microsoft\Accessibility\CursorSize` live caused `RegNotifyChangeKeyValue` + 400 ms debounce to log:
     - `Applied Compact at 40px (Accessibility 3/15)`
     - `Applied Compact at 36px (Accessibility 2/15)`

---

## 5. Measured Performance & Resource Benchmarks

### Registry Write Latency (20 Cursor Keys)
Measured directly on Windows 11 against `HKCU`:

| Implementation | Method | Elapsed Time (20 writes) | Speedup |
| :--- | :--- | :--- | :--- |
| V5.1 Guardian | 20× `reg.exe add ...` subprocess spawns | `795.13 ms` | `1.0×` |
| **V6 / v1.0.0** | Native `advapi32.dll` (`RegCreateKeyExW` + `RegSetValueExW`) | **`2.92 ms`** | **`271.9× faster`** |

### Installer End-to-End Timing (`--sandbox` mode)
Measured against `ModernInvertedSetup-v1.0.0.exe` running the real parallel `curl.exe` + SHA-1 pipeline into a clean directory:

| Scenario | Operations | Observed Time |
| :--- | :--- | :--- |
| **Cold install (new PC simulation)** | Download all 32 `.cur`/`.ani` files from GitHub (4 workers) + Git blob SHA-1 verify + extract `276,480`-byte guardian | **`29,333.5 ms`** (`~29.3 s` network-bound) |
| **Warm verify / update** | Verify all 32 cached files by size & Git blob SHA-1 + extract guardian | **`1,043.5 ms`** (`~1.04 s`) |

### Guardian Runtime Resource Footprint
Observed live via `Get-Process` on Windows 11:

| Stage | Working Set | Private Memory | Total CPU Time |
| :--- | :--- | :--- | :--- |
| Immediately after startup & cursor apply (`V6`) | **`471 KB – 545 KB`** (`471,040`–`544,768` bytes) | **`~1.45 MB`** (`1,515,520` bytes) | `0.062s`–`0.078s` |
| Idle background running (30+ minutes, `PID 12984`) | **`~2.29 MB`** (`2,404,352` bytes) | **`~1.37 MB`** (`1,433,600` bytes) | **`0.078s`** (`0.00%` idle CPU) |
| Legacy V5.1 Console Guardian (for comparison) | `7,528,448` bytes (~7.18 MB) + Windows Terminal host | — | Opens full Terminal tab |
