TT;DR: Make SD-300's measurements trustworthy, keep monitoring responsive when a provider fails, and redesign the terminal dashboard. Publish the complete result once, with optional ND-300 diagnostics.

## Why
Operator accepted the full v4 plan on 2026-09-22. Source audit found unnormalized network deltas, subprocess-runtime ping latency, CPU-truncated process sorting, stale history duplication, blocking refreshes, and incomplete non-Windows providers.

## Scope
All six existing product targets, shared collectors, TUI redesign, GUI data/action parity, schema-2 exports with schema-1 compatibility, optional companion installation, and one public release. Preserve existing lifecycle ownership, credentials, exports, GUI settings, ND-300's independent lifecycle, and open historical operator-visual acceptance. No automatic network repair, helper install, elevation, or throughput tests.

## Plan
Measurement foundations -> bounded collection -> platform coverage -> adaptive TUI -> companion integration and GUI parity -> performance/lifecycle qualification -> one public release.
Current cycle: prove network rates use actual elapsed time, process ranking includes the full inventory, and connectivity displays reply RTT. Deterministic tests are the first oracle; Windows plus hosted native CI follow every validated commit. A failure without causal output redirects the next cycle to instrumentation.

## Impact
Both frontends gain accurate, attributed readings and explicit incomplete states. Sampling and rendering changes can regress cadence, resource use, and terminal behavior, so qualify them against immutable pre-change bytes and contract fixtures.

## Acceptance
Functional bar: all accepted behavior implemented and exercised; all six supported targets remain usable independently and through the composite installer lifecycle.
Evidence bar: deterministic collector and failure fixtures; root/engine/native tests; PTY acceptance; independent OS-counter comparisons; foreground/hidden/soak performance; hosted installers; exact public artifacts and ownership verification.
Gate ownership: accepted operator plan and repository release policy. Operator permits research, provider fixtures, local Windows, and hosted native evidence for hardware unavailable physically. No inherited soak waiver is assumed.
Bounded convergence: one candidate per failing lane. After two cycles without new evidence, audit the measurement environment and diagnostic reporting before another product change. Never publish an incomplete candidate.

## Verification
- [x] Clean starting checkout at f83ae42; preceding audit passed 114 Windows library tests
- [ ] Measurement and failure regression fixtures pass
- [ ] Native Windows/macOS/Linux and musl qualification pass
- [ ] TUI and GUI behavior and parity pass
- [ ] Performance and soak acceptance pass
- [ ] Composite install/update/repair/rollback/uninstall passes
- [ ] Public release and exact bytes verified

## Status
ACTIVE on codex/sd300-v4-monitoring. Implementation starting; product remains 3.1.3 until coordinated release preparation. ND-300 public 4.0.1 is the integration baseline; its local 4.0.2 work is independent.

## Activity
- 2026-09-22 — codex: accepted implementation and single-release authorization; refreshed current main, created feature branch, upgraded the board bundle to installed 1.2.0 and verified its root-bound server.
