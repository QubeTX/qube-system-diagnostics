# Overview candidate qualification

2026-09-27. Candidate branch: `codex/sd300-overview-cards`. This is an unpublished 4.1.0 candidate. Operator release authorization is conditional on confidence and qualification. ADR 0023 accepts only the recorded Windows Overview CPU result; timing and other qualification requirements remain open.

## Functional and visual evidence

- Root tests, separate engine tests, native tests, strict model bindings, product-version reconciliation and pinned Windows builds pass for the implementation.
- Windows automation confirms fresh-launch readings, six card destinations, two process-sort destinations, expansion, mode switching and keyboard input.
- Live retained-scene reference captures show three/two/one columns at window widths 1180/950/760. Six cards and Processes fit at 1180 by 760. The revised graphics card uses compact model names and separate memory lines; a regression covers two adapters plus an additional count and delayed notice.
- Reference raster captures at scales 1, 1.5 and 2 are not physical Windows display-DPI acceptance. Actual 100/150/200% display scaling remains unverified.
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
- Native reports are retained as the run's `sd300-gui-interaction-*` artifacts. Remaining platform jobs must be evaluated as they finish; their presence does not imply success.

Responsiveness work remains owned by #r16, resources by #r17, and overview/release acceptance by #ov6. Complete physical DPI and composite installer qualification, exact-candidate CI, original performance gates and public-byte verification before describing this as released or deployed. Do not merge to main to trigger publication while these requirements remain open.
