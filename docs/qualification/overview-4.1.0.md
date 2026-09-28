# Overview candidate qualification

2026-09-27. Candidate branch: `codex/sd300-overview-cards`. This is an unpublished 4.1.0 candidate. Operator release authorization is conditional on confidence and qualification. ADR 0023 accepts only the recorded Windows Overview CPU result; timing and other qualification requirements remain open.

## Latest combined optimization, b19ee7a

All six native interaction reports from [CI 36366744715](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36366744715) complete their twelve cohorts and clean shutdowns. Original thresholds are unchanged. Full compact results and report/binary identities are retained in [overview-timing-b19ee7a.json](overview-timing-b19ee7a.json).

| Target | Worst frame p95 | Worst input p95 | Timing result |
|---|---:|---:|---|
| Linux GNU ARM64 | 16.141 ms | 22.783 ms | Pass |
| Linux GNU x86-64 | 21.297 ms | 33.910 ms | Frame fails |
| Linux musl x86-64 | 20.855 ms | 33.997 ms | Frame fails |
| Apple Silicon | 25.323 ms | 63.752 ms | Frame and input fail |
| Intel Mac | 18.434 ms | 21.974 ms | Frame fails |
| Windows | 17.810 ms | 49.084 ms | Frame fails |

All ordinary-refresh maximum checks pass. Intel's result is substantially lower than the shadow-only report, but separate hosted sessions are not a controlled benchmark. Windows' earlier shadow-only pass and this failure also show run variation; preserve both. Linux x86-64 remains dominated by the combined pixel-render/host-present interval (19.538 ms GNU, 19.613 ms musl p95). Add nested `raster` and `host_pixels` profiling to distinguish those paths without changing the existing combined interval or gates.

The composite Windows installer qualification [36366898012](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36366898012) passes on b19ee7a. Later graphics spacing and secondary-card hierarchy changes still require exact-source qualification before publication. The old-layout local resource sample was interrupted after the operator's next visual correction and is not qualification evidence.

After all six timing artifacts were retained, the remaining old-source resource job and superseded spacing-only PR CI were canceled to qualify the revised hierarchy/diagnostic source. No incomplete resource window is counted as acceptance. The revised hierarchy passes 83 native tests (2 expected skips), strict bindings and its Windows build; actual window inspection confirms the default footprint and clearer readings. A final trailing-column adjustment for unavailable temperatures is validated by tests and awaits the refreshed native preview.

## Functional and visual evidence

- Root tests, separate engine tests, native tests, strict model bindings, product-version reconciliation and pinned Windows builds pass for the implementation.
- Windows automation confirms fresh-launch readings, six card destinations, two process-sort destinations, expansion, mode switching and keyboard input.
- Live retained-scene reference captures show three/two/one columns at window widths 1180/950/760. Six cards and Processes fit at 1180 by 760. The revised graphics card uses compact model names and separate memory lines; a regression covers two adapters plus an additional count and delayed notice.
- Reference raster captures at scales 1, 1.5 and 2 are not physical Windows display-DPI acceptance. The actual b19ee7a native candidate was subsequently inspected at 100% and physical 150% Windows scaling: adapter names occupy their own lines and do not overlap usage/memory. The default window fits all cards and Processes; the shorter scaled viewport uses vertical scrolling. The active 1920x1080 monitor's standard scale menu stops at 175%, leaving physical 200% acceptance unverified. Original 100% (Recommended) was restored and read back; the isolated preview shut down cleanly.
- Multi-GPU/unified memory, missing and delayed observations, fixed-volume selection, complete-inventory process leaders and finding identity/severity have deterministic coverage. This is not a claim of a physical hardware matrix.

## Original resource gate remains failed

Operator acceptance: on 2026-09-27 the operator answered "That's okay" for the measured Windows Overview 2.24% CPU result. ADR 0023 records that specific result as nonblocking without changing its original failed verdict or extending acceptance to other workloads or platforms.

Production Windows artifact, observers and compilers closed, 15-second warmup followed by a 330-second Overview measurement using `scripts/measure-gui-windows.py`. CPU is percent of one logical core including owned descendants; RSS is the conservative process sum.

| Source commit | CPU | RSS MiB | Private MiB | Verdict |
|---|---:|---:|---:|---|
| 29bb41d | 2.258% | 125.16 | 259.58 | CPU fails |
| 0b1e53d | 2.239% | 124.95 | 259.48 | CPU fails |

Original limits are CPU 2%, RSS 150 MiB and private 300 MiB. Both runs shut down cleanly. Avoiding unused per-core collection did not materially resolve the cost. Retained local reports: `target/overview-resources-final.json` and `target/overview-resources-optimized.json`.

The opt-in `profile-monitor 30 --overview` diagnostic splits CPU, memory, full process inventory and network. On this machine the process stage averaged 8.76 ms (p95 10.03 ms), versus CPU 0.25 ms, memory 0.32 ms and network 1.17 ms. These sequential timings use quantized process counters, omit child/service work and are not a replacement for the whole-product gate. They identify the next investigation, not an established kernel-call root cause. Preserve complete-inventory process leaders and existing lane cadence.

## Hosted evidence

[CI 36348355191](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36348355191) tests exact source 0b1e53d. Core Windows/macOS/Linux, security and distribution planning pass. Native GUI qualification has failures; this candidate cannot publish.

