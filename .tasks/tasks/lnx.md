TT;DR: Prove Linux native diagnostics and the managed shell/Cargo lifecycle without conflating cross-compilation with live hardware evidence.

## Why
Direct operator request for all-platform consistency. Linux uses the managed shell bootstrap as default and has no v2 DEB/RPM commitment.

## Plan
Exercise x64 and ARM builds, native/hosted collectors, sysfs/hwmon/DRM/power_supply/PSI/storage sources, shell updates, proven Cargo updates, uninstall, and unknown-owner refusal.

## Impact
Completes platform parity without inventing an unrequested package-manager channel.

## Acceptance
Linux diagnostics and managed shell lifecycle pass on supported architectures with evidence labels.

## Verification
- [ ] GNU x64 and ARM builds/tests pass
- [ ] MUSL release target passes
- [ ] Shell install/update/uninstall preserves its receipt
- [ ] Proven Cargo origin remains Cargo
- [ ] Live or fixture hardware evidence covers sensors, GPU, battery, and storage

## Status
Cross-target source proof is complete for GNU x64/ARM and MUSL. The stable shell lifecycle is implemented and locally parsed; the final public gate now performs real v1.9.9-to-candidate managed-shell and Cargo updates before fresh install/uninstall. Hosted execution and hardware availability evidence remain in #ga2.

## Activity
- 2026-07-18 14:45 - created from the original all-platform request.
- 2026-07-18 16:31 - all Linux targets compile and the final qualifier contains a public latest shell install/update/uninstall receipt smoke; hosted execution awaits release qualification.
- 2026-07-18 17:01 - changed the public qualifier from already-current checks to real managed-shell and Cargo version transitions, including same-owner JSON and final-version verification.
