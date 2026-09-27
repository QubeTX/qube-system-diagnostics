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
Active: revised 4.1.0 Windows build, navigation, native tests and strict bindings pass. Live reference captures verify default/narrow layout and raster scales; physical Windows display-scale acceptance remains distinct. Final resource measurement and hosted qualification pending. Release authorized after qualification; candidate remains unpublished.

## Activity
- 2026-09-27 — Revised Windows build passes distribution pins/path checks and live navigation again. Confirmed 3/2/1 card columns at window widths 1180/950/760 after waiting for responsive layout; all six cards plus Processes fit at 1180x760. Native suite: 78 passed, 2 expected skips; strict check validates the emitted model contract. Root 4.1.0 tests and clippy pass. Preparing hosted candidate while retaining the initial CPU overrun and physical-DPI limitation. (agent: codex)
- 2026-09-27 — Initial live Windows candidate populated all cards, passed card/process navigation, details/mode toggles and 16 keyboard inputs; reference captures at 1180/950/760 and raster scales 1/1.5/2 retained locally. Operator rejected the visual result: hardware names clipped and readings crowded. Revised names, grouping, missing-state typography and card density; 4.1.0 version surfaces aligned. Initial observer-free 45-second sample: 2.355% one-core CPU (FAIL), 136.21 MiB RSS and 254.66 MiB private (PASS); no gate waived. Revised native build hit memory exhaustion alongside test compilation; stop concurrent compiler runs and retry serially after cache compression. (agent: codex)
- 2026-09-27 — Full root tests, 18 engine tests, native tests (77 passed, 2 expected skips), clippy and strict model checks pass. Disk exhaustion interrupted the Windows build; non-destructive cache compression recovered space, and the build is retrying. Operator authorized release after confidence and existing gates. (agent: codex)
- 2026-09-27 — Created on codex/sd300-overview-cards; confirmed Overview currently enables only Fast/Static and collects only CPU/memory. Next: subscribe required lanes and summaries. (agent: codex)
