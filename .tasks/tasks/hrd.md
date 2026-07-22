TT;DR: TASK FOR CODEX. Post-release hardening sweep: every deliberately deferred robustness item from the v3.0.0 reviews and qualification campaign, none release-blocking, all documented with context.

## Why
The 2026-07-22 release drive deferred well-understood hardening to keep the release scope safe. Each item below has review or ADR context and a conservative failure mode today.

## Plan
1. Uninstall-fix trio (ADR 0003 / Terra review): consider full-HRESULT equality (0x80070091) instead of the low-word 145 check; noninteractive `File.Delete` for receipt removal; force the cleanup child's working directory outside the receipt tree with a regression test.
2. Engine robustness (engine/ABI review findings): make `freshness_ms`/`availability` real (compute at read time from `captured_unix_ms` or drive from Observations); move inline gpu/netstat collection off the engine thread so a slow provider cannot stall the 1 s fast tick; contain background-collector panics (reset running flags, surface a warning) so a topic cannot wedge silently.
3. GUI polish: rename `window_visible` to reflect presentation-active semantics; consider joining the Windows quit-signal thread at exit for symmetry; pooled allocator for per-second JSON parsing.
4. Installer polish: macOS PKG uninstall's terminal `pkgutil --forget` failure tolerance; document (or gate) the MSI fault-injection property `SD300TESTFAILAFTERCARGO` as an accepted qualification design.
5. Backlog quirks: `logical_processors` 0 in a Processes-only first session; one-frame network-rate spike on profile transition.

## Impact
Reduces latent field-failure surface; none of these change contracts.

## Acceptance
Each item implemented with a test or explicitly re-deferred with reasoning.

## Verification
- [ ] Uninstall trio resolved or re-deferred with reasons
- [ ] Engine staleness/threading/panic items resolved
- [ ] GUI and installer polish items resolved
- [ ] Quirks fixed or documented

## Status
Backlog. Blocked on #qv3. Context: ADR 0003, `.tasks/tasks/cpl.md` activity (2026-07-22), and the three review reports summarized in `.tasks/tasks/gux.md`/`cpl.md` activity.

## Activity
- 2026-07-22 12:35 — created by Fable consolidating all deferred hardening from the release-drive reviews (agent: fable)
