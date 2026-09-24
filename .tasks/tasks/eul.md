TT;DR: Replace both Windows MSI placeholder license pages with the existing product license and publish a small corrective release.

## Scope
Use LICENSE.md (PolyForm Noncommercial 1.0.0), preserve license terms, acceptance/navigation and installer ownership. Prepare 4.0.2 consistently across product version surfaces. No collector, UI, runtime or performance changes.

## Acceptance
Both MSI editions embed and render the full existing license instead of WiX placeholder text. Hosted Windows composite qualification passes, then authorized release and public-byte verification. Operator explicitly excluded performance work on 2026-09-24: leave performance gates as they are; report any automated blocker separately without tuning or weakening thresholds.

## Verification
- [x] License RTF generated from authoritative LICENSE.md; source consistency check passes.
- [x] All 17 product version surfaces agree on 4.0.2.
- [x] Built Global and Corporate MSI license contents match the authoritative RTF and render through Windows RichEdit.
- [x] Hosted Windows installer lifecycle passes on exact candidate.
- [x] Release published and public MSI EULA verified.

## Status
DONE — Public 4.0.2 published 2026-09-24 23:27:10 UTC. PR #14 merged as abf55bd47a99ae33818d46d9c844d96353379541; immutable tag and release manifest match. Candidate CI 36067187928 and Windows qualification 36067190730 passed; merged CI 36069849623 and all native producers passed; final Qualify and Publish 36072546804 passed including post-public lifecycle and provenance checks. All 59 assets are public, latest is 4.0.2 and crates.io 4.0.2 is not yanked.

Global, Corporate and compatibility public MSIs report ProductVersion 4.0.2, match SHA-256 sidecars, pass attestation verification and embed the full LICENSE.md-derived EULA (4357 characters rendered by Windows RichEdit). Both latest MSI routes return the same verified bytes. Evidence: docs/qualification/v4/eula-4.0.2.md and target/eula-public-4.0.2/. Operator explicitly authorized the version-scoped performance exception in ADR 0021; thresholds and failed measurements remain unchanged. Future performance goals remain in #r16/#r17. No local installed-product mutation was requested or performed.

## Activity
- 2026-09-24 — Published 4.0.2 and verified all three public MSI license payloads, versions, hashes and attestations plus both latest routes. Final publication run 36072546804 passed; task completed.
- 2026-09-24 — Final candidate CI and Windows lifecycle passed. Merged PR #14 as abf55bd; release archive build passed and native producers are running. Draft identity matches the merged commit.
- 2026-09-24 — Operator explicitly authorized the 4.0.2-only exception and production deployment. Added ADR 0021 and a separate release-acceptance result without modifying benchmark thresholds, raw failures, or the exact-CI publication barrier.
- 2026-09-24 — Windows run 36062830554 passed completely. Three downloaded MSI artifacts passed local full-license readback. CI 36062833833 failed only native interaction timing; publication held pending explicit resolution of unchanged gates versus a release-scoped exception.
- 2026-09-24 — Hosted MSI build and embedded EULA checks pass for both editions on 0d27a78 (Windows run 36062830554). Local rendered text matches every source word; the checker rejects an old placeholder MSI. Operator authorized production deployment when ready.
- 2026-09-24 — Confirmed both MSI definitions omit WixUILicenseRtf; EXE already uses LICENSE.md. Operator confirmed existing 1.0.0 license and authorized a small release, excluding performance work. Added shared generated RTF and embedded-MSI verification.
