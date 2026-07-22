TT;DR: Physical macOS acceptance of the released v3 product when Mac hardware access returns: installed app behavior, notarization experience, tray/status item, accessibility, and PKG lifecycle on real Intel and/or Apple Silicon.

## Why
Hosted macOS builds, preflight, and PKG qualification cannot prove user-facing app behavior, Gatekeeper/notarization experience, or long-lived lifecycle on real hardware. Temporary Mac access ended before v3.

## Plan
Install the public PKG on real hardware; verify `sd300` TUI and `sd300 gui`, status-item lifecycle, launch-at-login, update/repair/uninstall through the managed shell path, redacted export, and basic VoiceOver behavior (named canvas + documented TUI fallback).

## Impact
Converts hosted-only macOS evidence into physical acceptance.

## Acceptance
The documented physical checklist passes on at least one real Mac.

## Verification
- [ ] Physical install/update/uninstall lifecycle passes
- [ ] GUI interaction and status-item behavior pass
- [ ] Notarization/Gatekeeper first-run experience is clean

## Status
Backlog. Blocked on hardware access; not blocked on #qv3 once released bytes exist.

## Activity
- 2026-07-22 12:35 — created by Fable from the standing handoff limitation (agent: fable)
