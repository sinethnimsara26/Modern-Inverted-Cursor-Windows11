# Post-Usage Notes & Live System Verification

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
