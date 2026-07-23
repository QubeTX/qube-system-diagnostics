# Architecture Decision Records

Numbered, immutable decision records for SD-300. Each ADR captures one decision:
its context, the evidence behind it, the decision itself, rejected alternatives,
and consequences — so future agents and contributors can trace *why* without
re-deriving the investigation. The convention follows the QubeTX/Goose family
pattern (`docs/adr/NNNN-title.md`).

Rules:

- ADRs are append-only history. Supersede with a new ADR; never rewrite an old
  one to say something different.
- Land each ADR in the same commit as the work it justifies whenever possible.
- Cross-reference task board detail files (`.tasks/tasks/<id>.md`) and both
  changelogs; the ADR holds reasoning, the board holds status, the changelog
  holds user-facing narrative.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-soak-early-exit-attribution.md) | Soak early-exit attribution: operator window close, not a product defect | Accepted |
| [0002](0002-warmed-scroll-latency-root-cause.md) | Warmed-state scroll latency: full-viewport scroll frames + queue collision | Accepted |
| [0003](0003-managed-uninstall-receipt-parent-cleanup.md) | Managed uninstall removes the receipt parent only when empty | Accepted |
| [0004](0004-v3-release-scope-decisions.md) | v3.0.0 release scope: functional bar now, evidence bar deferred deliberately | Accepted |
| [0005](0005-in-app-update-coordinator.md) | In-app updates spawn the CLI as a detached coordinator; the GUI never mutates the installation | Accepted |

## Dual-frontend contract pointer

The rules for changing SD-300 across its two frontends — what the TUI and native
GUI share, what they keep separate, and the step-by-step recipe for wiring a
field into both — live in the **Dual-frontend editing model** section of
`CLAUDE.md` and `AGENTS.md` (identical in both). Read that before opening an ADR
about collector, projection, or parity work.

ADRs [0001](0001-soak-early-exit-attribution.md),
[0002](0002-warmed-scroll-latency-root-cause.md), and
[0003](0003-managed-uninstall-receipt-parent-cleanup.md) are the current worked
examples of the evidence discipline these records demand: each traces a decision
to measured or reproduced evidence rather than asserting a conclusion. ADR 0004
(v3.0.0 release-scope decisions) joins them at release closure.
