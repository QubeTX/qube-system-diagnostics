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
- [ ] Release published and public MSI EULA verified.

## Status
EULA FIX VERIFIED; AUTHORIZED RELEASE EXCEPTION IN VALIDATION. Product commit 0d27a78864b6ac8b89ee36b5289f74db9a75bbbd / PR #14. Windows qualification 36062830554 passed the complete lifecycle. Downloaded Global, Corporate and compatibility MSIs each match wix/License.rtf and render 4357 characters in Windows RichEdit. Artifacts are under target/eula-qualification/windows/native.

Operator subsequently authorized the specifically requested 4.0.2-only release exception: "Yes, get it deployed, please." ADR 0021 records it. The interaction script retains all thresholds and failed reports, with a separate version-scoped release acceptance only for complete functional cohorts and clean shutdown. Exact-source green CI is still required. Next: validate exception fixtures, push and dispatch exact-candidate CI; merge the verified EULA correction and monitor normal publication, then verify public MSI bytes.

## Activity
- 2026-09-24 — Operator explicitly authorized the 4.0.2-only exception and production deployment. Added ADR 0021 and a separate release-acceptance result without modifying benchmark thresholds, raw failures, or the exact-CI publication barrier.
- 2026-09-24 — Windows run 36062830554 passed completely. Three downloaded MSI artifacts passed local full-license readback. CI 36062833833 failed only native interaction timing; publication held pending explicit resolution of unchanged gates versus a release-scoped exception.
- 2026-09-24 — Hosted MSI build and embedded EULA checks pass for both editions on 0d27a78 (Windows run 36062830554). Local rendered text matches every source word; the checker rejects an old placeholder MSI. Operator authorized production deployment when ready.
- 2026-09-24 — Confirmed both MSI definitions omit WixUILicenseRtf; EXE already uses LICENSE.md. Operator confirmed existing 1.0.0 license and authorized a small release, excluding performance work. Added shared generated RTF and embedded-MSI verification.
