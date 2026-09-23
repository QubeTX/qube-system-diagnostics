# Native GUI review

Operator expanded the v4 scope on 2026-09-23 to include a GUI design/UI/UX review and improvements. This review retains the existing Warm Carbon identity and pinned Native SDK. It is part of the unpublished v4 candidate.

## Evidence and limits

Current Windows candidate was launched with isolated settings after the completed, unobserved fifteen-minute resource window. Native Computer Use captured the actual application at its default 1180 by 760 client size. The eleven original captures are retained locally under `target/gui-ux-review/captures` and are not published because live monitoring includes device and process identifiers. No optional installation, privileged read, network scan or bandwidth test was executed during the visual review. Opening and dismissing the SpeedQX confirmation was exercised.

Windows accessibility exposes the named canvas and window controls, not the individual canvas controls. This is an existing limitation tracked by #acc; keyboard operation and visible focus are separate checks and do not establish screen-reader support. Native macOS/Linux visual acceptance is not implied by a Windows capture or shared markup compilation.

## Findings and implementation

| Priority | Evidence | Finding | Change |
|---|---|---|---|
| High | 01 overview; startup source path | Initial numeric defaults could be described as light CPU demand or low memory pressure. | Gate values and interpretations on a captured sample; retain an explicit delayed/interrupted state for older readings. |
| High | 03 bandwidth consent | The throughput limitations were ellipsized, even at the default window size. | Wrap explanatory and consent paragraphs, retain the complete time/data budget and separate M-Lab opt-in. |
| High | 04 process table; captured-page code | Values updated each second while displayed rank was intentionally retained for thirty samples. | Preserve the engine's full-inventory rank on every page update; retain identity including process creation time. |
| High | 06 storage, 07 graphics, 10 thermals | Valid measurements displayed “The collector has not run yet” because an omitted optional detail inherited a pending default. | Treat absent detail as absent; keep initial uncollected observations distinct. |
| Medium | 01 overview | Large headings, identity and implementation-facing explanations displaced readings and findings. | Reduce page headings, move identity below live readings/history, use plain descriptions and central findings. |
| Medium | 02 network | Optional installation buttons, including an unrelated storage helper, preceded live network data. | Put monitoring first, group deliberate diagnostics separately, move SMART setup to Disk, and separate bandwidth controls. |
| Medium | 04 processes; 11 drivers | Controls competed with counts in a single row; numeric columns wrapped unevenly. | Separate sorting controls, label sort direction, simplify process memory cells and give driver versions more room. |
| Medium | 05 settings; all sections | Viewing mode required a trip to Settings; explanations were clipped. | Expose mode switching in the header, wrap settings descriptions and remove obsolete About interface labels. |
| Medium | graphics source bindings | Collected unified/shared GPU memory categories were not explained by the view. | Label unified memory independently and expose recommended working set, shared limits and identity in the appropriate detail. |
| High | 16 compact process table; paging source | Only eight of each sixteen returned rows were visible before Next advanced. | Show every row of the bounded page; verify the last row and next offset. |
| Medium | 14 compact network; full inventory | Numerous virtual adapters displaced the active physical interface from the bounded list. | Prioritize active aggregate contributors before selecting the visible interface rows. |
| Medium | 16 compact process controls | Direction-arrow glyphs were missing in the button font. | Use short sort buttons and a readable direction description. |
| High | 24 network detail; diagnostic badge source | Connectivity results were labeled Current without evaluating their own capture age. | Evaluate diagnostic topic availability and age independently of fast readings, distinguish DNS resolution duration from RTT, and use readable probe states. |

## Review flow

1. Overview: captured; hierarchy and startup-state corrections implemented.
2. Network: captured after readings settled; monitoring/action ordering corrected.
3. SpeedQX consent: opened and dismissed; no bandwidth test; wrapping corrected.
4. Processes: captured populated inventory; stale displayed ordering corrected.
5. Settings: captured; mode switched in the isolated session; descriptions corrected.
6. Disk: captured in Technician mode; helper placement and explanatory wrapping corrected.
7. Graphics: captured in Technician mode; provenance/default-state and memory-category corrections implemented.
8. CPU: captured; smaller section heading and simpler history description implemented.
9. Memory: captured; description wrapping and loading interpretation corrected.
10. Thermals: captured; permission-denied explanation wrapping and battery row overflow corrected.
11. Drivers: captured; version-column readability improved.

## Post-change evidence

The first rebuilt candidate was captured at the default size and approximately 950-pixel compact client width: Overview (12), Network (13/14), complete SpeedQX consent (15), Processes (16), settled full-inventory search (17), and keyboard mode activation (18). Search retained visible focus; Tab reached the header mode button when both page buttons were disabled, and Enter switched modes. The isolated session closed cleanly. Consent was dismissed without running a bandwidth test.

