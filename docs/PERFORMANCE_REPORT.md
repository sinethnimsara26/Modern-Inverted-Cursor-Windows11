# Performance & Resource Report

## Methodology & Measurement Caveats
- **Host:** Windows 11 (`x64`), 125% DPI (`dpi=120`), active NVMe/local disk, live internet connection to `raw.githubusercontent.com`.
- **CPU Measurement Precision:** `Get-Process.TotalProcessorTime` uses Windows' standard `15.625 ms` clock tick granularity, meaning very brief wakeups over a 5-second idle interval can read `0.000000s` even if small timer ticks occur; therefore we also inspect `TIMER_*` frequencies, Win32 API call counts, GDI/kernel handles, and `QueryProcessCycleTime`/longer sampling windows instead of blindly asserting absolute "0% CPU".
- **Variability:** Cold network installs vary due to TLS handshakes, WPAD proxy lookup latencies, CDN edge routing, and GitHub rate/cache state; reporting 3+ trials with median prevents single-run bias.

---

## Baseline Measurements (`v1.0.2` at `398f09b`, before hardening)

| Metric | Baseline Measurement (`v1.0.2`) | Root Cause / Observation | Target Optimization |
| :--- | :--- | :--- | :--- |
| **Cold install duration (32 assets)** | Trials: `30,899.2 ms`, `12,656.7 ms`, `1,029.0 ms` (Median: **`12,656.7 ms`**) | `WinHttpOpen` + `WinHttpConnect` + `LoadLibraryW("winhttp.dll")` called 32 separate times; auto-proxy probe per file | Per-worker/shared keep-alive session + timeouts (Batch 04/06) |
| **Warm install duration (cached)** | Trials: `73.6 ms`, `70.7 ms`, `58.8 ms` (Median: **`70.7 ms`**) | Fast parallel file hash check | Keep <= baseline while eliminating `assets_quick_ok` hash bypass |
| **Network payload (32 assets)** | `90,664 B` across 32 files | Minimal upstream asset size | Bounded download limit (`MAX_ASSET_BYTES`) |
| **Disk footprint (sandbox)** | `377,674 B` total (`286,720 B` guardian + `90,664 B` assets) | Single binary setup doesn't stay on disk; after Batch 03 Setup/Maintenance EXE stays too | Compact size + trimmed deps |
| **Guardian idle (`PID 9072`)** | `2.01 MB` WS (`2,109,440 B`), `1.36 MB` PM (`1,429,504 B`), `122` Handles | 122 handles on V6 `v1.0.0` binary; single event-driven wait | Audit handles, `FreeLibrary` / HKEY lifecycle, idle timers (Batch 05) |
| **Setup GUI idle (`3s -> 8s`)** | `29.56 MB` WS (`30,998,528 B`), `7.00 MB` PM (`7,340,032 B`), `351` Handles, `0.234s` startup CPU | Re-rasterizes entire 4×4 SDF surface & 4 cursor previews on every `WM_PAINT`; `TIMER_ANIM` stays running at 30ms; `LoadLibraryW` repeated; `HBITMAP` leaked before `DeleteDC` | Static layer caching, dirty redraw, stop `TIMER_ANIM` when settled, GDI fix (Batch 06) |

---

## Optimization Results (Before vs After)

### Batch 04 — WinHTTP Session Reuse, Timeouts & Size Caps
- **Change:** Removed `assets_quick_ok()` size-only hash bypass; cached `winhttp.dll` System32 exports via `OnceLock`; reused per-worker `WinHttpHostSession` keep-alive connections (`WINHTTP_ACCESS_TYPE_DEFAULT_PROXY` primary with `DIRECT`/`4` fallback); bounded `WinHttpSetTimeouts(10_000, 20_000, 15_000, 20_000)` and `MAX_ASSET_BYTES = 256 * 1024`; sibling worker `AtomicBool` cancellation.
- **Measured Warm Cached Validation (`ModernInvertedCursors-Setup-v1.1.0-rc.1-x64.exe --sandbox`, 3 trials):**
  - Baseline `v1.0.2` (with weak `assets_quick_ok`): median `70.7 ms` (`73.6, 70.7, 58.8 ms`)
  - Batch 04 `v1.1.0-rc.1` (WITH full SHA-1 verification on every asset): warm median improved to **`52.1 ms`** despite stricter cryptographic verification.
- **Measured Cold Network Install (3 trials under local ISP TLS variability):** `39,172.6 ms`, `1,422.5 ms`, `15,122.6 ms` (note: cold HTTPS handshakes to `raw.githubusercontent.com` from this network varied between `1.4s` and `11.6s+` per connect attempt during testing).

