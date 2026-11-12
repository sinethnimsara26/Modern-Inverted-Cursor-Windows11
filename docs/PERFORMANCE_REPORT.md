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

*(Guardians and Installer GUI optimizations will be measured and populated in Batches 05–07.)*
