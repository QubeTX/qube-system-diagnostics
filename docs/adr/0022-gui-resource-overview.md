# ADR 0022 — Resource cards and overview subscriptions

Status: Accepted for implementation by the operator on 2026-09-27; unpublished candidate.

## Decision

The native Overview shows equal-size cards in GPU, CPU, memory, disk, network,
thermal order, with stronger emphasis on the first three. It exposes bounded
process leaders and expandable findings/system evidence. User mode targets
gamers without inventing FPS, latency, throttling, or health assessments.

Extend ADR 0006's narrower GUI Overview profile to Fast/Static/Slow/Activity/Health.
Fast captures CPU, memory, network and the complete process inventory. Select the
CPU and memory leaders before bounding the internal JSON projection. Preserve
existing cadences, hidden-mode behavior, independent TUI sessions, public schemas
and fixed-layout C ABI. Connections, diagnostics and drivers remain unsubscribed.

Prepare bounded card copy outside rendering. Choose the fullest fixed volume
(mount-name tie break, removable fallback), and stable GPU identity order rather
than activity order. Label aggregate network scope, unavailable fields and old
captures explicitly. Shared finding identity and severity cross into the GUI;
presentation does not create new diagnoses or weaken collector observations.

## Verification

Profile-selection, complete-inventory leader, native projection/layout/navigation,
fresh-launch and resource tests qualify the candidate. Existing performance
ceilings remain unchanged. The operator authorized a new release after validation
on 2026-09-27; local proof does not satisfy the complete native platform and
installer qualification.