### Batch 05 — Guardian Hot-Path Reapply, Registry Elision & Idle Footprint Optimization
- **Changes:**
  - `AssetVerifyCache::ensure_all_assets_cached`: keeps full cryptographic Git-blob SHA-1 verification on startup/drift/manual reapply (`force_full_hash=true`), while hot reapplies verify in-process `(byte_len, mtime_secs, mtime_nanos)` stamps before touching file contents.
  - `reg_sync_cursor_values`: opens `HKCU\Control Panel\Cursors` once instead of 19 separate `RegCreateKeyExW`/`RegCloseKey` pairs and elides writes when values already match verbatim; elided redundant pre-`SetSystemCursor` `SPI_SETCURSORS` disk cursor reload inside `register_and_apply()`.
  - `guardian.rs`: skips redundant `Shell_NotifyIconW(NIM_MODIFY)` when `(paused, preset, px, step)` is unchanged; calls `trim_working_set()` after `apply_now()` and popup menu dismissal; added isolated `--sandbox <dir>` mode.
- **Measured Before vs After (`x64` release microbench, 5 trials of 50 iterations each):**
  - `ensure_all_assets` (`x50` calls): `406.578 ms` median (`8.132 ms/call`) -> **`104.351 ms` median (`2.087 ms/call`)** (**3.90x faster**, zero disk reads on cache hit).
  - Registry cursor scheme sync (`x50` calls): `32.009 ms` median (`0.640 ms/call` for 19 separate writes; `38.631 ms` / `0.773 ms` for 19 naive reads) -> **`8.594 ms` median (`0.172 ms/call`)** single-HKEY diff-check (**3.72x faster**, zero writes on match).
- **Live Isolated Guardian Process Telemetry (`ModernInvertedCursors-Guardian-v1.1.0-rc.1-x64.exe --tray --sandbox`, vs Baseline Live `v1.0.0` `PID 9072`):**
  - Baseline `v1.0.0` (`PID 9072`): `2.01 MB` WS (`2,109,440 B`), `1.36 MB` Private (`1,429,504 B`), `122` Handles.
  - Batch 05 Sandbox Guardian (`PID 9040`, 3s idle): **`0.75 MB` WS (`782,336 B`, -62.9%)**, `1.52 MB` Private (`1,593,344 B`), **`116` Handles (-6)**, `4` Threads.
  - Sandbox startup-to-ready latency: First cold AV-scan run `3,108.1 ms`; subsequent 3 warm launches **`233.9 ms`, `66.1 ms`, `71.0 ms` (median `71.0 ms`)**.

### Batch 06 — Setup GUI Repaint Caching, Timer Discipline, Working-Set Trim & Keyboard Accessibility
- **Changes:**
  - `CursorGlyphSprite` + `GlyphSpriteCache`: pre-rasterizes 4×4 supersampled SDF cursor glyphs once per DPI and composites via fixed-point integer alpha blending instead of running half a million floating-point segment distance evaluations on every hover repaint.
  - `GuiState.dirty`: `WM_PAINT` skips full `render_ui()` on clean OS expose / Alt+Tab paints and performs a single `BitBlt` from the retained `Backbuffer`.
  - `TIMER_ANIM` auto-stop: kills `TIMER_ANIM` (`30 ms`) immediately once `(target_pct - anim_pct).abs() <= 0.002` and re-arms only when new progress arrives.
  - `trim_working_set()` called after initial GUI settle and on worker completion.
- **Measured Before vs After (`ModernInvertedCursors-Setup-v1.1.0-rc.1-x64.exe`):**
  - **50× UI cursor-glyph redraws (`43px @ 125% DPI`, 5 glyphs/frame, 5 trials):** Direct 4×4 SDF `299.225 ms` median (`289.52, 294.18, 299.23, 299.56, 300.60 ms`) -> **`1.034 ms` median** (`1.012, 1.017, 1.034, 1.052, 1.066 ms`, **`289.4x` faster** per UI repaint!).
  - **Live Setup GUI Idle Working Set (`3s`/`8s`):** Pre-Batch-06 `29.51 MB` (`30,945,280 B`) -> Post-Batch-06 **`22.86 MB` (`23,973,888 B`, `-22.5%`)**, `6.97 MB` Private (`7,311,360 B`), `353` Handles, `0.234s` CPU.
  - **`--smoke-ui` 3-trial run (`4` BMPs rendered + written in Batch 06 vs `3` BMPs Pre-B6):** `209.5 ms` median (`6,331.8, 209.5, 199.1 ms`) vs Pre-B6 `209.9 ms` (`2,353.7, 191.6, 209.9 ms`).
