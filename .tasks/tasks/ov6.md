TT;DR: Make the app overview easy to scan with graphics, CPU, memory, disk, network and temperature cards.

## Why
The operator approved a compact dashboard aimed at gamers in User mode, with equal cards and stronger emphasis on graphics, CPU and memory.

## Scope
Implement the approved overview plan on codex/sd300-overview-cards. Preserve collector truth, missing readings, bounded histories, settings, TUI and public interfaces. On 2026-09-27 the operator additionally authorized a new version and deployment once validated and confident. The operator authorizes a 4.1.0-specific performance/display exception on 2026-09-29; functional and exact-source release checks remain mandatory (ADR 0025).

## Plan
Wire the Overview profile and complete-inventory summaries; prepare bounded presentation; replace the overview layout; test and inspect the native candidate.

## Impact
GUI overview and its collection subscriptions; shared profile scheduling and internal topic projections.

## Acceptance
All cards populate from a fresh launch, route to details, distinguish missing/stale data, and remain readable at compact/default sizes. Operator owns the approved visual requirements; repository policy owns performance and release gates. Two cycles without new evidence trigger diagnosis rather than another unchanged run.

## Verification

- 2026-09-27: bounded text-width reuse passes 81 native tests (2 skipped), strict bindings, coordinated versions and both patch preparers. Hosted native timing and release qualification remain open.
- [x] Root and engine tests for sampling and summary correctness
- [x] Native tests and strict bindings
- [x] Fresh-launch live readings, card navigation and expansion
- [ ] Default/narrow layout and display-scale inspection
- [x] Observer-free overview resource measurement — ce6b42e: CPU fails; RSS/private memory pass
- [ ] Exact-candidate hosted CI

## Status

Active: production source 4f2f253 passes exact CI and Windows/Linux installer qualification. Mac Intel package lifecycle passes; Apple Silicon stops before the updater because the synthetic prior executable is killed after in-place overwrite. Diagnostic 36635433166 pinpoints exit 137; unchanged retry reproduces it. Atomic fixture replacement passes the complete lifecycle on both architectures in 36635969196. Recover the stopped unpublished draft chain with the minimal harness fix; retain exact-source CI and all production gates.

## Activity

- 2026-09-29 - codex: corrected native diagnostic 36635969196 passes Apple Silicon and Intel trust/install/launch/update/uninstall against the unchanged signed 4f2f253 package. Apple Silicon now executes synthetic 1.9.9 and updates to 4.1.0 successfully. All production jobs have stopped; verify no 4.1.0 tag/publication, discard only its unpublished draft, then merge PR 16 and rebuild the complete candidate from one corrected source.

- 2026-09-29 - codex: PR 15 merged as 4f2f253 after candidate CI 36628600345 and Windows installers 36628604881 passed. Production CI 36632095352, Windows 36632821407 and Linux 36632821393 pass. Mac 36632821384 fails twice only on Apple Silicon synthetic fixture execution; diagnostic 36635433166 proves exit 137 before update. Qualify fresh-inode fixture setup in diagnostic 36635969196; then restart the unpublished chain with the minimal harness correction. Preserve original failures and source identities.

- 2026-09-29 — codex: Operator explicitly requests production deployment after the gate-owner choice. Record ADR 0025, preserve timing/CPU failures and defer remaining resource/physical-display evidence. Prior installer 36369836717 passes; all six final pre-exception interaction reports complete with clean shutdown. Next: final-source CI and installer oracle.

- 2026-09-27 — codex: production ce6b42e window 9708162 confirms complete status labels and default-size fit. Visual capture completed during a 60-second warmup; no further app interaction/observers or local builds during the following 330.047-second sample. CPU 2.528049% of one core (FAIL), RSS 132.5078125 MiB/private 260.40625 MiB (PASS), clean shutdown. GUI SHA-256 7d729862ab52960c41c0ee72aecbc7e5b1b9c55d6345c38da58d7818a3530270. Ask the gate owner about a dated exception versus continued blocking; preserve ADR 0023's narrow scope until answered. Raw local report: target/overview-resources-ce6b42e.json.

