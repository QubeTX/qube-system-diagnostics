# ADR 0004 — v3.0.0 release-scope decisions: functional bar now, evidence bar deferred deliberately

Status: Accepted (2026-07-22). Records operator-directed scope decisions made
during the v3.0.0 release drive; written with the v3.1.0 release that closed
out the same session.

## Context

The v3 native-GUI effort ran 24+ hours under a prior agent without converging
(see `docs/agents/2026-07-22-codex-loop-post-mortem.md`). When this session
took over, implementation was substantially complete but several maximalist
release gates remained open: a two-hour soak, exhaustive GUI automation,
formal frame/input percentiles, published-v2 PTY replay, and a font-license
evidence question. The operator resolved the deadlock by explicitly separating
the *functional bar* (works, launches, updates, uninstalls cleanly) from the
*evidence bar* (exhaustive formal proof), directing release on the former with
the latter tracked as owned post-release work.

## Decisions

1. **Pre-release soak waived; post-release soak owned.** The two-hour soak was
   not required for v3.0.0 (operator, 2026-07-22). The prior soak's early exit
   was attributed to an operator window close, not a defect (ADR 0001). A
   machine-quarantined released-bytes soak with exit attribution is Backlog
   task #sok.
2. **Makira ships under operator authority.** The operator purchased the face
   from Craftwork Design; no license document was provided with the purchase.
   The operator explicitly directed shipping it. Obtaining embedding-license
   evidence (or substituting an open font) is Backlog task #mkl. Do not strip
   or substitute the font silently.
3. **Exhaustive testing moved to owned backlog, not dropped.** GUI automation
   sweeps, PTY replay, physical interaction regression, and formal performance
   re-proof live in Backlog #ext; robustness hardening in #hrd. Waivers on the
   board are dated and attributed — evidence-class honesty is preserved.
4. **In-app updates deferred to v3.1.0 (task #giu).** The coordinator carves a
   deliberate exception to "update never launches the app" and belonged in its
   own qualified release, not bundled into the main one (see ADR 0005).
5. **Success criteria carry a renegotiation path.** Overly tight board gates
   with no waiver escape valve were a compounding cause of the original loop.
   Criteria now carry an owner; agents surface "this gate is expensive,
   ship-blocking, and waivable" to the operator instead of grinding.

## Rejected alternatives

- **Holding the release for the full evidence bar** — the loop had already
  demonstrated this converges on never shipping; the operator explicitly chose
  fix-forward patches over pre-release exhaustiveness.
- **Silently dropping the waived gates** — every waived item became a tracked,
  owned backlog task instead; a build/CI pass is still never claimed as
  physical, soak, license, or public-byte acceptance.

## Consequences

- v3.0.0 published 2026-07-22 15:02 UTC (59 assets, live crate, three verified
  attestations, physical Windows MSI acceptance) roughly one day after this
  scope reset — the functional bar was reachable; the evidence bar was not the
  release contract.
- Future agents inherit the two-bar model: state which bar a claim satisfies,
  and renegotiate gates with the operator rather than looping on them.
