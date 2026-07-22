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
| [0003](0003-managed-uninstall-receipt-parent-cleanup.md) | Managed uninstall removes the receipt parent only when empty | Accepted |

Planned: 0002 (warmed-state scroll latency root cause and fix ladder — lands with
the fix), 0004 (v3.0.0 release-scope decisions — lands with the release).
