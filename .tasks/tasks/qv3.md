TT;DR: Qualify and publish v3.0.0 only after the complete GUI and lifecycle matrices pass on all six existing targets and the physical Windows candidate is approved. Probe Native SDK's additional Windows ARM64 and Linux-musl ARM64 toolchains separately; do not advertise them until their complete SD-300 lifecycle is owned and qualified.

## Why
Direct operator order and release closure for the milestone. A green build on one host is not enough for a cross-platform monitoring product.

## Plan
Run Rust/Native SDK quality gates, all hosted native matrices, physical Alienware installer and performance proof, SBOM/provenance generation, immutable release qualification, public-byte verification, Cargo two-update messaging, and website documentation checks.

## Impact
Publishes the new major release to current users. Partial publication or inaccurate platform claims are unacceptable.

## Acceptance
All six existing target and lifecycle gates pass, the public release and crate are consistent, and the website documents install/update/uninstall and verification correctly. Any additional architecture is advertised only after the same installer/update/uninstall and runtime proof.

## Verification
- [ ] Hosted and physical qualification matrices pass with archived evidence
- [ ] Public crate, release assets, checksums, attestations, stable routers, and website copy agree on v3.0.0
- [ ] Fresh install, update, repair, GUI/TUI launch, and uninstall pass against public bytes

## Status
Queued behind #gux and #cpl.

## Activity
- 2026-07-20 21:26 — created from the approved v3 implementation plan (agent: codex)
- 2026-07-21 07:35 — coordinated all 18 product-version surfaces at 3.0.0, added exact Native SDK/Zig/Rust toolchain records and clean/offline dependency checks, expanded pull-request GUI builds across the five native host lanes plus pinned Alpine musl, and made final qualification fail closed unless source checkout, tag, draft target, artifact checksums, SBOM, and attestations all identify the same candidate. Local workflow structure, distribution pins, path-leak checks, Rust fmt/Clippy, 71 unit tests, 2 CLI compatibility tests, and 7 engine ABI tests pass; hosted and sustained-performance gates remain open (agent: codex)
- 2026-07-21 08:31 — hardened release qualification so an already-public rerun verifies immutable metadata instead of overwriting it, stable post-public installers are checksum-verified before execution, every manifest subject is provenance-verified, explicit tags and existing tags must resolve to the exact Cargo version/source SHA, and all host lanes prove clean-cache plus warmed/offline Native SDK restoration. The local cargo-dist plan contains all six required targets and no generic CLI-only MSI; hosted signing, GTK-free launch, native lifecycle, performance-soak, attestation, and public-byte proof remain open (agent: codex)
- 2026-07-21 18:31 — after 15 hours, performed a hard release-contract audit instead of weakening acceptance. Pushed head `389dba1` has green ordinary Rust CI and Windows Native GUI; five macOS/Linux GUI lanes share one Zig 0.16 strict-analysis compiler SEGV after 23/24 tests pass, and Windows lifecycle independently fails Global MSI same-version repair with exit 2. The release remains queued behind repaired hosted builds, sustained performance, lifecycle matrices, signing/provenance, and public-byte verification (agent: codex)
