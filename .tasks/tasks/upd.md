TT;DR: Replace the Cargo-first updater with one explicit managed-channel state machine shared by install, update, and uninstall. Official `irm` and shell bootstraps are the advertised defaults; native packages remain options, and only a deliberately fresh official install may change ownership.

## Why
Direct operator order, reconciled against the latest TR-300, ND-300, and Goose implementation tasks on 2026-07-18. The current SD-300 updater merely tries Cargo and then a generic installer, so it cannot prove ownership or preserve MSI/EXE/PKG origin.

## Normative invariants
1. **Managed CLI defaults:** Windows advertises a stable versionless `irm ... | iex` wrapper; macOS/Linux advertise a stable versionless `curl ... | sh` wrapper. Generated cargo-dist installers are exact-tag internal payloads, not the public ownership layer.
2. **Same-channel update:** `sd300 update` preserves installer family, edition, scope, target, and release track recorded by the proven owner. It never converts channels as fallback.
3. **Fresh intent:** A deliberately run fresh official wrapper or native installer may reinstall, downgrade, or replace another recognized official channel. It must converge to one active public command and one owner.
4. **Fail before mutation:** Unknown, conflicting, unauthorized, or unsafe opposite-scope ownership stops before destructive action. It never changes only the receipt or leaves stale native registration while claiming success.
5. **Windows authority boundary:** The recommended PowerShell wrapper may request UAC and orchestrate cross-scope takeover before launching MSI/EXE. A raw opposite-scope native installer cannot safely start another Windows Installer transaction and must point to the wrapper.
6. **Raw Cargo boundary:** `cargo install tr300-tui` has no post-install hook and is an advanced unmanaged path. A proven Cargo origin may update through Cargo, but raw Cargo is outside fresh-channel takeover guarantees and is not advertised as default.
7. **Mac transition:** Normal native install/update uses a directly signed, notarized, stapled PKG. A compatibility DMG is published only if immutable pre-v2 clients hard-code it, and new clients never use it as their preferred path.
8. **Verified completion:** Resolve `latest` once, pin exact-tag bytes, verify manifest/hash/target/channel, launch the owning transaction, then verify final version, path, receipt, registration, and public command before reporting success.
9. **Truthful failure:** Pre-commit failure leaves the old owner active. Post-commit cleanup failure leaves the new owner active and reports cleanup pending. JSON stdout contains one final object; human progress uses stderr.
10. **Stable public surface:** Public commands and filenames contain no version. Tags, native metadata, exact internal URLs, receipts, and versioned staging/slots retain version identity for integrity.

## Plan
Introduce a versioned receipt schema and conservative origin resolver. Port the proven one-binary transaction patterns from TR-300/ND-300, including detached maintenance helpers where the running executable cannot replace or uninstall itself. Add `update [--json]` and `uninstall [--json]`, preserve legacy flags, and test every channel transition and refusal.

## Impact
Intended: predictable updates, one active owner, safe installer switching, versionless public install commands, and copyable behavior across QubeTX products. Risks: native installer transaction ordering, UAC/scope boundaries, immutable legacy-client asset dependencies, PATH duplication, and false success after a detached handoff.

## Acceptance
Every managed channel has a documented identity, install root, receipt/native proof, update artifact, takeover rules, uninstall path, JSON outcome, and tested failure behavior. The ADR, README, changelogs, tests, and release automation agree.

## Verification
- [x] Origin resolution refuses unknown or conflicting ownership without mutation
- [x] Every managed channel maps only to its matching artifact
- [x] Fresh managed installers converge recognized prior owners, including intentional downgrade
- [x] Opposite-scope Windows native transitions fail before mutation and route through `irm`
- [x] Update/uninstall JSON emits exactly one final stdout object with truthful exit status
- [x] Immutable 1.4.x asset names are preserved through compatibility routers; public transition proof remains in #ga2

## Status
Done locally. The lifecycle engine, receipts/native proof, stable assets, compatibility routers, transactional cleanup, and JSON contract are implemented. Cargo ownership requires exact `.crates2.json` proof, overlapping exact Cargo/managed evidence uses the newer record and refuses ties, and Windows replacement uses a rollback-capable live-image handoff rather than depending on Restart Manager behavior.

## Activity
- 2026-07-18 14:45 - created from the operator's original updater/install request.
- 2026-07-18 14:49 - reconciled current TR-300, ND-300, and Goose lifecycle discoveries into ten normative invariants.
- 2026-07-18 16:31 - implemented every managed/native channel, exact-tag SHA-256 staging, receipt/native/Cargo provenance, fresh takeover, uninstall, versionless assets, and immutable-client routers; unknown-origin release-binary update failed closed in one JSON object.
- 2026-07-18 17:01 - incorporated the latest TR-300/ND-300/Goose findings: added the elevated Global worker, bounded live-image cleanup, metadata-timestamp overlap resolution, nested payload hash verification, and hosted real-version transition gates.
