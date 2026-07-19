TT;DR: Release v2.0.0 only after Windows live evidence, hosted native platform gates, installer matrices, and fresh public artifact verification are all complete.

## Why
The operator selected one combined major release. Local Windows success alone cannot establish all-platform correctness.

## Plan
Run full Rust/package gates, hosted matrices, Apple signing/notarization, Linux targets, Windows native transitions, public-byte audits, crates verification, and final smoke tests.

## Impact
Publishes the major diagnostic and lifecycle contract. A failed gate blocks the tag rather than producing a partial claim.

## Acceptance
The exact released SHA is green everywhere, public artifacts match the manifest, and supported fresh/update/uninstall paths are proven.

## Verification
- [x] Local Windows live qualification passes
- [ ] Hosted Windows, macOS Intel/ARM, and Linux gates pass
- [ ] Crates package installs the `sd300` command
- [ ] Public release manifest and assets hash-match
- [ ] Fresh latest commands install and report v2.0.0

## Status
Active. All seven Apple certificate/notary secrets and three non-secret identity variables are configured. Hosted exact-SHA qualification and public-byte verification remain; do not publish the website until every hosted/public gate passes.

## Activity
- 2026-07-18 14:45 - created as the combined release gate.
- 2026-07-18 16:31 - local Rust, release build, Alienware snapshot, TUI render, workflow, package, cross-target, wrapper-render, and website gates pass. Signing identity/team variables are configured; awaiting the private certificate/notary secrets, then hosted matrices and fresh public-byte audit.
- 2026-07-18 17:01 - sibling-task convergence rechecked; release matrices now require real synthetic-prior Windows/PKG transitions and public Linux shell/Cargo updates, not already-current origin checks.
- 2026-07-19 06:25 - re-read the completed TR-300 v4.2.2, ND-300 v3.7.3, and Goose v1.2.6 tasks. SD matches their managed-command/native-channel contract. Validated the issued Developer ID Installer CER against its DPAPI-protected PKCS#12 private key; provisioned all seven Apple secrets without logging plaintext, including a one-time sealed transfer from TR-300 for credentials not recoverable locally. Hosted native qualification is now the active boundary.
- 2026-07-19 06:38 - repeated local release qualification after sibling comparison: 39 Rust tests, format, all-target Clippy, release build, actionlint, PowerShell parsing, Bash syntax, ShellCheck, publish dry-run (68 packaged files), dist plan, and all seven non-host Rust target checks pass. The release binary again produced a redacted schema-v1 Windows snapshot and fail-closed update; lifecycle JSON now includes sibling-parity recovery fields.
- 2026-07-19 06:48 - PR #3 exact-SHA hosted CI passed Windows 2025, macOS 15, and security audit but found one Linux test failure: Unix receipt resolution let ambient XDG_CONFIG_HOME outrank the installer-supplied user profile. Fixed the production precedence, isolated both paths in the dry-run fixture, added an explicit Unix receipt-path regression, and repeated local Rust/Linux cross-check/actionlint gates before the full hosted rerun.
- 2026-07-19 06:58 UTC - The hosted rerun passed Ubuntu, macOS, Windows, security audit, and cargo-dist plan. Codex review then found three pre-release gaps: exact SMBIOSMemoryType casing, swallowed WmiMonitorBrightness errors, and receipt-less Cargo takeover without structured Cargo ownership proof. Fixed all three with regressions (42 tests), repeated Clippy/release build, and proved the release binary on Alienware now reports both 16 GiB DIMMs as DDR5/5600 plus available display and brightness provenance. Native takeover now preserves unknown Cargo-path binaries before mutation.
