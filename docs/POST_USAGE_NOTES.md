**Updated for `v1.1.0-rc.1` (Hardened Release Candidate):**
- **Installed Binaries:** `%LOCALAPPDATA%\ModernInvertedCursors\ModernInvertedCursors-Setup.exe` (maintenance app registered in Windows Installed Apps with `ModifyPath`, `NoModify=0`, `NoRepair=0`) and `%LOCALAPPDATA%\ModernInvertedCursors\ModernInvertedCursors-Guardian.exe` (silent system-tray daemon).
- **Live Upgrade Verification Observed:** Upgrading over the pre-existing `v1.0.0` `ModernInvertedCursorGuardian.exe` cleanly preserved `backup-original.reg` (`4,954` bytes, `2026-10-03`), stopped the previous PID cleanly without `taskkill /IM`, deployed the `v1.1.0-rc.1` Setup + Guardian pair, removed the superseded legacy executable, and applied `Compact (36 px)`.

---

# Legacy `v1.0.x` Post-Usage Notes & Live System Verification

**Recorded:** 2026-11-12  
**Target Environment:** Windows 11 (x64, 125% DPI / `dpi=120`)  
**Active Installed Guardian:** `C:\Users\sinet\AppData\Local\ModernInvertedCursors\ModernInvertedCursorGuardian.exe` (`268,288` bytes, V6 pure-Win32 tray build)  
**Public Release Tested Alongside:** `ModernInvertedSetup-v1.0.0.exe` (`604,672` bytes, verified via `--smoke-ui` and isolated `--sandbox` without disturbing the live user setup)

---

## 1. Live System State Verified After Completion

On the target workstation, the V6 silent tray guardian was verified active and healthy with the original pre-install cursor backup preserved:

