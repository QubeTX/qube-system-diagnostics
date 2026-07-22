TT;DR: Improve GUI screen-reader accessibility beyond the named Native SDK canvas, or document the boundary permanently with the TUI as the accessible fallback.

## Why
Native SDK 0.5.4 exposes the GUI as one named canvas without an internal widget tree to Windows/Linux screen readers. The unchanged TUI is the honest, documented fallback today.

## Plan
Track Native SDK releases for accessibility-tree support (the SDK ships ~2-3×/week; the distribution lock pins 0.5.4 — an upgrade is a deliberate, fully-requalified change). If support lands, wire roles/labels for the nine sections and controls; if not, keep the documented fallback current in README/gui/README.

## Impact
Serves screen-reader users natively instead of via the TUI fallback.

## Acceptance
Either an accessible widget tree ships, or the boundary is re-documented against the then-current SDK with a dated re-evaluation.

## Verification
- [ ] SDK accessibility capability evaluated against a newer release
- [ ] Implementation shipped or boundary re-documented

## Status
Backlog. Not release-blocking; revisit alongside any SDK version change.

## Activity
- 2026-07-22 12:35 — created by Fable from the standing platform limitation (agent: fable)
