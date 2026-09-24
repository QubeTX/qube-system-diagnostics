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
EULA FIX VERIFIED; PUBLICATION BLOCKED. Product commit 0d27a78864b6ac8b89ee36b5289f74db9a75bbbd / PR #14. Windows qualification 36062830554 passed the complete lifecycle. Downloaded Global, Corporate and compatibility MSIs each match wix/License.rtf and render 4357 characters in Windows RichEdit. Artifacts are under target/eula-qualification/windows/native.

Exact-candidate CI 36062833833 finished with all core platforms, security and dist plan passing, but all six native GUI jobs failed interaction performance gates (musl reports this within its combined build/package step; its log ends with Interaction performance gates did not pass). No EULA, build, or functional installer failure was found. Existing require-release-ci.py blocks publication on failed CI. Operator said to leave performance gates unchanged and also deploy when ready; a clarification asking whether to permit a 4.0.2-only publication exception is pending. Do not tune performance or bypass the gate without that answer. Next action: resolve the pending decision; if exception is authorized, record its exact version scope and preserve measured failures/thresholds, then resume normal qualified release/public-MSI verification.

## Activity
- 2026-09-24 — Windows run 36062830554 passed completely. Three downloaded MSI artifacts passed local full-license readback. CI 36062833833 failed only native interaction timing; publication held pending explicit resolution of unchanged gates versus a release-scoped exception.
- 2026-09-24 — Hosted MSI build and embedded EULA checks pass for both editions on 0d27a78 (Windows run 36062830554). Local rendered text matches every source word; the checker rejects an old placeholder MSI. Operator authorized production deployment when ready.
- 2026-09-24 — Confirmed both MSI definitions omit WixUILicenseRtf; EXE already uses LICENSE.md. Operator confirmed existing 1.0.0 license and authorized a small release, excluding performance work. Added shared generated RTF and embedded-MSI verification.
