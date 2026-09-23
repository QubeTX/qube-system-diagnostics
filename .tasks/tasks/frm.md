TT;DR: Explore whether formal modeling can reveal lifecycle and concurrency defects in a future SD-300 version. This is an idea to evaluate, not part of v4 qualification.

## Why

On 2026-09-23 the operator shared a post about Lean and TLA+ for finding race conditions, then explicitly deferred exploration to a later version.

## Scope

Begin with one small, implementation-linked model of collector cancellation, shutdown and latest-value delivery. Consider update rollback or measurement arithmetic only after the first experiment produces useful evidence. Do not add a v4 release gate or replace native runtime qualification.

## Plan

Define the concrete invariants and environmental assumptions, choose a suitable verifier, map its transitions to the implementation, and turn any reproducible counterexample into a regression test. Record the model's bounds and limitations.

## Impact

May expose execution orderings that ordinary tests miss. A checked abstraction does not establish correct OS behavior, physical measurement accuracy, performance, or correspondence with the implementation by itself.

## Acceptance

Report whether the bounded experiment found actionable defects, which properties were checked, and what remains unproved. Decide on broader adoption only from that evidence.

## Verification

- [ ] Select a bounded future-version experiment and explicit assumptions
- [ ] Run the verifier and retain reproducible results
- [ ] Map results to production code and regression tests where applicable

## Status

BACKLOG. Deliberately deferred beyond v4 by the operator; no formal-verification work or dependency installation is scheduled for the current release.

## Activity

- 2026-09-23 — codex: captured the operator's future-version interest after reviewing the proposed approach. Current v4 performance and lifecycle qualification continues unchanged.
