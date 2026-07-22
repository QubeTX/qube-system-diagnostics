TT;DR: TASK FOR CODEX. After v3.0.0 is public, run the extensive testing and performance sweep that the operator deliberately waived out of the release under the functional-bar directive, and feed anything found into patch releases.

## Why
Operator direction (2026-07-22): ship v3.0.0 at the "loads, works, functions" bar and do touch-ups and performance updates afterward. Every heavy gate that was waived with a dated reason lands here so nothing is silently dropped.

## Plan
1. Exhaustive GUI automation coverage: every section, both audience modes, keyboard paths, scaling, exports, and unavailable/denied/unsupported states. Use Computer Use for GUI acceptance freely EXCEPT tray interactions — tray behavior gets programmatic dispatch plus a manual operator check (Computer Use cannot see the Windows tray; see the post-mortem).
2. Published-v2 binary PTY replay on hosted targets (the waived `nsp` item).
3. Physical interaction regression sweep on released bytes: scroll granularity per wheel notch, warmed-state scroll smoothness, minimize/restore freshness, tray-toggle close semantics, singleton focus.
4. Formal foreground (15-min, <=2%) and hidden (30-min, <=1%) budget re-proof on released bytes, no observers attached.
5. Varied-load performance regression checks (background load, many processes/connections, driver scans during interaction) and memory-growth verification.
6. Coordinate with #sok (two-hour soak + frame/input p95) — run them in one quarantined session where practical.

## Impact
Converts the release-day waivers back into proven evidence and drives the first patch releases.

## Acceptance
Every waiver recorded in `gux`/`nsp` verification on 2026-07-22 is either re-proven on released bytes or has a filed defect driving a patch release.

## Verification
- [ ] Exhaustive automation coverage complete with archived evidence
- [ ] PTY replay green on hosted targets
- [ ] Physical interaction sweep clean or defects filed
- [ ] Formal foreground/hidden budgets re-proven on released bytes
- [ ] Varied-load and memory-growth checks clean or defects filed

## Status
Backlog. Blocked on #qv3. Context: the waivers live in `.tasks/tasks/gux.md` and `.tasks/tasks/nsp.md` (dated 2026-07-22), ADRs 0001-0003 in `docs/adr/`, and the Codex post-mortem in `docs/agents/`.

## Activity
- 2026-07-22 05:20 — created at operator request as the consolidated home for all functional-bar testing waivers plus performance regression work; assigned to Codex post-release (agent: fable)
