TT;DR: Make the app overview easy to scan with graphics, CPU, memory, disk, network and temperature cards.

## Why
The operator approved a compact dashboard aimed at gamers in User mode, with equal cards and stronger emphasis on graphics, CPU and memory.

## Scope
Implement the approved overview plan on codex/sd300-overview-cards. Preserve collector truth, missing readings, bounded histories, settings, TUI and public interfaces. On 2026-09-27 the operator additionally authorized a new version and deployment once validated and confident. Existing release gates remain unchanged.

## Plan
Wire the Overview profile and complete-inventory summaries; prepare bounded presentation; replace the overview layout; test and inspect the native candidate.

## Impact
GUI overview and its collection subscriptions; shared profile scheduling and internal topic projections.

## Acceptance
All cards populate from a fresh launch, route to details, distinguish missing/stale data, and remain readable at compact/default sizes. Operator owns the approved visual requirements; repository policy owns performance and release gates. Two cycles without new evidence trigger diagnosis rather than another unchanged run.

## Verification
- [x] Root and engine tests for sampling and summary correctness
- [x] Native tests and strict bindings
- [x] Fresh-launch live readings, card navigation and expansion
- [ ] Default/narrow layout and display-scale inspection
- [ ] Observer-free overview resource measurement
- [ ] Exact-candidate hosted CI

## Status
Active: implementation and revised layout verified locally. Operator accepted the recorded Windows Overview CPU overrun in ADR 0023. ARM64/Mac diagnosis identifies navigation drawing cost, decorative shadow blur and repeated text measurement; optimization and qualification remain in #r16. Physical Windows display-scale acceptance remains unverified. Release authorization remains conditional on qualification.

## Activity
- 2026-09-27 — Operator explicitly authorizes the remaining optimizations and release. Begin with the measured shadow cost, then bounded text-measurement reuse; complete native qualification and composite release checks before publication. (agent: codex)
- 2026-09-27 — Native foreground diagnosis complete on both Macs: Overview and Processes reach shadow-blur and repeated text-measurement paths; all four samples shut down cleanly. Evidence and exact artifact identities retained with #r16. Corrected a stale Windows harness fixture for Overview's Slow/Activity workers; five resource-harness and fifteen interaction-report tests pass. This cycle diagnoses remaining work, does not claim a renderer fix or publish. (agent: codex)
- 2026-09-27 — Operator accepts the specific 2.24% Windows Overview CPU measurement; record ADR 0023, preserving original verdict and #r17. Exact-source 97d569f run 36349910900 completes: all six native targets fail frame p95, both Macs also fail input p95, but all interaction cohorts and shutdowns complete. Mac layout and combined raster/present dominate; add separate foreground Overview/Processes native stack sampling to attribute those costs. This diagnostic cycle changes no product code or thresholds. (agent: codex)
- 2026-09-27 — 0b1e53d production Overview: observer-free 330-second CPU 2.239% FAIL; RSS 124.95 MiB/private 259.48 MiB PASS. Split profiling attributes most fast-lane cost to complete process inventory (mean 8.76 ms), not CPU/memory/network. Hosted run 36348355191 fails ARM64 and Apple Silicon frame limits; preserve original thresholds and route follow-up through #r16/#r17. Fix explicit storage finding navigation and add its regression. No publication. (agent: codex)
- 2026-09-27 — Production 29bb41d measured for 330 seconds with observers/builds closed: CPU 2.258% (FAIL), RSS 125.16 MiB and private 259.58 MiB (PASS), clean shutdown. Narrow the overview fast sampler to aggregate CPU plus full process/network captures; measure unchanged cadences again. Added a layout regression for two adapters plus additional-count/delayed state; it exposed footer overflow, fixed by moving the count into the header. Native tests now 79 passed, 2 expected skips. Engine version-assertion repair passes all 18 tests. (agent: codex)
- 2026-09-27 — Pushed implementation 29bb41d and dispatched exact-commit resource CI 36347312459. Root Linux/macOS tests and security audit pass; native jobs exposed a stale engine envelope assertion expecting 4.0.2. Corrected the test to use the package version; validate and redispatch after the observer-free 330-second production Overview measurement. (agent: codex)
- 2026-09-27 — Revised Windows build passes distribution pins/path checks and live navigation again. Confirmed 3/2/1 card columns at window widths 1180/950/760 after waiting for responsive layout; all six cards plus Processes fit at 1180x760. Native suite: 78 passed, 2 expected skips; strict check validates the emitted model contract. Root 4.1.0 tests and clippy pass. Preparing hosted candidate while retaining the initial CPU overrun and physical-DPI limitation. (agent: codex)
- 2026-09-27 — Initial live Windows candidate populated all cards, passed card/process navigation, details/mode toggles and 16 keyboard inputs; reference captures at 1180/950/760 and raster scales 1/1.5/2 retained locally. Operator rejected the visual result: hardware names clipped and readings crowded. Revised names, grouping, missing-state typography and card density; 4.1.0 version surfaces aligned. Initial observer-free 45-second sample: 2.355% one-core CPU (FAIL), 136.21 MiB RSS and 254.66 MiB private (PASS); no gate waived. Revised native build hit memory exhaustion alongside test compilation; stop concurrent compiler runs and retry serially after cache compression. (agent: codex)
- 2026-09-27 — Full root tests, 18 engine tests, native tests (77 passed, 2 expected skips), clippy and strict model checks pass. Disk exhaustion interrupted the Windows build; non-destructive cache compression recovered space, and the build is retrying. Operator authorized release after confidence and existing gates. (agent: codex)
- 2026-09-27 — Created on codex/sd300-overview-cards; confirmed Overview currently enables only Fast/Static and collects only CPU/memory. Next: subscribe required lanes and summaries. (agent: codex)
