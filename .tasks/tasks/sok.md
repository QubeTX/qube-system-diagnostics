TT;DR: TASK FOR CODEX. After v3.0.0 is public, run the deferred two-hour Processes soak and capture formal frame-p95/input-p95 evidence against the released bytes, with the machine quarantined and exit attribution understood.

## Why
The operator waived the pre-release two-hour soak on 2026-07-22 to focus the session on shipping v3.0.0 ("functional over perfect; patches can follow"). The evidence itself is still owed: sustained-run stability and formal latency percentiles were release-blocking budgets in AGENTS.md and remain the honest long-run proof. The original 2026-07-21 soak attempt was invalidated by an operator window close, not a product defect (see gux activity 2026-07-22 01:07 and ADR 0001).

## Plan
1. Install the public v3.0.0 (or current) release on the Alienware host through a supported channel.
2. Quarantine the run: no Codex Computer Use/UI Automation observers, no concurrent installer/acceptance work (the 2026-07-21 attempt died during concurrent Honk300 MSI acceptance + operator presence), no interactive use of the machine during the window.
3. Run `scripts/measure-native-gui-performance.ps1` with `-DurationSeconds 7200 -Section Processes` against the installed GUI binary; keep `-ChildStandardErrorPath` set.
4. Remember the Windows close mechanism: `close_policy = .hide` + tray-off converts ANY window close into a graceful code-0 quit within one second (`shouldQuitForHiddenWindow`). An early exit means something closed the window or signaled `Local\SD300.Gui.Quit.v1` — attribute before rerunning.
5. Capture frame-p95 (<=16.7 ms) and input-p95 (<=50 ms outside scans) evidence using the warmed-state `SD300_RENDER_BENCH` extension added during the scroll-lag fix, plus any bounded physical input sampling needed.
6. Record results (pass/fail against <=2% foreground CPU, <=150 MiB working set, <=300 MiB private, no growth) in this file and the changelogs if a defect is found.

## Impact
Completes the deferred long-run performance evidence for the v3 line. A failure here becomes a patch-release driver, not a retroactive release blocker.

## Acceptance
A complete, uninterrupted two-hour soak against released bytes passes all budgets with no unbounded growth, and formal frame/input percentile evidence exists.

## Verification
- [ ] Two-hour Processes soak on released bytes completes with all budgets green
- [ ] Frame-p95 and input-p95 evidence captured and within budgets
- [ ] Any anomaly attributed (exit source, growth, stall) before rerun

## Status
Backlog. Blocked on #qv3 (needs the public release first). All context an agent needs is in this file, ADR 0001/0002 (docs/adr/), and the gux/nsp task histories.

## Activity
- 2026-07-22 01:07 — created by Fable as the operator-directed replacement for the waived pre-release soak gate; assigned to Codex for an overnight run after the v3.0.0 release (agent: fable)
