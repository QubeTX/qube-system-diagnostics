TT;DR: Publish one coherent versionless artifact set whose stable latest links resolve to exact verified release bytes, with no partial or contradictory release state.

## Why
Direct operator order for versionless downloads and dependable updates. The current cargo-dist release notes expose versioned commands and lack the native installer matrix and manifest contract.

## Plan
Create `release-manifest.json` and SHA-256 inventory, canonical `sd300-*` assets, compatibility aliases, candidate workflows, exact-main gates, crates publication ordering, immutable tag hosting, and post-release public-byte audit.

## Impact
Changes release workflows and asset inventory. Old tags/assets remain immutable; failures fix forward.

## Acceptance
Every public URL is stable/versionless, every updater pins exact-tag bytes after discovery, and the release only completes when all artifacts and hosted tests agree.

## Verification
- [x] Manifest-derived asset allowlist matches hosted release exactly
- [x] Latest aliases hash-match exact-tag artifacts
- [x] Release notes and docs contain no versioned install command
- [x] Clean committed-tree package and publish dry-run pass
- [x] Partial release states stop or repair only the explicitly supported case

## Status
Complete. v2.0.2 is public with 46 qualified assets, stable latest names, exact-tag/sidecar verification, native gates before crates publication, and a successful post-public lifecycle rerun.

## Activity
- 2026-07-19 09:28 UTC - post-release documentation push exposed that successful no-deploy Release runs still launched native workflows, which then failed because no draft existed. Added a cheap draft gate so public/already-published versions skip native runners successfully while real drafts and manual Mac credential-only preflights retain their existing behavior.
- 2026-07-19 09:25 UTC - v2.0.2 passed the full asset allowlist/hash gate, crates.io publication check, latest publication, and public managed-shell/Cargo lifecycle qualification.
- 2026-07-18 14:45 - created from the versionless release requirement.
- 2026-07-18 16:31 - moved crates publication behind the complete Windows/macOS asset and test matrix; actionlint and dist plan pass with no versioned public lifecycle asset names.
