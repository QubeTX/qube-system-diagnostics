# Next-version targets: preserve the original SD-300 goals

Recorded at the operator's explicit request on 2026-09-23. These are the goals
for the version after 4.0.0, including targets temporarily relaxed for v4.
Owner: Codex; progress is tracked by #r16 (responsiveness), #r17 (resources),
and the existing physical-device/accessibility tasks. This document does not
claim that every target has already been achieved.

## Performance acceptance

| Measurement | Original target, restored after 4.0.0 | Temporary 4.0.0 ceiling |
|---|---:|---:|
| Foreground CPU, CLI/TUI/GUI including owned children | ≤2% of one logical core | ≤4% |
| Hidden GUI CPU including owned children | ≤1% of one logical core | ≤3% |
| Peak summed working set / RSS | ≤150 MiB | ≤200 MiB |
| Private memory / commit | ≤300 MiB | Unchanged |
| Frame-work p95 | ≤16.7 ms | ≤100 ms |
| Input receipt-to-presentation p95 | ≤50 ms | ≤100 ms |
| Ordinary refresh maximum | ≤100 ms | Unchanged |

The exceptions in ADRs 0016/0017 apply to **4.0.0 only**, including on macOS.
The test policies automatically restore the original limits for every other
version, including 4.0.1. Preserve original verdicts as well as release-policy
verdicts; a waived overrun remains an overrun against the long-term target.

Measure release builds with exact GUI/engine/CLI identities and native runtimes.
Use aligned before/after workloads and the same methodology. Include terminated
owned helpers in CPU accounting. Measure resources with no visual observer,
debugger, competing build or UI automation attached. Retain foreground, hidden
and two-hour soak evidence, clean shutdown, memory trends, handle/descriptor and
child counts. Instrumented frame/input tests run separately. Record startup,
collector duration, missed samples and input/render latency; do not invent
additional numeric limits for measures that had none in the accepted plan.

## Measurement accuracy and data availability

- Use monotonic elapsed intervals and stable device/process identities. First
  samples, resets, replacement, resume and PID reuse must not fabricate spikes.
- Preserve unavailable, unsupported, denied, error and contradictory fields.
  Never turn inaccessible data into measured zero or repeat stale samples as new.
- Keep DNS duration separate from actual ICMP RTT; blocked ICMP alone is not
  proof of unavailable internet. Keep actual units, source and capture age visible.
- Sort/filter complete inventories before selecting visible rows. Timestamp
  histories and preserve gaps; never splice different sensors into one history.
- Expand trustworthy native provider coverage deliberately. Validate readings
  against independent OS counters over aligned windows with recorded tolerances.
  Parser fixtures and same-driver comparisons do not prove physical accuracy.

## Reliability and cross-platform parity

- Keep input/render paths free of slow collection. Use bounded latest values,
  one active provider, skipped overdue ticks, finite histories and retry backoff.
- Bound helper execution, output, inherited-pipe draining and cancellation;
  terminate owned descendants and join workers before GUI engine unload.
- Surface provider failures and recovery with actual freshness. Distinguish
  pressure, hardware-reported faults and incomplete observations centrally.
- Preserve Windows x86-64, both macOS architectures, Linux GNU x86-64/ARM64,
  and Linux musl x86-64. Both frontends consume the same diagnostic truth while
  retaining independent sessions, mutable state and settings namespaces.
- Keep native accessibility and duplicate-input fixes, complete keyboard
  operation, readable compact/wide layouts, truthful pause-view, reduced motion,
  ASCII/no-color fallbacks and responsive search/selection/inspection.

## Optional tools, compatibility and release evidence

- ND-300 remains independently installed and explicitly invoked. Preserve
  partial results, warning/failure outcomes, bounded cancellation and supported
  public-interface compatibility. Never trigger repairs or speed tests through
  a diagnostic refresh. SpeedQX retains its separate time/data and M-Lab consent.
- Install helpers or elevate bounded read-only probes only after explicit
  consent. Preserve existing ownership and useful unprivileged monitoring.
- Retain schema-1 defaults, nullable schema-2 measurements, shared redaction,
  bare CLI chooser, lifecycle/exit contracts and terminal restoration.
- Maintain deterministic failure fixtures and real PTY/native tests on all six
  targets. Qualify install, update, rollback, repair and uninstall as a composite
  CLI plus branded clickable application. Preserve settings and optional-tool
  ownership. Verify public bytes, checksums, attestations, crate and stable routes.
- Exercise the actual last public updater across engine ABI/schema changes as
  well as synthetic-prior fixtures. Public 4.0.0 verification found the real
  3.1.3 ABI-1 rejection and a Windows PowerShell update-check defect; preserve
  the recovery instructions and track correction/qualification in #p4h.
- Keep unavailable physical-device, screen-reader and tray acceptance explicit;
  hosted runs do not replace physical-device proof. Formal Lean/TLA+ exploration
  remains a separate future-version task (#frm), as requested by the operator.

## Baseline evidence

The [v4 qualification record](qualification/v4/README.md) retains complete
native timing/resource results, earlier failures, the Windows soak and exact
candidate identities. Use the final public release manifest to connect those
candidate measurements to the released product; do not label candidate runs as
measurements of newly downloaded public bytes.
