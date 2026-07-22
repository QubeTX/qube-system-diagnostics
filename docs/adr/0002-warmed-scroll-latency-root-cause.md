# ADR 0002 — Warmed-state scroll latency: root cause and fix ladder

Date: 2026-07-22
Status: Accepted
Related: `.tasks/tasks/gux.md`, `gui/src/tests.zig` (warmed-state bench), `gui/patches/native-sdk-0.5.4-software-render.patch`, `gui/src/main.zig`

## Context

The operator reported severe scroll/input lag on scrollable GUI sections after
the installed app had been open for roughly one minute, while all average
CPU/memory budgets passed (15-min foreground 1.58%, 30-min hidden 0.18%).

## Evidence (headless warmed-state benchmark, ReleaseFast, patched renderer)

A new `SD300_RENDER_BENCH`-gated benchmark drives fully populated Network,
Processes, and Drivers sections cold vs. warmed (all ten 60-sample histories
filled) at scroll offsets 0 and 420, plus 16-frame scroll bursts, with byte-exact
correctness gates against a memoless reference render:

- Steady 1 Hz data ticks cost 0.27–1.7 ms. The worst case is the warmed Network
  tick (0.320 → 1.693 ms): once histories fill, shift-left `pushHistory` changes
  all 60 samples every second and `chartDataEqual` invalidates the full chart —
  but it stays on the sparse damage path (4 rects, ratio 0.402).
- EVERY scroll frame is a full-viewport repaint: the `<scroll>` container bakes a
  `-scroll_y` translation into layout, so advancing the offset moves essentially
  every command; the diff yields exactly one viewport-sized dirty region
  (rects=1, ratio 1.000) → the sparse multi-region path is architecturally
  unable to engage → union fallback renders all 1180×760 pixels. Cost:
  15.8–17.0 ms average, 18–22 ms p95/max — at or above the 16.7 ms frame
  budget, identical on chartless sections and identical cold vs. warm.
- The render command memo is defeated by translation (position-keyed): scroll
  frames show hundreds of command misses; stable-position ticks show zero. The
  position-independent glyph-mask cache still hits ~73% mid-scroll — the cost is
  gradient/fill re-raster, not text.
- The sparse damage path fired in only 4 of 15 scenarios; even two small
  far-apart changes fall back because canonical regions must span the scissor.
  Fallback per se is cheap (0.3 ms at ratio 0.04); cost tracks dirty ratio.

## Root cause

Scroll frames were always expensive (~16 ms full-viewport software repaints,
independent of warm state). Input (wheel/trackpad) events can arrive faster than
that service rate, and input shares one single-threaded update queue with the
1 Hz data tick. Before ~60 s, idle ticks are effectively free and the scroll
train runs alone; after the histories fill, every second injects a mandatory
chart repaint into the same queue. The combination pushes service time past the
arrival rate, the queue backs up, and the user perceives severe scroll lag —
"fine at first, laggy after about a minute."

## Decision (fix ladder)

1. Bound the queue (primary, correctness): coalesce scroll input in the runtime
   so a burst of scroll events costs one repaint at the latest offset per
   render cycle, and process pending input before starting an expensive paint.
   Implemented through the reviewed downstream SDK patch (already-pinned files)
   with lockstep SHA-256 pin updates through both patch preparers.
2. Keep warmed-tick work sparse (already true; no change required by evidence).
3. In reserve if physical testing still shows lag: blit-scroll the retained
   surface (translate pixels, repaint only the exposed band) to cut the ~16 ms
   floor itself. Not taken first: substantially more invasive raster surgery.

Rejected: raising the 8-dirty-rect cap (does not help — scroll produces ONE
full-viewport rect, not many; and it would touch serialization formats).
Rejected: lowering data cadence or history fidelity (contract violation).

## Consequences

- The warmed-state benchmark stays in `gui/src/tests.zig` as the regression
  harness; scroll-burst p95 and damage-mode attribution are now measurable on
  every build.
- Physical warmed-scroll acceptance (2+ minutes open, then scroll) remains the
  operator-facing gate; average CPU metrics are proven insufficient for
  interaction latency.
