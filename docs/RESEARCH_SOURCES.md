# Research Sources & Technical Verification Log

All sources below were accessed and verified via web search on **2026-11-12**.

---

## 1. WinHTTP Proxy, Timeouts & Windows 7 TLS 1.2 Behavior
- **Research Question:** How should `WinHttpOpen`, proxy selection, transaction timeouts, and TLS 1.2 behave across Windows 7 SP1 through Windows 11?
- **Sources:**
  - Microsoft Learn — *WinHttpOpen function (`winhttp.h`)* (`https://learn.microsoft.com/en-us/windows/win32/api/winhttp/nf-winhttp-winhttpopen`)
  - Microsoft Learn — *WinHttpSetTimeouts function (`winhttp.h`)* (`https://learn.microsoft.com/en-us/windows/win32/api/winhttp/nf-winhttp-winhttpsettimeouts`)
  - Microsoft Support / Learn — *Update to enable TLS 1.1 and TLS 1.2 as default secure protocols in WinHTTP in Windows (`KB3140245`)* (`https://support.microsoft.com/en-us/topic/update-to-enable-tls-1-1-and-tls-1-2-as-default-secure-protocols-in-winhttp-in-windows-c4bd73d2-31d7-761e-0178-11268bb10392`)
- **Accessed:** 2026-11-12
- **Key Findings:**
  1. `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY (4)` requires Windows 8.1+; on Windows 7/8 `WinHttpOpen` must fall back to `WINHTTP_ACCESS_TYPE_DEFAULT_PROXY (0)`.
  2. Default `WinHttpSetTimeouts` leaves send/receive timeouts at 30s and connect at 60s, while DNS resolve (`nResolveTimeout`) defaults to `0` (infinite!). Calling `WinHttpSetTimeouts(h, 10_000, 10_000, 15_000, 20_000)` prevents indefinite DNS/socket hangs.
  3. On Windows 7 SP1, TLS 1.2 in WinHTTP requires Microsoft Update **KB3140245** (plus `DefaultSecureProtocols` or explicit `WINHTTP_OPTION_SECURE_PROTOCOLS`). Unpatched Windows 7 SP1 cannot negotiate TLS 1.2 with GitHub (`raw.githubusercontent.com` dropped TLS 1.0/1.1) — an OS-level requirement that cannot be bypassed cleanly without bundling a custom TLS stack, and must be documented honestly.
- **Implementation Impact:** Reused a single WinHTTP session/connect per worker thread (cutting 32× proxy/TLS handshakes cleanly), added explicit `WinHttpSetTimeouts`, bounded response sizes (`MAX_ASSET_BYTES`), guaranteed `FreeLibrary` / `WinHttpCloseHandle` RAII cleanup, and documented Win7 KB3140245 requirement.

## 2. Cursor Handle Ownership, `SetSystemCursor` & `RegNotifyChangeKeyValue`
- **Research Question:** Who owns `HCURSOR` handles passed to `SetSystemCursor`, when is `DestroyCursor` unsafe, and how does `RegNotifyChangeKeyValue` interact with `SPI_SETCURSORS`?
- **Sources:**
  - Microsoft Learn — *SetSystemCursor function (`winuser.h`)* (`https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setsystemcursor`)
  - Microsoft Learn — *DestroyCursor function (`winuser.h`)* (`https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-destroycursor`)
  - Microsoft Learn — *SystemParametersInfoW (`SPI_SETCURSORS`)* (`https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-systemparametersinfow`)
  - Microsoft Learn — *RegNotifyChangeKeyValue function (`winreg.h`)* (`https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regnotifychangekeyvalue`)
- **Accessed:** 2026-11-12
- **Key Findings:**
  1. When `SetSystemCursor(hcur, id)` succeeds, **the system automatically destroys `hcur`**; the caller must only call `DestroyCursor` if `SetSystemCursor` **fails** (or for shared handles never use `DestroyCursor`, which is why we load with `LR_LOADFROMFILE` without `LR_SHARED`).
  2. `RegNotifyChangeKeyValue(..., asynchronous=TRUE)` fires at most **once** per call and must be re-armed after each signal while the registry `HKEY` stays open on the owning thread.
