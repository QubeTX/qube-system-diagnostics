TT;DR: Publish accurate v2 install/diagnostic content and restore SD-300 immediately after ND-300 only after the public v2 release is verified. A prepared isolated website branch already contains the discoverability-only portion.

## Why
Direct operator decision. CLI bootstraps are the advertised defaults, installers are alternatives, and the site must not expose stale v1.4.3 claims when SD-300 becomes discoverable again.

## Plan
Update SD-300 product/install content, wrappers, offline guide, fallback version, package assets, lifecycle copy, changelogs, and agent guidance. Rebase or reproduce the prepared `codex/sd300-relist-after-v2` changes, verify public links, then merge to the auto-deploying website main branch.

## Impact
Makes SD-300 discoverable and deploys public install instructions. Shaughv OS remains delisted.

## Acceptance
Every visible product sequence is TR, ND, SD, WB; CLI is primary; direct PKG and four Windows installers are accurate alternatives; all links resolve to verified v2 assets.

## Verification
- [x] Public v2 release and assets verified before website merge
- [x] Website lint/build and desktop/mobile route checks pass
- [x] SD-300 appears immediately after ND-300 everywhere
- [x] Shaughv OS remains undiscoverable
- [x] Install commands and filenames are versionless

## Status
Complete. Website PR #13 is merged and the Vercel production deployment is live with v2.0.2 fallback/content, versionless CLI-first commands, native alternatives, and SD-300 immediately after ND-300. Shaughv OS remains separately delisted.

## Activity
- 2026-07-19 09:25 UTC - merged website PR #13 only after public v2.0.2 qualification and offline bundle PR #1. Production `/sd300`, `/install-guide`, and `/executables` pass 1440px/390px browser checks, link/order assertions, and console/overflow checks.
- 2026-07-18 14:45 - created from the website relisting decision.
- 2026-07-18 16:31 - added direct CLI-first latest commands, PKG and four Windows alternatives, v2 fallback/content, authoritative install copy, and final product ordering; lint/build and 1440px/390px browser checks pass.
- 2026-07-18 17:01 - fast-forwarded the prepared branch onto ND-300's newly merged v3.7.3 site release, resolved additive guidance conflicts, and assigned the deferred SD-300 rollout its own website v1.17.0; lint/build remain green.
