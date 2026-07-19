TL;DR: Prove that `sd300 uninstall` removes the complete installation owned by every supported channel, then make the command discoverable on the SD-300 website.

## Why
The v2.0.2 command exists and public managed-shell/macOS qualification invokes it, but the Windows native transition matrix used installer-specific cleanup helpers after testing updates. The website Commands section also omits the uninstall command.

## Acceptance
- Public managed PowerShell install followed by `sd300 uninstall --json` removes the binary and managed receipt.
- A Corporate MSI install is removed on Alienware through `sd300 uninstall --json`, including payload root, native registration, install marker, and owned PATH entry; the hosted matrix proves the equivalent EXE path.
- Hosted Windows qualification invokes `sd300 uninstall --json` for Global/Corporate MSI and EXE and verifies full convergence.
- The SD-300 Commands section and install guidance list `sd300 uninstall` with accurate ownership-aware copy.
- Website lint/build and desktop/mobile production checks pass.

## Verification
- [x] Public managed PowerShell lifecycle passes on Alienware
- [x] Corporate MSI lifecycle passes on Alienware
- [x] Hosted Corporate EXE lifecycle passes
- [x] Windows hosted matrix covers all four CLI uninstall channels
- [x] Rust/workflow local gates pass
- [x] Website lint/build and local desktop/mobile rendering pass
- [x] Website production rendering passes

## Status
Done. SD-300 v2.0.3 is public after Windows MSI/EXE, macOS PKG, and Linux managed qualification; the public managed PowerShell lifecycle passes on Alienware and the uninstall command is live on the website.

## Activity
- 2026-07-19 05:00 - created after the operator noticed `sd300 uninstall` was absent from the website Commands section.
- 2026-07-19 13:20 - reproduced v2.0.2 Corporate MSI cleanup terminating the caller before JSON output, then implemented the rollback-capable live-image handoff used by the v2 updater.
- 2026-07-19 13:31 - v2.0.3 managed PowerShell removed binary/receipt/config directory without touching Cargo; a real Corporate MSI registration returned one JSON result and removed payload, ARP, marker, PATH, and stale config residue.
- 2026-07-19 13:48 - 43 tests, Clippy, release build, publish dry-run, dist plan, seven cross-target checks, PowerShell parsing, Actionlint, website lint/build, and 1440px/390px browser checks passed locally.
- 2026-07-19 14:04 - Windows native run 29699591355, macOS native run 29699591398, and final qualifier 29699832571 passed; v2.0.3 is public on GitHub and crates.io.
- 2026-07-19 14:05 - Alienware updated public v2.0.2 to v2.0.3 in place, completely removed the managed install, then restored v2.0.3 through the versionless IRM installer; Cargo, Rustup, and shared PATH state remained intact.
- 2026-07-19 14:07 - website PR 14 merged and production passed desktop/mobile checks with the uninstall command visible, stable CLI/PKG links, no console errors, and no horizontal overflow.
