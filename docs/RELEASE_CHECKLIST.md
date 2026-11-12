# Release Candidate Checklist (`v1.1.0-rc.1`)

All 15 mandatory acceptance gates prior to user sign-off for stable release:

- [ ] **Gate 1:** No known critical or high-severity unresolved defects (`BUG-01`..`BUG-06` resolved).
- [ ] **Gate 2:** Installer (`ModernInvertedCursors-Setup`) and Guardian (`ModernInvertedCursors-Guardian`) cleanly separated and named consistently across `x64`, `arm64`, `win7-win10-x64`, `win7-win10-x86`.
- [ ] **Gate 3:** Setup installs and maintains both persistent Setup/Maintenance executable and Guardian executable cleanly.
- [ ] **Gate 4:** Safe, non-destructive upgrade migration from `v1.0.0`, `v1.0.1`, and `v1.0.2` (preserving valid `backup-original.reg`).
- [ ] **Gate 5:** Failure-safe transactional restore/uninstall preserving config if backup is missing or corrupt.
- [ ] **Gate 6:** Bounded startup readiness handshake + rollback on failure; `--silent` exits non-zero cleanly with zero blocking `MessageBoxW`.
- [ ] **Gate 7:** Zero indiscriminate `taskkill /IM`; PID + executable path verified shutdown only.
- [ ] **Gate 8:** Unit, integration, backup/restore, IPC failure-injection, and endurance tests passing cleanly (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`).
- [ ] **Gate 9:** Genuine Windows CI green across Modern `x64`, Native `ARM64`, Legacy `win7-win10-x64`, and Legacy `win7-win10-x86` (including PE + import audits).
- [ ] **Gate 10:** Real GUI interactive flows + visual inspection at `100%`, `125%`, `150%`, and `200%` DPI scaling verified.
- [ ] **Gate 11:** Multi-trial before/after resource & latency benchmarks recorded with no unexplained regressions.
- [ ] **Gate 12:** Security findings resolved or accurately documented and accepted (`SEC-01`..`SEC-08`).
- [ ] **Gate 13:** Consistently named artifacts with verified `SHA256SUMS.txt`.
- [ ] **Gate 14:** User docs (`README.md`, post-usage, CLI/upgrade instructions) match exact code & CI facts.
- [ ] **Gate 15:** Persistent audit/decision/evidence docs updated, Draft PR & Release Candidate prepared without merging to `main` or publishing final release.
