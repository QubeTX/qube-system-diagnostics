# ADR 0011 — Draw opaque panels without retaining duplicate pixels

Date: 2026-09-23
Status: Accepted; whole-product and native-platform qualification remains open
Related: ADR 0002, task #v4a, `gui/src/tests.zig`

## Evidence

The completed 09b46a3 whole-process-family Windows foreground measurement passes
CPU/private memory but exceeds the 150 MiB working-set limit at 153.44 MiB.
Role samples identify retained GUI allocation as one recoverable cost.

The old warmed benchmark bypassed the production static-chrome cache. Comparing
its timings to production would be invalid. The corrected benchmark captures
the prefix count immediately after the same chrome builder, calls the runtime's
`renderPassDamageWithStaticPrefix` path with frame dirty regions, and labels the
old generic path as diagnostic. Before/after use ReleaseFast, x86_64-windows,
baseline CPU and tracing off. Scroll bursts now exercise 128 bounded forward and
reverse movements, with a byte-exact full-redraw check after each burst.

The production path exposed an existing pixel mismatch on both old and new
panel implementations. Shadow-occlusion scissors met at fractional coordinates;
outward pixel rounding could blend boundary pixels twice. Round the guaranteed
opaque core inward to whole pixels before splitting its visible perimeter.
The corrected candidate passes the full-view comparisons and reports local
scroll raster p95 below 10 ms in all three measured sections. This measures
raster time, not the complete OS frame/input path or product resource budgets.

## Decision

Extend ADR 0002's fix ladder with direct drawing of opaque, uniformly colored
rectangles and rounded panels, and skip provably untouched solid border
interiors. These panels can replace interior destination
pixels directly, without hashing their background or retaining a second copy of
the panel. Keep the existing rounded-edge coverage calculation and square-corner
pixel-center rule. Use disjoint whole-pixel shadow scissors. Translucent fills, gradients, shadows and other expensive
background-dependent drawing retain the bounded command memo.

Apply the change through the canonical downstream SDK patch and both checked
preparers. The upstream npm integrity and Zig content hash still identify the
unchanged official archive; the separately pinned patch hash and patched-file
hash identify this change. Never edit the installed SDK or dependency caches.

## Qualification

A constant-gradient fixture exercises the existing per-pixel path as an
independent comparison for solid fills. Compare every output byte across scale,
fractional placement, clipping, rounded corners, changed backgrounds, alpha and
opacity, plus thin and thick borders. Assert that opaque panels allocate no command-pixel entries while
translucent panels keep their memo behavior. Also retain the full-view sparse
damage and scroll comparisons against a memoless reference. A separate small
shadow fixture checks fractional position/scale, clipping and translucent fills
without requiring the opt-in performance benchmark.

Repeat the unchanged release-mode benchmark after terminating other builds and
observers. Then measure a complete CLI/engine/GUI bundle over the same foreground
window, retaining exact artifact hashes and per-role memory. Native macOS and
Linux runs must separately verify runtime behavior and budgets. A successful
pixel comparison or microbenchmark does not establish the product resource gate.

Blit scrolling remains in reserve. This change does not alter sampling cadence,
input coalescing, visual design, cache caps or acceptance thresholds.
