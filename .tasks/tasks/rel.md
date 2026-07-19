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
- [ ] Manifest-derived asset allowlist matches hosted release exactly
- [ ] Latest aliases hash-match exact-tag artifacts
- [ ] Release notes and docs contain no versioned install command
- [ ] Clean committed-tree package and publish dry-run pass
- [ ] Partial release states stop or repair only the explicitly supported case

## Status
Locally ready. Stable/latest names, exact-tag verified bytes, draft native qualification, post-native crates publication, and final latest publication are implemented. Actionlint, dist plan, package inventory, publish dry-run, and rendered PowerShell routers pass; the clean committed-tree gate and hosted run remain.

## Activity
- 2026-07-18 14:45 - created from the versionless release requirement.
- 2026-07-18 16:31 - moved crates publication behind the complete Windows/macOS asset and test matrix; actionlint and dist plan pass with no versioned public lifecycle asset names.
