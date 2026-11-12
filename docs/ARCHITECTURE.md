# Architecture & Engineering Design

This document details the Win32 internals, memory safety guarantees, and Windows 11 integration choices behind `ModernInvertedSetup` (`src/bin/setup.rs`), `ModernInvertedCursorGuardian` (`src/bin/guardian.rs`), and the shared core (`src/lib.rs`).

---

## 1. Zero-Dependency Pure Rust

- `Cargo.toml` has **zero third-party dependencies** (`[dependencies]` is empty).
- Only standard core Windows system DLLs are linked: `kernel32`, `user32`, `advapi32`, `gdi32`, `shell32`.
- Optional Vista/Windows 10/11 enhancements (`wtsapi32.dll` session notifications, `dwmapi.dll` dark mode + rounded corners, `GetDpiForWindow`, `SetProcessDpiAwarenessContext`, `AdjustWindowRectExForDpi`) are loaded dynamically via `LoadLibraryW` + `GetProcAddress` so compilation succeeds cleanly on both `x86_64-pc-windows-msvc` and `x86_64-pc-windows-gnu` toolchains without requiring extra `.a` import libraries.

---

## 2. Guardian Event Loop & Idle Footprint (`src/bin/guardian.rs`)

### Why `#![windows_subsystem = "windows"]` Matters
Legacy console guardians cause Windows Explorer (`HKCU\...\Run`) to spawn `conhost.exe` / Windows Terminal at sign-in because their PE `IMAGE_OPTIONAL_HEADER.Subsystem` is `3` (`IMAGE_SUBSYSTEM_WINDOWS_CUI`). Compiling both binaries with `#![windows_subsystem = "windows"]` sets `Subsystem = 2` (`IMAGE_SUBSYSTEM_WINDOWS_GUI`), eliminating terminal creation entirely.

### Hidden Top-Level Broadcast Window vs `HWND_MESSAGE`
The guardian creates an invisible `0×0` `WS_POPUP` window (`WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`) rather than `HWND_MESSAGE`. Message-only (`HWND_MESSAGE`) windows do **not** receive top-level shell/system broadcasts such as:

- **`TaskbarCreated`** (`RegisterWindowMessageW("TaskbarCreated")`) — automatically re-adds the tray icon if `explorer.exe` restarts.
- **`WM_WTSSESSION_CHANGE` (`WTS_SESSION_UNLOCK`)** — re-applies cursors after lock/unlock transitions.
- **`WM_DISPLAYCHANGE`** — re-applies cursors after monitor/DPI topology changes.
- **`WM_POWERBROADCAST` (`PBT_APMRESUMEAUTOMATIC` / `PBT_APMRESUMESUSPEND`)** — re-applies after sleep/hibernate.

If `Shell_NotifyIconW(NIM_ADD)` fails during early sign-in before Explorer's tray initializes, a 2-second fallback timer (`TIMER_TRAY_RETRY`, up to 15 attempts) ensures the icon always appears inside **Show hidden icons**.

### Single-Threaded Kernel Sleep + Non-Blocking Debounce
Instead of polling or sleeping via `thread::sleep(500)` (which would freeze the tray popup menu), the main thread runs:

1. `RegNotifyChangeKeyValue(key, 0, REG_NOTIFY_CHANGE_LAST_SET, reg_event, 1)` (asynchronous kernel event).
2. `MsgWaitForMultipleObjectsEx(1, &reg_event, INFINITE, QS_ALLINPUT_COMPAT, MWMO_INPUTAVAILABLE)`.
3. When `reg_event` signals, it re-arms notifications and schedules a non-blocking `SetTimer(hwnd, TIMER_REAPPLY, 400, None)`. Rapid burst writes by Windows `SystemSettings.exe` coalesce into a single reapply while keeping tray menus responsive.
4. After startup and each apply, `SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX)` trims transient startup pages from the working set.

### Sound Pointer Handling Across Reentrant Win32 Modal Loops
Windows `TrackPopupMenuEx` and `MessageBoxW` run nested modal message loops that can re-enter `wnd_proc` (e.g., if `WM_TIMER` fires while the tray menu is open). To prevent Stacked Borrows `&mut AppState` aliasing UB:

- `AppState` is heap-allocated via `Box::into_raw` and stored in `GWLP_USERDATA` (no live `Box` or `&mut AppState` exists on the `run()` stack).
- `WM_APP_SHOW_MENU` copies an immutable value snapshot `(preset, paused, last_px, last_step)`, drops the `&mut AppState` borrow **before** entering `TrackPopupMenuEx`, and handles `CMD_RESTORE`'s `MessageBoxW` confirmation **before** borrowing `AppState` again.

---

## 3. Cursor Engine & Integrity (`src/lib.rs`)

### Registry Persistence + Direct `SetSystemCursor`
1. Writes all 17 scheme roles (`Arrow` .. `Person`) to `HKCU\Control Panel\Cursors\Schemes` and `HKCU\Control Panel\Cursors` using direct `advapi32.dll` calls (`RegCreateKeyExW`, `RegSetValueExW`) — eliminating 20+ `reg.exe` spawns per apply.
2. Invokes `SystemParametersInfoW(SPI_SETCURSORS, ...)` for baseline compatibility.
3. Explicitly loads each `.cur` / `.ani` file via `LoadImageW(..., IMAGE_CURSOR, px, px, LR_LOADFROMFILE)` at the balanced size (`32 + (step - 1) * 4`) and replaces the live session handles via `SetSystemCursor(handle, id)`. If `SetSystemCursor` fails, `DestroyCursor(handle)` prevents USER handle leaks.

### Upstream Verification & Parallel Setup Download
- All 32 cursor files from `emvaized/modern_inverted_mouse_cursors` (`v3.6.2`) are pinned by exact byte size and Git blob SHA-1 (`SHA1("blob " + len + "\0" + bytes)`), implemented in pure Rust (`sha1()` / `git_blob_sha1()`).
- `prepare_assets_parallel` spawns up to 4 worker threads inside `std::thread::scope` using Windows' built-in `curl.exe` (`CREATE_NO_WINDOW`) when files are missing, streaming live `[done/32]` updates to the GUI progress bar.

---

## 4. Dark-Mode GUI Installer (`src/bin/setup.rs`)

- **Native Dark Window Chrome:** Uses `DwmSetWindowAttribute` for immersive dark title bar (`20`), rounded corners (`33 = DWMWCP_ROUND`), and subtle `#2B313D` border tint (`34`).
- **Per-Monitor DPI V2:** Calls `SetProcessDpiAwarenessContext(-4)` and scales every layout rectangle, corner radius, glyph size, and `Segoe UI` ClearType font dynamically via `GetDpiForWindow` and `WM_DPICHANGED`.
- **Anti-Aliased Custom Vector & SDF Rendering:** Renders rounded cards, borders, toggles, progress bar, step checkmarks, and 4 distinct cursor preset previews via 4×4 supersampling and signed-distance-field (`sdf_round_box`) coverage into a 32-bit `CreateDIBSection` backbuffer before compositing GDI ClearType text and blitting (`BitBlt`) with zero flicker (`WM_ERASEBKGND => 1`).
- **Clean Windows Integration:** Registers `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\ModernInvertedCursors` so users can repair/uninstall cleanly from **Windows Settings → Apps → Installed apps** without needing administrator privileges.