- 2026-09-27 — codex: actual final-hierarchy window reveals proportional-font shortening of the missing-sensor label even in the fixed column. Use the existing compact monospace status face; 83 native tests (2 skipped) and strict bindings pass. Verify the production build's actual status text before its observer-free resource window. Exact diagnostic-source CI 36369688990 and Windows installer qualification 36369836717 are running; no publication.

- 2026-09-27 — codex: operator requests clearer secondary-card hierarchy. Prepare adaptive byte-rate values/units, separate disk mount/free-space context, group network rates/history/scope, and emphasize available temperatures in aligned sensor rows. Actual native window 11338756 confirms the hierarchy and default-size fit. Fix the observed missing-temperature wrap with a consistent trailing column; final 83 native tests pass (2 skipped), including unit/missing-data and secondary-card bounds/nonintersection, and strict bindings pass. Close the preview cleanly. Cancel superseded CI 36368413767 and the remaining old-source resource window in 36366744715 after retaining all six completed timing reports; incomplete resource windows are not acceptance evidence.

- 2026-09-27 — codex: operator requests more separation after the overlap correction. Increase graphics heading gap from 6 to 10 pixels and adapter-group gap from 6 to 16, keeping the card size unchanged. All 81 native tests pass (2 expected skips), including two adapters plus extra-count/delayed-state containment and pairwise nonintersection; strict bindings and pinned Windows build pass. Actual native window 8460810 inspected: both adapter groups read separately and six cards plus Processes remain visible at default size. Stop the in-progress old-layout local resource sample after this steering; it is incomplete and provides no qualification result.

- 2026-09-27 — codex: b19ee7a live native window inspected at physical Windows 150% scaling: GPU names and usage/memory remain separate and readable. Standard scale choices on the active 1920x1080 monitor end at 175%, so 200% is unverified. Restored and read back 100% (Recommended), closed Settings and the isolated preview cleanly. Exact-source CI 36366744715 and Windows installer qualification 36366898012 are running; no publication.

- 2026-09-27 — codex: operator's live Windows capture reveals the second GPU name wrapping into memory text; prior reference capture and containment-only test missed the overlap. Move names to full-width single lines, use a separate usage/memory row, and separate thermal names from temperatures. All cards remain equal at 216 logical pixels. Pairwise text-bound checks pass at 1180/950/760 widths (81 native tests, 2 skips). Actual candidate window (PID 7776, build path ending windows-x86_64-automation/app/zig-out/bin) inspected via native Windows capture: names/readings are separated, six cards plus Processes fit at 1180x760. Internal 950-wide viewport also renders without overlaps; this is not physical OS-DPI acceptance.

- 2026-09-27 — codex: unpublished Windows installer run 36364695685 passes on 395e243. New 330-second local sample of that production build records CPU 2.807% (FAIL), RSS 132.54 MiB and private 260.71 MiB (PASS), clean shutdown. Operator captured the UI during this window, so this was not a quarantined repeat; preserve the measurement without extending ADR 0023. Replace older candidate qualification with the visually corrected source.

- 2026-09-27 — codex: Windows production build/distribution check and cargo publish dry run pass on 395e243. Broad SDK verification passes 902 tests / 3 skipped after fixing stale test fixtures. Shadow-only Linux ARM64 timing passes; Linux x86-64 still fails frame p95. Windows installer run 36364695685 remains in progress; exact final candidate qualification is still required.

- 2026-09-27 — codex: implement bounded native text-width reuse through the pinned SDK patch while the independent shadow-only run 36364119149 measures the first optimization. Preserve all release thresholds and exact-candidate checks; local cache regressions and native timing qualification remain pending.
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
