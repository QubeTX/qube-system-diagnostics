TT;DR: Replace the rejected ECG-style SD-300 artwork with the operator-selected isometric SD/300 identity, then fix every supported package so application and tray icons are embedded, loaded, installed, updated, and removed correctly.

## Why
This is a direct operator order. The operator's installed Windows copy shows the generic Windows application and tray icons even though the previous v3 task recorded icon work as complete. The existing amber ECG/heart-monitor mark is also explicitly rejected as overcomplicated, generic, and not a distinctive SD-300 identity.

Read-only reconstruction on 2026-07-23 established two independent failures:

- The managed v3.1.1 updater did install `app/assets/icon.png`, and its hash matches the repository asset.
- `sd300-gui.exe` has no embedded Windows icon resource, so shell surfaces extract the generic executable icon.
- Native SDK's Windows host passes the same PNG path to Win32 `LoadImageW(..., IMAGE_ICON, ...)`; that API expects an ICO and falls back to the generic application icon.
- The window class does not assign `hIcon`/`hIconSm` or send `WM_SETICON`.

The completed v3 release remains historical fact; this new task corrects the artwork and delivery defect without rewriting that release.

## Scope
In scope:

- The operator-selected Quiver application mark and simplified companion tray glyph.
- Deterministic Windows/macOS/Linux derivatives from those supplied masters.
- Windows executable resource embedding, runtime window/tray loading, installer packaging, managed update ownership, and conservative uninstall cleanup.
- Local build/package/runtime evidence on Windows and non-physical package checks for macOS/Linux.
- Lockstep technical and human changelogs.

Deferred / out of scope:

- Publishing a new public release or replacing the installed public v3.1.1 channel.
- Redesigning the GUI, changing the CLI/TUI contract, or adding diagnostic functionality.
- Physical macOS/Linux acceptance beyond the existing release process.

## Plan
1. Preserve the supplied Quiver SVG/PNG masters and derive deterministic platform assets with the pinned Native SDK generator.
2. Embed the application ICO through a Win32 resource, split application/tray runtime paths by platform, and extend the reviewed Native SDK patch so Windows sets large/small window icons and loads the dedicated tray ICO.
3. Include all runtime assets in native archives, WiX/Inno, managed manifests, updater ownership checks, and uninstall cleanup; reconcile pinned patch hashes.
4. Run strict/model tests, GUI tests, Rust tests, Windows release build, distribution-lock checks, package/manifest checks, WiX lanes, associated-icon extraction, runtime taskbar/Alt+Tab inspection, programmatic tray dispatch, and the manual operator tray visual check.

## Design reasoning
**Empathize:** The primary user is looking for SD-300 in a crowded Windows taskbar, Start surface, Alt+Tab switcher, or tray. They need instant recognition at 16–32 px and a mark that makes the product feel deliberate rather than like a generic monitoring utility.

**Define:** Create an original SD-300 object/symbol with one memorable silhouette and one reusable negative-space idea. The application badge may carry depth and the Warm Carbon environment; the tray companion must survive as a stripped-down glyph.

**Research:** The rejected icon uses a dense grid plus ECG line. QubeTX's family mark uses disciplined isometric geometry and luminous edge treatment. SD-300's GUI uses near-black surfaces, off-white structure, and `#FF5E1A` orange. The new mark should inherit the discipline and palette without copying either symbol.

**Divergence — concepts considered before evaluation:**

1. A faceted diagnostic core opened along an offset seam, exposing a hot internal plane.
2. Three asymmetric scan shutters locking around a square negative-space target.
3. Three offset chassis rails whose gaps form a compact hidden S without literal lettering.
4. A topology knot built from three broken paths around a central void.
5. A cutaway rotor/turbine with one missing blade revealing the monitored core.
6. Worst possible idea: another ECG line on a grid or gauge face; explicitly rejected because it repeats the operator's complaint.

**Convergence:** Keep concepts 1, 2, and 3 for ImageGen. They have different outer silhouettes, preserve the app/tray simplification path, avoid diagnostic clichés, and can be expressed with few vector shapes. The strongest dissent is that an abstract monogram may be less immediately “diagnostic”; the response is to prioritize unique product recognition over a pictogram that merely describes monitoring.

## Impact
Intended:

- SD-300 gains a distinctive identity that reads clearly at native Windows icon sizes.
- Explorer, Start, taskbar, Alt+Tab, window chrome, installer/ARP, and tray use intentional artwork instead of a generic fallback.
- Managed update and uninstall treat the new files as owned product payloads.

Possible unintended:

- Changing the pinned Native SDK patch can disturb all native targets if its staged hashes are not reconciled.
- ICO resource or path mistakes can fix Explorer while leaving the runtime window or tray generic.
- Excessive visual detail can disappear at 16–24 px even when the 1024 px source looks strong.
- macOS template behavior can collapse a multi-color glyph if its alpha silhouette is not designed separately.

## Acceptance
**Functional bar:** The selected application identity appears in the built Windows executable and running application surfaces, while the simplified companion appears in the Windows tray with its menu/lifecycle intact. The managed candidate package installs, updates, and uninstalls every new owned icon asset.

**Evidence bar:** Image-scale comparison at 16/24/32/48/256/512 px; Native SDK strict checks and tests; Rust tests; Windows release build; associated-icon extraction; local runtime taskbar/Alt+Tab inspection; programmatic tray dispatch plus operator visual confirmation; archive/WiX/managed-manifest validation; synthetic prior update/uninstall proof; macOS/Linux package icon validation.

**Gate ownership:** The operator owns visual selection and the manual tray appearance check. Repository release policy owns build, package, update, and uninstall evidence. Public release and physical non-Windows acceptance are explicitly deferred.