- **Implementation Impact:** Verified pre-loading all cursor handles up front before calling `SetSystemCursor` guarantees every `.cur`/`.ani` file loads at the target pixel size before any live handle is overwritten, eliminating half-applied cursor states; failed handles are explicitly `DestroyCursor`-cleaned.

## 3. Per-User `HKCU\...\Uninstall`, UAC Installer Detection & `asInvoker` Manifest
- **Research Question:** What registry keys power **Apps / Programs & Features** Modify + Uninstall without elevation, and how does Windows treat 32-bit `Setup` binaries?
- **Sources:**
  - Microsoft Learn — *Uninstalling Applications (`HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall`)* (`https://learn.microsoft.com/en-us/windows/win32/msi/uninstall-registry-key`)
  - Microsoft Learn — *User Account Control Overview & Installer Detection Technology* (`https://learn.microsoft.com/en-us/windows/security/application-security/application-control/user-account-control/how-it-works`)
  - Microsoft Learn — *Application Manifests (`requestedExecutionLevel level="asInvoker"`)* (`https://learn.microsoft.com/en-us/windows/win32/sbscs/application-manifests`)
- **Accessed:** 2026-11-12
- **Key Findings:**
  1. Windows UAC Installer Detection flags unmanifested 32-bit executables containing `"setup"`/`"install"`/`"update"` and fails un-elevated launches with `ERROR_ELEVATION_REQUIRED (740)`. Declaring `level="asInvoker"` in the embedded XML manifest disables Installer Detection cleanly without requesting Admin privileges.
  2. Setting `ModifyPath` alongside `UninstallString`, `QuietUninstallString`, and `NoModify = 0` allows users to launch the Setup/Maintenance GUI directly from Windows Settings -> Installed Apps -> **Modify**.

## 4. Rust Windows 7 Support Tier History
- **Research Question:** Why do Rust `1.78+` binaries fail on Windows 7/8 and what is the official support status of `1.75.0` vs `win7-windows-msvc`?
- **Sources:**
  - Rust Blog — *Updated baseline standards for Windows targets (Rust 1.78)* (`https://blog.rust-lang.org/2024/02/26/Windows-7-support.html` / `https://doc.rust-lang.org/nightly/rustc/platform-support/windows-7.html`)
- **Accessed:** 2026-11-12
- **Key Findings:**
  - Rust `1.78` raised Tier-1 `*-pc-windows-msvc` minimum OS to Windows 10 (introducing `WaitOnAddress` / `api-ms-win-core-synch-l1-2-0.dll` imports). Dedicated `*-win7-windows-msvc` targets are **Tier 3** (not shipped prebuilt in `rustup`), whereas Rust **`1.75.0`** ships full precompiled Tier-1 `std` targeting Windows 7 SP1+ cleanly.
- **Uncertainty / Limitation:** `1.75.0` receives no new compiler security updates, and GitHub hosts Windows 11/Server 2022 runners rather than Windows 7/8 VMs — runtime testing on actual Win7/Win8 hardware remains out-of-environment unless tested externally.

## 5. OS End-of-Support Lifecycle & Authenticode SmartScreen Reputation
- **Research Question:** What are official OS support dates and SmartScreen implications for unsigned FOSS releases?
- **Sources:**
  - Microsoft Lifecycle Policy (`https://learn.microsoft.com/en-us/lifecycle/products/windows-7`, `windows-81`, `windows-10-home-and-pro`)
  - Microsoft Learn — *Microsoft Defender SmartScreen & Code Signing Overview* (`https://learn.microsoft.com/en-us/windows/security/operating-system-security/virus-and-threat-protection/microsoft-defender-smartscreen/`)
- **Accessed:** 2026-11-12
- **Key Findings:**
  - **EOL dates:** Windows 7 SP1 ended **2020-01-14** (ESU **2023-01-10**); Windows 8.1 ended **2023-01-10**; Windows 10 22H2 mainstream ended **2025-10-14** (ESU optional).
  - **Code signing:** Unsigned (or self-signed) EXEs have zero SmartScreen cross-version reputation transfer and trigger *"Windows protected your PC"* prompts (or Smart App Control blocks where enforced) on fresh downloads; without an OV/EV/Trusted Signing certificate, users rely on SHA-256 checksums (`SHA256SUMS.txt`) and open-source reproducible builds.
