TT;DR: Ship Global/Corporate MSI and EXE channels that install one stable SD-300 command, preserve their origin during updates, and handle fresh takeover or refusal transactionally.

## Why
Direct operator order. Corporate MSI must remain Corporate MSI on CLI updates, while a fresh EXE/MSI/IRM selection represents new intent.

## Plan
Adapt the proven WiX/Inno identities, scope-specific roots, receipts, maintenance helpers, downgrade flags, and hosted transition matrix from TR-300/ND-300. Keep Global per-machine and Corporate per-user.

## Impact
Adds four Windows native artifacts and UAC/scope behavior. Opposite-scope raw native transitions must refuse before mutation and point to the managed PowerShell wrapper.

## Acceptance
All four channels install, repair, update, downgrade, switch safely where authorized, and uninstall without duplicate PATH/registration.

## Verification
- [x] Global MSI and EXE lifecycle pass
- [x] Corporate MSI and EXE lifecycle pass without admin install
- [x] Matching CLI updates preserve exact format and edition
- [x] Fresh same-scope format changes become authoritative
- [x] Unsafe opposite-scope transitions refuse before mutation
- [x] Restart Manager/live-image and cleanup cases are verified

## Status
Complete. The hosted Windows matrix built all four stable native artifacts and passed fresh takeover, exact-origin update, deliberate downgrade, opposite-scope refusal, live-image handoff, cleanup, diagnostics, and uninstall checks.

## Activity
- 2026-07-19 09:25 UTC - v2.0.2 passed Global/Corporate MSI/EXE installation and real synthetic-prior same-channel transition checks; all public native bytes and sidecars were verified.
- 2026-07-18 14:45 - created from the operator's Windows installer requirements.
- 2026-07-18 16:31 - implemented Global/Corporate MSI and EXE packages plus managed-to-native, format-takeover, opposite-scope refusal, diagnostics, update-origin, and uninstall validation workflow.
- 2026-07-18 17:01 - added rollback-capable live-image/elevated-worker logic and a synthetic-prior hosted matrix that must perform real same-channel updates for Global/Corporate MSI/EXE.