- Linux GNU ARM64: default User navigation frame p95 18.336 ms exceeds 16.7 ms. Its present p95 alone is 16.810 ms. Input and refresh maxima pass in the retained report.
- Apple Silicon: frame p95 reaches 40.317 ms across the tested cohorts, exceeding 16.7 ms. Input p95 and ordinary-refresh maxima pass in that report.
- Native reports are retained as the run's `sd300-gui-interaction-*` artifacts.

The later exact-source [97d569f matrix](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36349910900) is complete. All six targets fail original frame p95; both Macs also fail input p95. Every report contains all twelve cohorts and a clean shutdown, with no functional exception. A compact projection with source report and binary hashes is retained in [overview-timing-97d569f.json](overview-timing-97d569f.json).

| Target | Worst cohort frame p95 | Worst cohort input p95 |
|---|---:|---:|
| Linux GNU ARM64 | 20.308 ms | 28.534 ms |
| Linux GNU x86-64 | 24.802 ms | 37.088 ms |
| Linux musl x86-64 | 22.739 ms | 35.081 ms |
| Apple Silicon | 41.339 ms | 52.536 ms |
| Intel Mac | 41.621 ms | 87.852 ms |
| Windows x86-64 | 23.838 ms | 46.801 ms |

Limits remain frame p95 16.7 ms and input p95 50 ms. Ordinary-refresh maxima pass on every target. On ARM64, keyboard and refresh cohorts pass the frame target; default-window navigation is dominated by the combined software-raster/host-present stage (18.696 ms present p95 versus 20.308 ms frame p95). On Apple Silicon, layout alone reaches 18.756 ms p95 in a navigation cohort; drawing/presentation is another substantial cost. Stage percentiles describe separate sample distributions and must not be added or used as the decomposition of one individual frame.

This is not evidence of a new overview-only regression. The retained v4 `native-interaction-latency-08b7fe5.json` already records original frame failures on ARM64 and both Macs. Different runner sessions and workloads prevent treating those old numbers as a controlled before/after benchmark.

The existing Mac native stack diagnostic observed a hidden window, so it could not explain visible layout or drawing. The follow-up diagnostic adds separate foreground Overview and Processes stack captures on each Mac architecture, using production binaries without automation. Attached samples are attribution evidence only, never resource or timing acceptance.

## Foreground attribution, 5f96a8d

[Diagnostic run 36362469153](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36362469153) changes CI diagnostics and acceptance documentation, not product runtime code.

Apple Silicon's production Overview and Processes captures both complete and shut down cleanly. Their normal, non-automation GUI SHA-256 is `8c6cb9788a86a74a6823a6f027d482bca02aa19d6ef25bf8c8c80b7d7a32b43a`. The `sd300-gui-resource-smoke-macos-arm64` artifact contains the full reports, including native stack text and separate live-thread counters.

Intel's production Overview and Processes captures also complete and shut down cleanly, and show the same shadow-blur and repeated measurement paths. Its GUI SHA-256 is `1c63c85c05baf5d5344f39e95e4eba48493df37f5bb9a6d15a4167dc04d23311`; raw reports are in `sd300-gui-resource-smoke-macos-x86_64`. Selected stack lines and full-report hashes for all four captures are retained in [overview-mac-stacks-5f96a8d.json](overview-mac-stacks-5f96a8d.json).

Observed main-thread paths on both pages:

- Packet presentation reaches `NativeSdkPacketDrawEffect`, `NSBezierPath fill`, Core Graphics shadow rendering, `RIPLayerGaussianBlur` and `RIPLayerSymmetricConvolve`. Source inspection connects this to inherited small-panel shadow tokens: `emitPanelWidgetChrome` emits an opaque panel shadow and AppKit draws it with `NSShadow`.
- Recursive `widget_layout` intrinsic sizing reaches `native_sdk_appkit_measure_text` and, on Processes, `native_sdk_appkit_measure_text_advances`. The width path includes Core Foundation string formatting. The host already caches measured widths, but a lookup still resolves the registered font and constructs formatted NSString keys before checking that cache.

These are demonstrated production hot paths, not proof that either alone explains every slow frame. The five-second stack sample contains waits and measures ordinary refresh, not a synchronized capture of the qualification run's worst navigation event. Do not sum recursive sample counts or turn sample frequency into CPU percentages.

The separate Apple Silicon timing repeat still fails frame p95 at 24.486 ms, while input p95 now passes at 24.665 ms and refresh/shutdown checks pass. The prior 41.339/52.536 ms failures remain recorded. There is no runtime change to credit for that improvement; runner/run variation is established, its cause is not.

The next focused experiments are (1) isolate the cost of decorative small-panel shadows while preserving the card geometry, borders and palette, then (2) reduce duplicate native text measurement with bounded, correctly invalidated reuse. Any SDK change must go through the canonical patch and all distribution pins. Neither experiment is claimed implemented or qualified by this diagnostic evidence.

Responsiveness work remains owned by #r16, resources by #r17, and overview/release acceptance by #ov6. Complete physical DPI and composite installer qualification, exact-candidate CI, original performance gates and public-byte verification before describing this as released or deployed. Do not merge to main to trigger publication while these requirements remain open.
