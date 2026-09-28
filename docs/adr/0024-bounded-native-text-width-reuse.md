# 0024 — Bounded native text-width reuse

Date: 2026-09-27

Status: Accepted for candidate implementation; native performance qualification pending.

## Context

The production macOS captures in CI 36362469153 show repeated intrinsic layout
visiting native text measurement. Even the host's existing width cache constructs
Objective-C string keys and resolves registered fonts on each call. The evidence
does not attribute every slow frame to this work. Small-panel shadow removal is
measured separately in CI 36364119149 before evaluating measurement reuse.

## Decision

Extend the reviewed Native SDK patch with an opt-in width cache at the existing
TextMeasureProvider seam. Runtime-owned providers opt in; ad-hoc providers retain
their uncached behavior. Reuse actual provider measurements, never replace them
with estimated widths. Font registration, runtime creation and appearance changes
already invalidate the shared text-measure generation.

Each measuring thread lazily owns 256 direct-mapped entries, each holding at most
256 text bytes. A hit requires the exact bytes, font ID, float-size bits, provider
function, context and current generation. Collisions replace entries and cannot
return another label's width. Longer text bypasses the cache. Invalid provider
results use the existing estimator without caching it, allowing immediate recovery.
Storage remains bounded, with no per-frame growth or inventory sorting.

Ship changes through the canonical patch and both verified build preparers. Keep
the upstream npm integrity and Zig content hash unchanged because the upstream
archive is unchanged; update the independent downstream patch/source hashes in
the toolchain lock and preparers together. Never modify an SDK cache in place.

## Verification and consequences

Regression tests exercise repeated labels, mutable text storage, fonts, sizes,
provider contexts, invalidation, opt-out, provider failures and long labels.
Strict bindings and all native interaction cohorts remain required. Native timing
and resource thresholds are unchanged. A local reduction in provider calls alone
is not proof that the release performance matrix passes.