## Verification
- [x] Operator selected and supplied the final application and tray masters.
- [x] Generated SVG/PNG/ICO assets pass required-size, alpha, silhouette, and deterministic-export checks.
- [ ] Native strict checks, GUI tests, Rust tests, Windows release build, distribution-lock validation, archive validation, and WiX builds pass.
- [ ] Built and synthetic-installed Windows candidates expose the selected associated icon; taskbar/Alt+Tab and dedicated tray icon are visibly non-generic; update/uninstall lifecycle passes.

## Status
ACTIVE — the combined v3.1.2 candidate now passes local Rust, engine, clippy, strict/model, icon-regeneration, Native SDK, release-target Windows, associated-icon, archive-manifest, and Inno installer-build checks plus PR #7's full hosted core/native/security/cargo-dist matrix. The staged executable visibly shows the selected app mark in Windows chrome; close-to-tray/reopen and close-to-quit behavior passed against isolated candidate settings without replacing public v3.1.1. Remaining gates are release-chain WiX and synthetic-prior managed update/uninstall qualification, native macOS/Linux package publication proof, manual Windows tray appearance/tooltip/Quit confirmation, and release integration.

## Activity
- 2026-07-23 01:08 — created in Active from the operator-approved implementation plan; discovery subtask marked complete from current installed/repository evidence (agent: codex)
- 2026-07-23 01:09 — upgraded the tracked board from 1.0.1 to 1.0.2 and relaunched it on its identity-bound port (agent: codex)
- 2026-07-23 01:10 — completed the critical-thinking Design divergence: six concepts including the intentionally bad ECG/gauge route were considered; faceted core, scan shutter, and hidden-S chassis directions advanced to separate ImageGen drafts (agent: codex)
- 2026-07-23 01:18 — incorporated operator feedback that the first concepts were too abstract; generated and displayed three grounded revisions using recognizable technical objects while retaining the Warm Carbon system. No Quiver spend has occurred (agent: codex)
- 2026-07-23 01:26 — operator rejected the second round as still too abstract; generated and displayed three fully representational SD-300 service scenes with recognizable computers and diagnostic equipment. No Quiver spend has occurred (agent: codex)
- 2026-07-23 01:31 — operator supplied the missing style constraint: simple flat isometric vector. Generated and displayed three new drafts built from a few large Warm Carbon polygons with realistic detail, lighting, and scene complexity removed. No Quiver spend has occurred (agent: codex)
- 2026-07-23 01:41 — received and inspected the missing Quiver screenshot, replacing the incorrect metaphor assumptions with the intended S-left / D-right / 300-top isometric construction; generated and displayed four independent flat-vector interpretations. No Quiver spend has occurred (agent: codex)
- 2026-07-23 04:02 — imported the operator-selected app SVG/PNG and tray SVG, preserved the originals, and generated deterministic multi-resolution ICO/ICNS/PNG/hicolor assets plus a monochrome macOS tray template with the pinned SDK icon generator (agent: codex)
- 2026-07-23 04:27 — embedded the multi-resolution ICO into the Windows executable, split app/tray runtime paths, added theme-adaptive Windows tray variants and macOS template loading, updated WiX/Inno/managed archive ownership and uninstall checks, and reconciled the 21-file downstream Native SDK patch through both preparers (agent: codex)
- 2026-07-23 04:58 — Windows release build passed with distribution locks; release-target Native tests passed 37/39 with two expected skips; associated-icon extraction returned a 32 px icon containing 152 selected-orange pixels; candidate manifest hash-verified all five Windows identity assets (agent: codex)
- 2026-07-23 05:04 — refreshed the Shaughv-Code Codex marketplace and confirmed the installed/enabled plugin is current at v0.36.1; no newer marketplace snapshot was available (agent: codex)
- 2026-07-23 05:16 — rebuilt the release candidate after the v3.1.2 version convergence: root/engine tests, clippy, strict/model checks, 37/39 release-target Native tests, distribution locks, and path-leak checks passed; the GUI self-test reported 3.1.2 and associated-icon extraction found 164 selected-orange pixels (agent: codex)
- 2026-07-23 05:18 — packaged the 3.1.2 Windows native ZIP and verified the byte-level SHA-256 of app-icon.png, app-icon.ico, tray-icon.ico, tray-icon-dark.ico, and tray-icon-light.ico against install-manifest.json; Global and Corporate Inno installers compiled while ingesting all five assets (agent: codex)
- 2026-07-23 05:22 — ran the staged candidate with isolated settings: the selected icon is visible in Windows title chrome, default X hides the window while the process remains alive, a singleton Open restores it, and opting out makes X terminate the process; public v3.1.1 and its saved preferences were not replaced (agent: codex)
- 2026-07-23 05:36 — PR #6 first matrix made the platform boundary explicit: Windows and both macOS GUI targets passed, while all Linux targets exposed a missing libc declaration on the clean full-analysis object. Added the Linux-only declaration, reconciled patch/content hashes, and re-passed both preparers plus the 37/39 local Native suite before the corrective push (agent: codex)
- 2026-07-23 03:53 — GitHub left PR #6's synthetic head ref pinned to the pre-fix commit even though the source branch contained `e44430a`; closed/reopened it to prove the stale-ref cause, then moved the exact candidate to replacement PR #7 and cancelled the duplicate stale-PR workflows (agent: codex)
- 2026-07-23 03:54 — PR #7 CI run 29991988139 passed every substantive job on the corrected candidate: core Windows/macOS/Linux, security, cargo-dist planning, Windows native GUI and payload packaging, both macOS native targets, and Linux GNU x86-64, GNU ARM64, and musl x86-64. The PR-only release source/plan run 29991988153 also passed (agent: codex)
