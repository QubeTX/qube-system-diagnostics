TT;DR: Obtain written Makira app-embedding license evidence from the vendor (operator purchased from Craftwork Design; only font files were delivered), or replace the face with a distributable open font in a patch release.

## Why
v3.0.0 shipped Makira under an explicit operator authorization (2026-07-22): the operator paid for the font but received no license document. Possession of bytes is not embedding-license evidence; this task retires the residual licensing risk.

## Plan
1. Operator contacts the vendor (Craftwork Design / Yukita Creative — see the licensing links in `docs/agents/handoff/2026-07-22-002-...md`) for App/Game embedding license documentation matching the purchase.
2. Store the evidence pointer in `.tasks/secure/` or company records (never the bytes/credentials in tracked files); update `gui/assets/fonts/` retained-evidence notes.
3. If evidence cannot be obtained, select an OFL alternative visually close to Makira, wire it through the existing font-embedding path, and ship in a patch release.

## Impact
Closes the only licensing caveat on the shipped product.

## Acceptance
Embedding-license evidence on file, or the replacement font released.

## Verification
- [ ] License evidence obtained and referenced, or replacement shipped

## Status
Backlog. Operator-led; agent support for the replacement path if chosen.

## Activity
- 2026-07-22 12:35 — created by Fable; ship authorization and purchase context recorded in `.tasks/tasks/cpl.md` verification waiver (agent: fable)