- **Running process:** `ModernInvertedCursorGuardian` (`PID 12984`), running since `10:18:25 AM` with `0.0781250s` total CPU time (`0.00%` idle CPU), `1,433,600` bytes (`~1.37 MB`) private commit memory, and blank `MainWindowTitle` (no taskbar button or terminal window).
- **Windows startup integration (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`):**  
  `ModernInvertedCursorGuardian = "C:\Users\sinet\AppData\Local\ModernInvertedCursors\ModernInvertedCursorGuardian.exe" --tray`
- **Active cursor scheme (`HKCU\Control Panel\Cursors`):**  
  `Modern Inverted v3.6.2 - Compact` (`arrow_no_tail_smaller_v2.cur`, `help_no_tail.cur`, `working_tail_detached_v3.ani`, `busy.ani`, etc.) at **`36 px`** (`CursorSize = 2/15`, `CursorType = 5`).
- **Original backup intact:** `C:\Users\sinet\AppData\Local\ModernInvertedCursors\backup-original.reg` (`4,954` bytes, timestamp `2026-10-03`) left completely untouched.

---

## 2. Real-World Usage Observations

1. **Where the icon lives on Windows 11:**  
   By default on Windows 11, newly registered `Shell_NotifyIconW` icons appear inside the **Show hidden icons (`^`)** overflow flyout near the clock. Drag the split black/white arrow icon onto the taskbar if you prefer it permanently pinned, or leave it neatly tucked inside **Show hidden icons**.
2. **Changing pointer size via Windows Settings:**  
   Dragging **Settings → Accessibility → Mouse pointer and touch → Size** still works normally — Windows briefly displays its own stock cursor during the write burst, and ~400 ms after release the guardian automatically reapplies the chosen Modern Inverted pack at the balanced `32..88 px` size (`+4 px` per step).
3. **Temporarily testing other Windows cursors:**  
   Use **Pause Guardian** in the tray menu before changing pointer styles in Windows Settings; otherwise the guardian treats any write to `HKCU\Software\Microsoft\Accessibility` as a cursor reset and restores Modern Inverted.
4. **Upgrading from V5.1:**  
   Installing V6 cleanly stops V5.1, eliminates the Windows Terminal popup at login, and stops creating new `installer-*.log` files on every boot (switching to a single 32 KB rolling `guardian.log`). Any historical `installer-*.log` files from earlier V5.1 boots in `%LOCALAPPDATA%\ModernInvertedCursors` can be deleted manually at any time if desired; never delete `backup-original.reg` while using the pack.
5. **Restoring defaults completely:**  
   Either right-click the tray icon → **Restore Original Cursors...**, select **Modern Inverted Mouse Cursors → Uninstall** inside Windows **Settings → Apps → Installed apps** (when installed via `ModernInvertedSetup-v1.0.0.exe`), or run `--restore`.

---

## 3. Post-Release GitHub & System Confirmation

- **Initial x64 Release:** [v1.0.0](https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11/releases/tag/v1.0.0) (`ModernInvertedSetup-v1.0.0.exe` `604,672` bytes, `ModernInvertedCursorGuardian.exe` `276,480` bytes, `SHA256SUMS.txt`).
- **Second Multi-Arch Release (First Release Series `v1.0.x`):** [v1.0.1](https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11/releases/tag/v1.0.1) adds native **Windows 11 ARM64 (`aarch64-pc-windows-msvc`)** builds (`ModernInvertedSetup-v1.0.1-arm64.exe` `560,640` bytes, `ModernInvertedCursorGuardian-arm64.exe` `257,024` bytes) alongside **Windows x64** (`ModernInvertedSetup-v1.0.1-x64.exe` `604,672` bytes, `ModernInvertedCursorGuardian-x64.exe` `276,480` bytes).
- **GitHub Actions Windows CI:**
  - `v1.0.0` (`#43160826875`, `50s`) passed on `windows-latest`.
  - `v1.0.1` (`#43162511198`) passed across **all 3 jobs**: `Build & Test (x64)` on `windows-latest` (`58s`), `Build & Test (arm64)` on `windows-11-arm` (`55s`, including `SMOKE_UI_OK dpi=96 dwm_dark=true size=620x540` and `0xAA64` PE verification), and `Publish Multi-Arch GitHub Release` (`5s`).
- **Post-Release Local PC Check:** Verified the installed V6 Tray Guardian (`PID 12984`, `268,288` bytes, `Compact` 36 px) remained running continuously throughout all builds, sandbox tests, and both releases with `0.078s` total CPU and `backup-original.reg` preserved untouched.


---

## `v1.0.2` Legacy Windows 7 SP1 / 8 / 8.1 / 10 Notes

1. **Both Legacy 64-bit (`win7-win10-x64`) & 32-bit (`win7-win10-x86`) Verified Locally & on CI:**
   - Built cleanly on **Rust `1.75.0`** (the last Tier-1 MSVC toolchain supporting Windows 7 SP1 / 8 / 8.1 out of the box) with `-C target-feature=+crt-static`.
   - Passed all 9 unit tests across `lib`, `guardian`, and `setup`, including 32-bit Win32 struct sizes (`NotifyIconDataW=956`, `Msg=28`, `TrackMouseEvent=16`, `PaintStruct=64`) and PE import table audits confirming zero `VCRUNTIME140.dll` or Windows 8/10-only synchronization DLL imports.
2. **WinHTTP TLS 1.2 + Vista–Win11 Fallbacks Confirmed Live:**
   - Native `winhttp.dll` download (trying `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY` then `WINHTTP_ACCESS_TYPE_DEFAULT_PROXY` for Win7/8) downloaded and SHA-1 verified all 32 cursors cleanly in `1283 ms` (`win7-win10-x64`) and `4561 ms` (`win7-win10-x86`) during non-destructive `--sandbox` runs, without touching the live installed Windows 11 cursor guardian.
   - `ModernInvertedSetup-v1.0.2-win7-win10-x86.exe` includes an explicit `asInvoker` manifest so 32-bit Windows 7/8/10 machines launch setup directly without Installer Detection UAC prompts.
