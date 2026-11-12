# Known Issues & Verified Environment Limitations

1. **No Real Windows 7 / Windows 8 / 8.1 Test Runner Available**
   - **Severity:** Informational / Verification Boundary
   - **Details:** Local development is on Windows 11 x64 (`125%` DPI), and GitHub Actions provides `windows-latest` (Windows Server 2022/2025) and `windows-11-arm`. True end-to-end runtime testing on physical Windows 7 SP1, Windows 8, or Windows 8.1 machines is NOT automated in CI.
   - **Evidence Covered Instead:** Legacy binaries compile via Rust `1.75.0` + `+crt-static` and pass automated `dumpbin /IMPORTS` checks proving zero Windows 8/10-only DLL or dynamic CRT imports (`VCRUNTIME140.dll`, `api-ms-win-crt-*`, `api-ms-win-core-synch-l1-2-0.dll`).

2. **Windows 7 SP1 TLS 1.2 Prerequisite (`KB3140245`) & OS End-of-Support**
   - **Severity:** Environment Prerequisite
   - **Details:** `raw.githubusercontent.com` requires TLS 1.2+. Windows 7 SP1 does not include TLS 1.2 in WinHTTP without Microsoft Update `KB3140245` and `DefaultSecureProtocols` configuration. Furthermore, Windows 7 (Jan 2020 / ESU Jan 2023), Windows 8.1 (Jan 2023), and Windows 10 22H2 (Oct 2025) are End of Life per Microsoft Lifecycle Policy.

3. **Unsigned Executables (SmartScreen / Smart App Control)**
   - **Severity:** Distribution Friction (Accepted FOSS Limitation)
   - **Details:** No Authenticode EV/OV/Trusted Signing certificate is configured for CI. Freshly downloaded binaries show Microsoft Defender SmartScreen prompts until hash reputation builds (and cannot bypass strict Smart App Control enforcement). Users should verify hashes against `SHA256SUMS.txt`.

4. **Upstream Asset Low-Bit-Depth Scaling Characteristics**
   - **Severity:** Upstream Design Constraint
   - **Details:** True Windows pixel-XOR inversion requires monochrome/low-color-depth cursor masks rather than 32-bit ARGB alpha-blended edges; extreme accessibility sizes (>88 px) show stair-stepping, which the balanced `32..88 px` curve mitigates.

5. **Resolved Pre-Hardening Defects (`BUG-01`..`BUG-15`):**
   - Resolved across Batches 02–07 on branch `hardening/v1.1.0-audit` (`v1.1.0-rc.1`), with unit, integration, multi-DPI, and live HWND tests verifying every fix (see [docs/BUG_AUDIT.md](BUG_AUDIT.md) and [docs/TEST_EVIDENCE.md](TEST_EVIDENCE.md)).
