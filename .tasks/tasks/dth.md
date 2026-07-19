TT;DR: Fix the Alienware contradictions where the overview reports four driver problems that the Drivers page does not show, and where all thermals are described as unsupported despite usable GPU or vendor telemetry.

## Why
Direct operator report from the physical Alienware. Windows SetupAPI and `pnputil` report no device problem while four Intel telemetry extension devices expose a generic WMI `Degraded` string. The Drivers page also omits categories counted by the overview. Public ACPI thermal telemetry is unavailable on this model even though Dell's vendor monitor can access additional sensors.

## Plan
Centralize driver attention semantics, give authoritative PnP problem codes precedence over generic WMI status text, and render every attention item in User Mode. Add a thermal provider ladder that preserves existing platform sensors, consumes supported Windows hardware-monitor WMI bridges when present, merges the already-proven GPU temperature provider, and reports vendor or privilege limits explicitly.

## Impact
The overview, Drivers page, snapshot warnings, and capabilities must agree. Missing public CPU telemetry must never become a fabricated temperature, and vendor control interfaces must not be invoked as read APIs.

## Acceptance
The physical Alienware reports zero driver issues when SetupAPI and ConfigManager agree there is no problem; synthetic real problems still surface everywhere. GPU temperature remains visible even when ACPI CPU telemetry is absent. Supported hardware-monitor bridges are classified and attributed. CPU vendor-only or permission-gated telemetry is described accurately.

## Verification
- [x] Windows PnP precedence and duplicate-row fixtures pass
- [x] User and Technician driver views use the shared attention set
- [x] Thermal provider and classification fixtures pass
- [x] Redacted Alienware snapshot matches independent Windows evidence
- [ ] All Rust, package, and release qualification gates pass

## Status
Active. The v2.0.4 diagnostics release and post-public installed update proof pass. Physical Windows testing then exposed and reproduced a same-file collision in deliberate managed reinstalls; the live-image fix passes the exact Alienware managed-path reproduction. The v2.0.5 candidate remained an unpublished draft after its qualification harness mishandled an expected missing registry property; v2.0.6 is the immutable fix-forward candidate. Hosted four-channel takeover qualification, publication, and final installed lifecycle proof remain. Elevated Dell firmware reads could not be launched under the local execution policy.

## Activity
- 2026-07-19 - Reproduced four Intel PMT/IPF false positives: WMI reports generic `Degraded`, but ConfigManagerErrorCode is 0, SetupAPI reports no problem, and `pnputil /enum-devices /problem /connected` finds none. Confirmed User Mode omits the System and Other categories counted by Overview.
- 2026-07-19 - Confirmed public ACPI thermal WMI is unsupported on this Alienware, while NVIDIA telemetry is already available to SD-300 and Dell AWCC exposes additional vendor telemetry through privileged/internal paths.
- 2026-07-19 - Implemented authoritative Config Manager precedence and a shared attention iterator. A live redacted snapshot now reports zero driver attention items and no driver warning, matching `pnputil` and SetupAPI.
- 2026-07-19 - Added independent CPU/GPU thermal observations, NVIDIA temperature merging, Libre/Open Hardware Monitor WMI bridge support, and guarded read-only Dell AWCC firmware enumeration based on the documented `Thermal_Information` byte protocol. The normal-user snapshot reports GPU 53 C as available and Dell CPU/fan access as permission denied.
- 2026-07-19 - Passed 52 Rust tests, strict clippy, 80x24 User/Technician rendering, and checks for Windows x64/ARM64, Linux x64/ARM64, and macOS Intel/Apple Silicon. Local policy blocked an attempted elevated snapshot before UAC, so successful AWCC firmware values are not yet claimed.
- 2026-07-19 - Tightened SetupAPI/WMI reconciliation from display-name joins to exact PnP device instance IDs, then passed final format, clippy, 52-test, optimized-build, publish-dry-run, cargo-dist-plan, and five cross-target checks. The v2.0.4 release binary matched `pnputil`: zero driver problems, GPU 52 C available, and Dell CPU/fan reads explicitly permission-gated.
- 2026-07-19 - Published v2.0.4 after Windows, Intel/Apple Silicon PKG, Linux, crate, checksum, and public-lifecycle gates passed. The installed 2.0.3 managed PowerShell copy updated to 2.0.4 in place, retained one PATH command/receipt, returned a successful no-op update, and produced the same zero-problem driver/GPU-available thermal snapshot.
- 2026-07-19 - Reproduced a separate `sd300 install --json` defect: the managed wrapper could not replace or roll back the still-running Windows executable. Added rollback-capable live-image handoff for managed/Cargo/Corporate installs plus an ownership-validated elevated Global takeover worker. The exact managed Alienware path now returns one successful JSON object, preserves the managed channel, verifies 2.0.4, and leaves no backup residue.
- 2026-07-19 - Expanded the ephemeral Windows qualification matrix so candidate Global/Corporate MSI and EXE installs must each transfer to the candidate managed PowerShell owner through the CLI, remove native registration/marker/PATH/payload state, verify the managed receipt and version, and uninstall cleanly.
- 2026-07-19 - Ran the v2.0.5 candidate from the exact managed Alienware path and deliberately installed public latest v2.0.4 over it. The command returned exactly one success JSON line, changed 2.0.5 to 2.0.4 with a matching receipt, and left zero live-image backups, proving an older fresh-install target still wins as the user's latest explicit intent.
- 2026-07-19 - Base v2.0.5 release assembly, CI, and Intel/Apple Silicon PKG qualification passed. Windows reached the new native-to-managed matrix and the first takeover returned success, but the harness treated the correctly removed `InstallSource` property as a terminating error. Kept v2.0.5 unpublished and immutable, corrected both registry probes, and bumped the fix-forward candidate to v2.0.6.
