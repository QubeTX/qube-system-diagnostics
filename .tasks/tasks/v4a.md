TT;DR: Make SD-300's measurements trustworthy, keep monitoring responsive when a provider fails, and redesign the terminal dashboard. Publish the complete result once, with optional ND-300 diagnostics.

## Why
Operator accepted the full v4 plan on 2026-09-22. Source audit found unnormalized network deltas, subprocess-runtime ping latency, CPU-truncated process sorting, stale history duplication, blocking refreshes, and incomplete non-Windows providers.

## Scope
All six existing product targets, shared collectors, TUI redesign, GUI data/action parity, schema-2 exports with schema-1 compatibility, optional companion installation, and one public release. Preserve existing lifecycle ownership, credentials, exports, GUI settings, ND-300's independent lifecycle, and open historical operator-visual acceptance. No automatic network repair, helper install, elevation, or throughput tests.

## Plan
Measurement foundations -> bounded collection -> platform coverage -> adaptive TUI -> companion integration and GUI parity -> performance/lifecycle qualification -> one public release.
Current cycle: finish schema-2 availability, process identity and shared findings in both frontends; validate locally and push to the native matrix. Next: GPU, power, display and hardware inventory on macOS/Linux, preserving Windows providers.

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
ACTIVE on codex/sd300-v4-monitoring. Measurement and bounded-worker foundations implemented; product remains 3.1.3 until coordinated release preparation. ND-300 public 4.0.1 is the integration baseline; its local 4.0.2 work is independent.

## Activity
- 2026-09-23 � codex: ceeb937 implements schema/process/findings parity; Windows native tests passed 39 with 2 platform skips. Power/display/identity candidate adds Linux sysfs and native Apple APIs with unit fixtures; 137 library and 7 CLI tests pass. Expanding native matrix to execute root and engine tests on Intel/ARM Macs, Intel/ARM Linux and musl. Next oracle: platform linking and actual provider calls on those native runners.
- 2026-09-23 � codex: bf93c8a passed hosted CI 35823140862 and release-plan run 35823140822. Schema/process/findings candidate passes 133 library + 7 CLI + 10 engine tests and clippy; native ABI and bindings are qualified together. Next oracle: hosted six-target builds and non-Windows process observations.
- 2026-09-23 — codex: runtime correction 1c5d73c passed CI 35822025040 and release-plan run 35822025055 across all native lanes. Storage candidate passes 129 library + 7 CLI + 10 engine tests, clippy, and 38 native GUI tests (2 platform skips); live Windows physical-disk counters available without elevation. Primary-source unit contracts and macOS/Linux parser/layering fixtures added. Corrected staged model-contract readback; strict GUI check now validates current bindings. Next oracle: hosted storage-provider builds and fixtures on both Mac architectures and all Linux targets.
- 2026-09-23 — codex: f594bd9 pushed the worker boundary. A subsequent startup placeholder assertion exposed overbroad masking of populated offline thermal data; restricted it to active-session warmup and reran all 121 library tests successfully. This correction is pushed immediately for the same native oracle.
- 2026-09-23 — codex: a019682 passed every hosted CI lane, including six native GUI targets and Windows/Linux/macOS core tests. Bounded-worker candidate passes 121 library + 7 CLI + 10 engine tests, root clippy, and native GUI tests (37 passed, 2 platform skips). Live Windows slow probe returned disks, two GPUs, and sensors; real PTY opened progressively and exited with terminal restoration. Next oracle: hosted builds of this runtime revision; remaining v4 scope stays open.
- 2026-09-23 — codex: first measurement cycle passed 118 library tests, then 35 focused collector tests with bounded command execution (120 total discovered). Preserved pre-change release executable at target/v4-baseline/sd300.exe, SHA-256 42d0ca558626b2a20f96f770f3d7b1a8f4bbed904371ff2c5f025e324406bc51. Next oracle: full root/engine compatibility and hosted native platform compilation.
- 2026-09-22 — codex: accepted implementation and single-release authorization; refreshed current main, created feature branch, upgraded the board bundle to installed 1.2.0 and verified its root-bound server.
