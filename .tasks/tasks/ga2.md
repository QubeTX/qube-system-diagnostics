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
- 2026-07-19 07:18 UTC - PR #3 merged as 87cf1bb after every product CI gate passed. The first main release run built all six cargo-dist targets but stopped before creating a tag/draft because its wrapper assertion expected a concatenated URL literal while the correct wrappers compose an exact-tag base with a bounded asset name. Replaced the impossible assertion with separate rendered-base and asset-call checks; v2.0.0 remains unpublished and is safe to rerun from the fix-forward main SHA.
- 2026-07-19 07:28 UTC - The fix-forward run created the unpublished 34-asset v2.0.0 draft, then both Apple credential preflights stopped before certificate import because Windows had recorded the two invoked helper scripts as mode 100644. Kept the draft private, changed the workflow to invoke both helpers explicitly through Bash, and corrected their Git executable bits to match the proven TR-300 scripts. The recovery remains same-tag and will rerun native qualification before publication.
- 2026-07-19 07:35 UTC - Manual recovery exposed two more fail-closed issues before publication: native workflow dispatch treated the draft release name as an existing Git ref, and PowerShell read LASTEXITCODE after piping version/snapshot output, producing a false failure even though the installed Global MSI reported sd300 2.0.0. Recovery checkouts now use the dispatch source SHA while keeping the draft tag as artifact metadata; native exit codes are captured immediately in both production managed installation and Windows validation.
- 2026-07-19 07:42 UTC - Apple credential preflight and notarized PKG build then passed on both Mac families; Apple Silicon completed the entire PKG lifecycle while Intel continued. Windows confirmed Global MSI takeover and version, then its update assertion failed without exposing captured output. Made this pre-public check hermetic against the exact draft tag and added exit/line/output diagnostics. The private draft PowerShell managed wrapper will be deterministically re-rendered from the production fix before final qualification.
- 2026-07-19 07:50 UTC - Both Apple families passed and attached the qualified PKG. Windows passed fresh takeover/diagnostics for all four native families, then the synthetic managed update preserved the binary and failed ownership because the fixture hardcoded LOCALAPPDATA while production honors XDG_CONFIG_HOME first. Aligned both prior and candidate receipt fixtures with production resolution and retained immediate native exit-code capture. The corrected managed PowerShell wrapper was re-rendered into the draft and verified from fresh bytes before this rerun.
- 2026-07-19 08:00 UTC - The aligned fixture proved a production origin-order bug: an exact managed receipt at the preferred Cargo-path location was still blocked when Cargo's registry had no tr300-tui owner. Changed absent/unowned Cargo metadata to mean not-Cargo while malformed, multiple, or version-conflicting evidence remains fail-closed; unknown paths still fail after every channel is evaluated. This Rust change invalidates the private draft binaries, so the unpublished draft will be discarded and rebuilt rather than publishing stale bytes.
- 2026-07-19 08:18 UTC - Rebuilt the private draft from corrected Rust; both Mac families and all five Windows update channels passed. Final cross-platform checksum qualification then caught CRLF endings in the four Windows-native sidecars, which made GNU sha256sum treat carriage returns as filename bytes. Changed the Windows producer to write explicit LF-only ASCII sidecars so published checksums are portable; the draft remains unpublished pending regenerated Windows attachments and final qualification.
