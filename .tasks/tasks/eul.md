TT;DR: Replace both Windows MSI placeholder license pages with the existing product license and publish a small corrective release.

## Scope
Use LICENSE.md (PolyForm Noncommercial 1.0.0), preserve license terms, acceptance/navigation and installer ownership. Prepare 4.0.2 consistently across product version surfaces. No collector, UI, runtime or performance changes.

## Acceptance
Both MSI editions embed and render the full existing license instead of WiX placeholder text. Hosted Windows composite qualification passes, then authorized release and public-byte verification. Operator explicitly excluded performance work on 2026-09-24: leave performance gates as they are; report any automated blocker separately without tuning or weakening thresholds.

## Verification
- [x] License RTF generated from authoritative LICENSE.md; source consistency check passes.
- [x] All 17 product version surfaces agree on 4.0.2.
- [ ] Built Global and Corporate MSI license contents match the authoritative RTF and render through Windows RichEdit.
- [ ] Hosted Windows installer lifecycle passes on exact candidate.
- [ ] Release published and public MSI EULA verified.

## Status
ACTIVE: EULA binding and deterministic RTF implemented on codex/sd300-msi-license. Next: validate local RichEdit rendering, commit/push and dispatch Windows qualification plus normal CI. No performance policy changed.

## Activity
- 2026-09-24 — Confirmed both MSI definitions omit WixUILicenseRtf; EXE already uses LICENSE.md. Operator confirmed existing 1.0.0 license and authorized a small release, excluding performance work. Added shared generated RTF and embedded-MSI verification.
