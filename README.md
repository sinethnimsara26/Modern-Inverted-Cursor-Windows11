# Modern Inverted Cursors — Windows 11 Setup & Tray Guardian (`v1.0.x`)

A lightweight, **pure-Rust (zero external crates)**, **100% FOSS (MIT Licensed)** Windows 10/11 installer and background system-tray guardian for **[Modern Inverted Mouse Cursors v3.6.2](https://github.com/emvaized/modern_inverted_mouse_cursors)** — preserving true dynamic XOR pixel inversion with balanced pointer sizing and **zero console/terminal windows**.

> **FOSS & Community Contributions Welcome:** Everything here is **100% Free and Open Source Software (MIT License)**. Anyone is completely free to use, tweak, fork, or build their own version! If you spot any bugs, Windows quirks, or improvements, please flag them anytime in **[GitHub Issues](https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11/issues)** or open a Pull Request.
>
> **Release Series:** `v1.0.0` (x64) and `v1.0.1` (x64 + native ARM64) make up the **initial first release (`v1.0.x`)**. Future updates may follow as we discover new bugs, edge cases, or Windows updates.

| Setup Screen | Installing Screen | Done Screen |
| :---: | :---: | :---: |
| ![Setup Screen](docs/images/01_setup_dark.png) | ![Installing Screen](docs/images/02_installing_dark.png) | ![Done Screen](docs/images/03_done_dark.png) |

---

## Why This Exists

Windows 11's built-in **Inverted** accessibility cursor style provides live pixel-level color inversion, but its stock pointer artwork is decades old. **Modern Inverted Mouse Cursors v3.6.2** solves the artwork while keeping XOR inversion — however, applying custom `.cur`/`.ani` packs on Windows 11 runs into three platform quirks:

1. `SystemParametersInfoW(SPI_SETCURSORS)` frequently fails silently or leaves Accessibility scaling active on Windows 11 builds (`GetLastError=0`).
2. Adjusting **Settings → Accessibility → Mouse pointer and touch → Size** overwrites custom cursors with stock pointers and scales `CursorBaseSize` aggressively (`32 → 256 px`, causing severe pixelation on low-bit-depth XOR masks).
3. Running a CLI guardian at Windows sign-in opens a full **Windows Terminal** window on the taskbar; closing it kills cursor protection.

This project solves all three cleanly in **pure Rust + Win32 APIs** (`std` only, zero crates.io dependencies):

- **Sleek Dark-Mode Setup (`ModernInvertedSetup-v1.0.0.exe`, ~590 KB):** Native Windows 11 dark title bar (`DWMWA_USE_IMMERSIVE_DARK_MODE`), rounded corners, Per-Monitor DPI V2 scaling, 4 vector preset previews, parallel 4-worker SHA-1 verified asset download, and Windows **Installed Apps** integration (`HKCU\...\Uninstall`).
- **Silent System-Tray Guardian (`ModernInvertedCursorGuardian.exe`, ~270 KB):** `#![windows_subsystem = "windows"]` background daemon living in **Show hidden icons** (`Shell_NotifyIconW`) with **0.00% idle CPU**, **~1.4 MB private RAM**, and **zero subprocess spawning** (`advapi32.dll` direct Registry calls instead of `reg.exe`).

---

## Quick Install (Any New Windows 10/11 PC — x64 & ARM64)

1. Download the installer for your PC from **[GitHub Releases](https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11/releases/latest)**:
   - **x64 (Intel / AMD PCs):** `ModernInvertedSetup-v1.0.1-x64.exe`
   - **ARM64 (Snapdragon / Windows on ARM PCs):** `ModernInvertedSetup-v1.0.1-arm64.exe` *(built & tested natively on GitHub's hosted `windows-11-arm` runner)*
2. Double-click the downloaded `.exe` (no admin rights or terminal required).
3. Choose a preset (**Compact** recommended), keep the startup & Installed Apps toggles enabled, and click **Quick Install**.
4. Your cursors switch immediately and the guardian settles into **Show hidden icons (`^`)** on the taskbar.

### System Tray Controls

Left-click or right-click the split black/white arrow icon in **Show hidden icons**:

- **Status line:** Shows active preset, balanced pixel size, and Accessibility slider step (e.g., `Compact (36px, 2/15)`).
- **Reapply Cursors Now** *(or double-click the tray icon)* — immediately re-enforces all 17 system cursor handles.
- **Pause Guardian** — temporarily suspends auto-reapply (turns the tray icon badge amber) if you want to test other cursors.
- **Preset** — switch live between `Compact`, `No Tail`, `Tail Detached`, and `Regular`.
- **Start at Windows Sign-in** — toggle `HKCU\...\Run` startup registration.
- **Restore Original Cursors...** — restores your saved `backup-original.reg` and exits cleanly.

---

## Measured Comparison: Legacy V5.1 vs V6 / v1.0.0

All measurements observed directly on Windows 11 (`x86_64-pc-windows-msvc`, 125% DPI):

| Metric | Legacy V5.1 (`legacy-v5.1` branch) | V6 Guardian + v1.0.0 Setup (`main`) | Improvement |
| :--- | :--- | :--- | :--- |
| Guardian EXE size | `5,161,090` bytes (~4.92 MB) | **`276,480` bytes (~270 KB)** | **18.7× smaller** |
| Startup window | Full Windows Terminal window | **None** (`PE Subsystem = 2` GUI) | **100% silent** |
| Registry apply (20 keys) | `795.13 ms` (20× `reg.exe`) | **`2.92 ms`** (native `advapi32.dll`) | **271.9× faster** |
| Idle CPU usage | `reg.exe` spikes on boot/change | **0.00% idle** (`0.078s` total / 30+ min) | **Kernel event wait** |
| Guardian RAM | `7.53 MB` Working Set | **`~0.5–2.3 MB` Working Set / `~1.37 MB` Private** | **~3–15× lighter** |
| Log disk behavior | New `installer-*.log` every boot | **Single 32 KB rotating `guardian.log`** | **Zero log clutter** |

See [docs/TESTS_AND_BENCHMARKS.md](docs/TESTS_AND_BENCHMARKS.md) for full benchmark tables and test logs.

---

## Balanced Accessibility Sizing Curve

The Windows 11 Accessibility pointer-size slider (`1..15`) normally scales `CursorBaseSize` from `32 px` to `256 px` (`+16 px` per step). Because XOR cursors use crisp 1-bit/low-color bitmaps, V6 maps those exact 15 positions cleanly to `32..88 px` in `4 px` increments:

```text
Step  1 -> 32 px     Step  6 -> 52 px     Step 11 -> 72 px
Step  2 -> 36 px     Step  7 -> 56 px     Step 12 -> 76 px
Step  3 -> 40 px     Step  8 -> 60 px     Step 13 -> 80 px
Step  4 -> 44 px     Step  9 -> 64 px     Step 14 -> 84 px
Step  5 -> 48 px     Step 10 -> 68 px     Step 15 -> 88 px
```

---

## CLI & Automation Flags

Both executables support unattended CLI options without opening any window:

```powershell
# Headless unattended install on a new PC
.\ModernInvertedSetup-v1.0.0.exe --silent --preset compact

# Restore original Windows cursors and remove startup/uninstall keys
.\ModernInvertedSetup-v1.0.0.exe --restore

# Isolated non-destructive sandbox test (no cursor or HKCU changes)
.\ModernInvertedSetup-v1.0.0.exe --sandbox C:\Temp\cursor-test --preset compact

# Render dark-mode UI smoke captures at current DPI
.\ModernInvertedSetup-v1.0.0.exe --smoke-ui C:\Temp\ui-smoke

# Control the running tray guardian directly
"%LOCALAPPDATA%\ModernInvertedCursors\ModernInvertedCursorGuardian.exe" --tray
"%LOCALAPPDATA%\ModernInvertedCursors\ModernInvertedCursorGuardian.exe" --quit
"%LOCALAPPDATA%\ModernInvertedCursors\ModernInvertedCursorGuardian.exe" --uninstall
```

---

## Repository Structure & Branches

- **`main`** — Current pure-Rust `v1.0.x` Cargo workspace, Dark-Mode Setup GUI, V6 Tray Guardian, docs, benchmarks, and multi-arch CI workflow (`x64` + `ARM64`).
- **`legacy-v5.1`** — Complete preserved Git history of earlier V1–V5.1 PowerShell and console-Rust iterations prior to the V6 rewrite.

```text
.
├── Cargo.toml                  # Zero external crates ([dependencies] empty)
├── build.rs                    # Resolves target-matched guardian binary (x64 or arm64)
├── build.ps1                   # Build guardian -> setup -> cargo test -> SHA256SUMS
├── src/
│   ├── lib.rs                  # Shared Win32 registry, SHA-1, cursor engine & SDF vector glyph renderer
│   └── bin/
│       ├── guardian.rs         # #![windows_subsystem = "windows"] tray daemon
│       └── setup.rs            # Dark-Mode Win32 GUI installer (embeds target guardian EXE)
├── dist/
│   ├── ModernInvertedSetup-v1.0.1-x64.exe
│   ├── ModernInvertedCursorGuardian.exe
│   └── SHA256SUMS.txt          # Note: ARM64 binaries are built & published on GitHub Releases via windows-11-arm
├── docs/
│   ├── ARCHITECTURE.md
│   ├── TESTS_AND_BENCHMARKS.md
│   ├── POST_USAGE_NOTES.md
│   ├── DEBUGGING_HISTORY.md
│   └── images/
└── archive/                    # Historical V1..V5.1 sources
```

---

## Build from Source & Contributing (FOSS)

Requires Rust (`rustc` / `cargo` on Windows x64 or Windows ARM64; zero external crates needed):

```powershell
git clone https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11.git
cd Modern-Inverted-Cursor-Windows11
.\build.ps1
```

Everything in this project is **100% Free and Open Source Software (FOSS)**. Feel free to fork, tweak, study, or build your own custom version. Found a bug or want to suggest an improvement? Open an [Issue](https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11/issues) or submit a Pull Request!

## Upstream Attribution & License

- Cursor artwork assets are downloaded at runtime from **[emvaized/modern_inverted_mouse_cursors v3.6.2](https://github.com/emvaized/modern_inverted_mouse_cursors)** (MIT) and verified individually by expected byte size and Git blob SHA-1. See [UPSTREAM.md](UPSTREAM.md).
- Installer & guardian source code is licensed under the [MIT License](LICENSE).