That pass exposed the page omission, unsupported arrow glyphs and interface priority issue above. Revision 7b08a2d passes 63 ordinary native tests, with two opt-in benchmarks skipped, a freshly generated strict view/model contract, the release-style Windows build and distribution-pin/path-leak checks. Its final live readback captures Overview (19), readable process sorts (20), a complete sixteen-row page (21), the next page (22), numeric full-inventory search with pagination reset (23), active Wi-Fi first among nineteen interfaces (24), graphics detail (25), readable thermal/battery data at default and compact widths (26/27), and compact Settings (28). The isolated session again closed cleanly.

The final detail pass adds diagnostic-specific freshness and improves long graphics provenance and Settings spacing. Its dedicated fixture checks delayed/error/future capture states while CPU remains live, then recovery. Layout fixtures cover all ten destinations in both modes at 960, 1180 and 1600 pixels; they establish control bounds, not readable text or screen-reader support. The release-style rebuild passes 64 ordinary tests, strict bindings and distribution checks. Live Network capture 29 confirms readable probe outcomes, separately labeled DNS resolution duration and RTT, and diagnostic freshness; the isolated app closes cleanly.

The combined GUI and macOS termination-cleanup source subsequently passes all 66 native tests, including the two opt-in renderer benchmarks, with a fresh strict contract and isolated Zig cache. Host-native production-path scroll raster p95 is 9.61 ms for Network, 5.96 ms for Processes and 6.30 ms for Drivers. These are renderer microbenchmarks, not release-baseline CPU, complete-frame or input-latency acceptance. Two earlier attempts failed on inconsistent Zig cache/dependency paths; the isolated-cache run succeeds without changing the product or global cache.

The earlier unobserved fifteen-minute Windows foreground report, `candidate-direct-panels-gui-foreground-15m.json`, belongs to renderer revision 7037bbc: 1.106 percent of one logical core, 148.53 MiB family peak RSS, 248.83 MiB private memory, and clean shutdown. It is not final performance acceptance for the subsequent GUI changes. Final foreground/hidden/soak qualification and native macOS/Linux evidence remain separate.

## Requested font pairing

On 2026-09-23 the operator requested Makira and Gail Rock as the main faces and supplied their location in Documents. Makira remains the heading/display face; Gail Rock becomes body/navigation/control text. The supplied static files remain unchanged and private. All 67 native tests pass, including font parsing, coverage of monitoring characters, measured-versus-painted face agreement, layout bounds and both render benchmarks. Five build-input checks cover absent, mismatched, exact reconstructed, cached and oversized compressed input. Native strict and distribution-lock checks pass after a clean dependency restore.

Gail Rock's original file lacks the grave-accent glyph; technical text retains Plex Mono. The whole-page raster comparison exposed one RGB channel differing by one code value at a translated glyph edge. Text-page comparisons now explicitly permit at most one RGB code value, with exact alpha; panel, shadow and overview comparisons remain byte-exact. This documents floating-point coverage rounding without accepting changed geometry or incomplete damage.

Revision 2c3a2ad passes the release-style Windows build, distribution locks and path-leak checks. Actual-window captures 30 through 33 verify Overview, populated Processes at default and approximately 950-pixel compact width, and the complete compact SpeedQX budget, limitations and separate M-Lab consent. The bandwidth action was dismissed without starting a test; the isolated app closed cleanly. Both faces remain readable without clipped headings or consent. The engine SHA-256 is `6fe3369e87f7fab170c1a1b9232f17a0f1e8bab01fca1b5078a27444d745d2e0`; the reviewed SDK patch SHA-256 is `29a33deb964db32f0543e5b2dd9a832325eaab6b63e08f1a2861755a51c5c599`. Hosted font builds run separately in CI 35892988794. Windows performance qualification begins only after closing this visual session.

The unobserved fifteen-minute Thermals window of this font bundle passes all resource gates: 1.017 percent of one logical core, 143.88 MiB peak summed working set and 242.66 MiB private memory. Required collectors remain present at both boundaries and shutdown is clean. The exact GUI, engine and CLI hashes are in `candidate-font-pair-gui-foreground-15m.json`. This is foreground acceptance for that section; the hidden window, two-hour soak and complete-frame/input percentiles remain distinct open checks.

CI 35892988794 finishes successfully on all six native targets for 2c3a2ad, including the embedded font build, native tests, platform interaction and orderly-shutdown checks. This closes native build qualification for the font pairing. It does not claim physical Mac/Linux visual acceptance or replace the longer performance matrix.
