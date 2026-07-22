# Handoff: SD-300 v3 Fable Changelog and Closure

**Date:** 2026-07-22
**Session:** Session 2 of the day
**Agent:** Codex
**Task/ticket ID(s):** milestone `#v3n`; active tasks `#nsp`, `#gux`, `#cpl`; queued tasks `#qv3`, `#giu`; draft PR #4

---

## Resume Here First

SD-300 v3 is not release-ready. Resume in this exact existing checkout; do not create a new branch or worktree unless the operator asks.

| Item | Current value |
|---|---|
| Repository and worktree | `C:\Users\hey\git\qube-system-diagnostics` |
| Branch | `codex/sd300-v3-native-gui` |
| Local documentation HEAD | This handoff/changelog/board commit at current `HEAD`; run `git rev-parse HEAD` |
| Prior local handoff commit | `0c0c92df5998b8ca0f7fd26f4123edead4e4e551` |
| Pushed product HEAD | `8aaeef7b1b3d009ba5d18f0e14e6cf57d20a09d0` |
| Upstream | `origin/codex/sd300-v3-native-gui` |
| Draft pull request | `https://github.com/QubeTX/qube-system-diagnostics/pull/4` |
| Task board | `http://127.0.0.1:4321/` at handoff; always re-read `.tasks/.board-server.json` |
| Board identity | `C:\Users\hey\git\qube-system-diagnostics\.tasks` |
| Board bundle | version `1.0.1`, full asset tier, shared hooks, project title `SD-300` |
| Original stalled Codex task | `019f7e10-080a-7f62-bdc0-37ef63a670ff` |
| Closest Goose/Honk reference task | `019f7c6f-93fc-7d91-9554-3ffded4a358b` |

The immediate end-user blocker is severe scrolling/input lag on scrollable pages after the installed GUI has been open for roughly one minute. The prior foreground and hidden processor/memory averages do not clear this. It is a release-blocking interaction defect with no controlled trace yet.

The immediate long-run blocker is an invalid soak. The exact app exited normally with code zero after roughly twenty-five minutes of a planned two-hour run, and the result JSON is empty. Find the source of the graceful exit before repeating the soak.

The immediate installer blocker is the two-file uncommitted product diff in:

- `src/update.rs`
- `scripts/validate-windows-self-update.ps1`

Those are the product files the previous session was editing. Do not discard them. They replace an interactive PowerShell receipt-parent deletion with nonrecursive empty-only deletion and add a byte-exact unrelated-sibling preservation check. They have not been parsed, tested, committed, pushed, or rerun in hosted Windows qualification.

## Session Narrative

The operator first asked this agent to take over and finish another Codex task in this repository that had run for more than fifteen hours and appeared stuck in a loop. The original work had already built most of an additive Native SDK desktop companion, shared Rust monitoring engine, six-target build system, and composite installer/updater lifecycle. This agent reconstructed the task from Git, the tracked board, local artifacts, physical Windows state, CI runs, the original task, and sibling products rather than restarting it.

The apparent loop had two concrete environmental causes: an unbounded actionlint/ShellCheck integration left six processes hung, and a still-attached UI automation observer contaminated GUI CPU measurements. After closing the observer, the unchanged app returned to the expected processor range. Those discoveries do not explain the operator's later minute-old scroll lag or the early clean soak exit.

The operator then asked for deeper CI diagnosis, Internet/GitHub research, a Terra x-high research agent, comparison with Honk300/Goose, and secondary comparison with TR-300 and ND-300. The previous handoff captured the broad research and product reconstruction. During this continuation, the Terra x-high installer reviewer returned and independently confirmed the PowerShell prompt cause and the conservative deletion model from primary sources. Its bounded hardening questions are recorded below.

The operator then asked for a Fable handoff, a fully current task board, an extensive explanation of that board for agents without the tasks plugin, all supplied Native SDK URLs, the branch/worktree/current-file details, and an explicit record of end-user performance trouble. Before pausing, the operator also asked for a partial changelog because the branch showed a very large change count.

This continuation therefore did documentation and continuity work only:

- expanded the technical unreleased changelog with the major GUI, renderer, lifecycle, CI, packaging, qualification, security, and open-blocker work;
- created a complete plain-English companion covering all eighteen technical release sections;
- added standing instructions that both changelogs move together;
- repaired/reopened the task board and reconciled its task details and milestone count;
- incorporated the research agent's installer findings;
- wrote this standalone Fable handoff.

No runtime fix was attempted in this continuation. The product diff remains exactly the two pre-existing installer files named above.

At the final pause, this handoff, both changelogs, both repository instruction files, and the reconciled task-board files are committed together at local branch `HEAD`. The branch is intentionally not pushed. The only remaining dirty files are the two product files awaiting Fable's review.

## The Plan and Where It Stands

1. **Reconstruct the stalled task and stop the loop — done.**
   - Exact branch, worktree, commits, CI evidence, local artifacts, and board state are known.
   - The old task should not receive more work.
2. **Finish the implemented native GUI and composite lifecycle — substantially implemented, not accepted.**
   - All nine diagnostic destinations and both audience modes exist.
   - Physical Corporate MSI and broad GUI interaction evidence exist.
   - Scroll/input latency, early clean exit, frame/input percentiles, and replacement soak are open.
3. **Close the CI failures — partially done.**
   - The large-output Windows test timeout was correctly scoped and fixed; exact-head ordinary CI is green.
   - The managed PowerShell uninstall prompt is diagnosed; a conservative local correction awaits review and qualification.
4. **Document the branch honestly — done for this checkpoint.**
   - Technical unreleased entry expanded.
   - Plain-English companion added with all eighteen release groups represented.
   - Changelog lockstep rules added to both agent instruction files.
5. **Keep the board and memory resumable — done for this checkpoint.**
   - Board bundle repaired at the current plugin version and launched at its identity-bound port.
   - Active/queued task status, activity, blockers, and milestone count reconciled.
6. **Release qualification and publication — not authorized.**
   - Font rights, signing/notarization, provenance, immutable release assets, public-byte checks, site verification, and final physical acceptance remain open after product blockers close.

## Files Changed in This Continuation

### Documentation and task-system changes intended for the focused handoff commit

- `CHANGELOG.md`
  - Expanded the unreleased section by roughly one hundred fifty lines.
  - Records the implemented desktop surfaces, rendering/performance work, packaging, lifecycle fixes, security boundaries, evidence already collected, and all known open qualification issues.
- `HUMAN_CHANGELOG.md`
  - New plain-English companion.
  - Three hundred five lines, one hundred fifty-five human-facing bullets, and eighteen release sections matching the technical release grouping.
  - The current-work section is clearly marked not released.
- `CLAUDE.md`
  - Added the lockstep technical/plain-English changelog rule.
- `AGENTS.md`
  - Added the same rule so Codex and other agents see it at repository entry.
- `.tasks/TASKS.md`
  - Preserves the board's move of `#giu` into To-Do behind `#qv3` and attaches it to `#v3n`.
- `.tasks/tasks/giu.md`
  - Reconciled its stale Backlog status with the board index while keeping it explicitly post-release.
- `.tasks/tasks/cpl.md`
  - Added the independent installer-research conclusion and three bounded hardening questions.
- `.tasks/tasks/gux.md`, `.tasks/tasks/nsp.md`, `.tasks/tasks/qv3.md`
  - Added the changelog/handoff pause and retained exact open blockers.
- `.tasks/milestones/v3n.md`
  - Corrected progress from zero of four to zero of five after the board attached the post-release update task.
- `docs/agents/handoff/2026-07-22-002-sd300-v3-fable-changelog-and-closure.md`
  - This document.

### Product changes that predate this continuation and must remain uncommitted

- `src/update.rs`
  - Uses nonrecursive empty-only directory deletion for the receipt parent and ignores the documented nonempty-directory outcome.
- `scripts/validate-windows-self-update.ps1`
  - Adds an unrelated sibling next to the receipt and requires byte-exact preservation through the real uninstall route.

Do not stage those two product files with the documentation/handoff commit. Review and qualify them as a separate product change.

## What Was Just Learned

### Managed PowerShell uninstall failure

The hosted failure is not an elevation problem. The cleanup removed the owned receipt, then asked PowerShell to remove its parent directory without recursion. A nonempty directory makes PowerShell ask whether it should recurse; the updater deliberately uses noninteractive Windows PowerShell, so that prompt becomes a terminating failure. SD-300 then correctly rolled back and restored the installed executable.

The research agent independently confirmed:

- `Remove-Item` still needs recursion for a nonempty directory; suppressing confirmation does not change deletion semantics.
- `.NET Framework Directory.Delete(path, false)` is available under Windows PowerShell 5.1 and removes only an empty directory.
- A nonempty Windows directory maps to Win32 error 145 and normally to HRESULT `0x80070091`; the staged low-word check sees 145.
- The current approach fails closed for permissions, locks, bad paths, and other I/O problems and does not follow a recursive delete through unrelated siblings.
- The existing hosted fixture is strong: it exercises the real uninstall, one-object JSON contract, receipt removal, exact unrelated-sibling byte preservation, and final empty-root cleanup.

Before qualification, Fable should decide three narrowly scoped hardening questions:

1. Require both the Win32 HRESULT facility and low word 145, rather than only testing the low word.
2. Replace silent receipt removal with noninteractive `.NET File.Delete`, which is already-successful if absent but exposes real access/lock failures.
3. Force the child PowerShell working directory to a trusted location outside the receipt tree and add a regression for a receipt root that is the current directory.

These are review questions. They are not justification for recursive deletion or broad error suppression.

### Large-output Windows test failure

The separate hosted failure involving one megabyte of PowerShell output was test timing, not a production collector regression. The test proves concurrent pipe draining beyond typical buffer capacity. A cold hosted PowerShell process exceeded a two-second normal-probe deadline, so that single capacity test now uses the existing slow-command budget. Production probe deadlines and the short-timeout cancellation test remain strict. Exact-head CI proves the scoped correction.

### Changelog scale

The branch is far larger than the visible “about 1,430” count suggested. Against `origin/main`, it is a major native-desktop and lifecycle branch spanning more than one hundred files and tens of thousands of inserted lines. The changelog is intentionally a partial progress record, not an assertion that every modified line is accepted or released.

## Current Evidence

- Exact-head ordinary CI `29892152552`: green across Rust/security and all six Native GUI target builds.
- Branch-safe release planning/build run `29892152578`: green; it did not publish anything.
- macOS package preflight `29892224525`: green on Intel and Apple Silicon; not physical signed/notarized acceptance.
- Windows Native Installers run `29892216141`: failed in managed PowerShell uninstall on the known interactive receipt-parent deletion; rollback restored the executable.
- Claude review run `29892152564`: failed externally before review and produced no code findings.
- Physical Alienware evidence covers all nine destinations, both modes, keyboard navigation, maximized scaling, redacted export, singleton focus, hidden startup, repeated tray close, launch-at-login add/remove, normal close, adjacent-engine self-test, real Cargo-to-Corporate takeover, rollback, repair, uninstall, export preservation, and exact old-Cargo restoration.
- Prior foreground and hidden average processor/memory samples pass the current average budgets.
- No valid two-hour soak, frame-time percentile, input-response percentile, scroll-lag trace, signed/notarized candidate, immutable release, or fresh-public-byte evidence exists.

## Performance Problems and End-User Behavior

### Severe minute-old scrolling/input lag

The operator reports that scrollable areas become “laggy as hell” after the end-user app has been open for roughly one minute. Treat that report as direct physical product evidence. It overrides any temptation to infer responsiveness from average CPU or memory.

Reproduce on the exact release-shaped bundle with no Codex Computer Use or UI Automation observer attached:

1. Open Drivers, Processes, or Network at a viewport that definitely overflows.
2. Record immediate scroll and keyboard response.
3. Leave the app visible and updating for at least ninety seconds.
4. Record input enqueue, message handling, model update, view construction, raster start/end, present, dirty regions, queue depth, history size, row counts, and scroll offsets.
5. Compare immediate and warmed states.
6. Isolate input delivery, message backlog, repeated model hashing, retained-tree growth, scroll-container invalidation, software raster, Windows presentation, or row/history growth.
7. Preserve the one-second foreground data contract; do not hide the defect by slowing collection.

Likely surfaces are hypotheses only: accumulated view/model work, retained renderer damage, repeated scroll-container invalidation, one-second message backlog, history/list recomputation, or input events waiting behind presentation. Measure before choosing.

### Invalid long soak

The planned two-hour Processes soak began on the exact final Windows app and ended normally after roughly twenty-five minutes. The harness correctly failed because the process did not survive the requested duration. There was no application crash event and no result JSON.

Read first:

- `target/performance-final-soak-20260721-233810.stderr.log`
- `target/performance-final-soak-20260721-233810.json`
- `scripts/measure-native-gui-performance.ps1` around the early-exit check
- GUI normal close, tray close, singleton message, lifecycle endpoint, startup-hidden, and test-automation cleanup paths

Add bounded close-source logging if the cause cannot be reconstructed. Do not rerun a two-hour test blindly.

### Observer contamination

A UI automation observer previously inflated GUI processor usage. Functional automation and performance measurement must remain separate: finish functional interaction, close the observer, verify it is gone, then sample the untouched release process. NVIDIA `nvidia-smi` may provide a supplementary GPU sanity sample if useful; it cannot clear CPU, frame, input, or memory budgets.

## Product and Release Decisions That Must Not Drift

- Bare `sd300` continues to open the existing terminal chooser. The desktop app is additive and launched only through `sd300 gui`.
- The terminal and desktop interfaces share collectors, not process state. The GUI owns its own Rust snapshot/runtime through a bounded C interface.
- CLI and GUI ship as one installed product; update, repair, and uninstall preserve the proven existing owner/channel.
- Existing Cargo-owned users intentionally update twice: first to receive the new CLI, then to perform the managed CLI-plus-GUI takeover.
- Foreground monitoring continues to present one-second data. Bounded latest-only projections are allowed; unbounded queues and fidelity reductions are not.
- Receipt files are owned; receipt parent directories may contain unrelated data. Never repair the prompt with recursive deletion.
- Exactly one final updater JSON object goes to standard output; progress belongs on standard error.
- `latest` is for discovery. Mutation uses exact-tag bytes with platform, architecture, kind, size, and checksum agreement.
- The six supported targets remain Windows x86-64, macOS Intel and Apple Silicon, Linux GNU x86-64 and ARM64, and Linux musl x86-64. Compilation feasibility alone does not add a new owned target.
- Makira's bytes being secret and checksum-verified is not embedding-license evidence. Require suitable App/Game rights or explicit authorization to replace it with a distributable open font.
- Build, hosted-native, physical UI, UAC, signing, notarization, attestation, public bytes, website, and long-lived install acceptance are separate evidence classes.
- Never retag, force-push a release tag, overwrite released bytes, or publish from this feature branch.

## Sibling Implementations and Research

### Honk300/Goose — closest comparator

Repository: `C:\Users\hey\git\goose`

Released reference: tag `v1.3.4`, commit `ee4d1f5fa135093591f802e828e86a0f03482bfc`

Read:

- `C:\Users\hey\git\goose\AGENTS.md`
- `C:\Users\hey\git\goose\docs\adr\0031-provenance-preserving-slot-self-update.md`
- `C:\Users\hey\git\goose\docs\adr\0038-cross-platform-tray-update-helper.md`
- `C:\Users\hey\git\goose\docs\readiness\v1.3.4-readiness.md`
- `C:\Users\hey\git\goose\src\install.rs`
- `C:\Users\hey\git\goose\src\update.rs`
- `C:\Users\hey\git\goose\.github\workflows\release.yml`

Reusable lessons:

- Treat the protected receipt as authoritative and fail closed on ambiguous ownership.
- Use immutable versioned payload slots and stable selectors so a running old image need not overwrite itself.
- Separate activation/receipt commit from deferred cleanup; cleanup failure can be pending without rewriting committed user intent.
- Resolve discovery first, then download and verify exact-tag bytes.
- Promote the same reviewed source through candidate CI, unchanged main, same-revision CI, one immutable tag, atomic publication, fresh-public-byte checks, and physical acceptance.

Do not copy Goose's product paths, receipt schema, slot layout, tray helper, or no-Cargo policy blindly. Its invariants are the useful part.

### TR-300 and ND-300 — secondary comparators

- TR-300: `C:\Users\hey\git\qube-machine-report`
- ND-300: `C:\Users\hey\git\qube-network-diagnostics`

Use them to cross-check origin preservation, transactional backup/rollback, stable wrappers, MSI committed-result handling, and conservative cleanup. Honk300 is closer because it also combines a running GUI, native lifecycle surface, and owner-preserving updater.

### Primary installer sources

- [PowerShell Remove-Item](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/remove-item?view=powershell-7.6)
- [Windows PowerShell 5.1 noninteractive mode](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_powershell_exe?view=powershell-5.1)
- [.NET Framework Directory.Delete](https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.delete?view=netframework-4.8.1)
- [.NET Framework File.Delete](https://learn.microsoft.com/en-us/dotnet/api/system.io.file.delete?view=netframework-4.8.1)
- [Win32 system error codes](https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-)
- [dotnet/runtime Win32 error mapping](https://github.com/dotnet/runtime/blob/main/src/libraries/Common/src/System/IO/Win32Marshal.cs#L25-L102)
- [dotnet/runtime Windows directory removal](https://github.com/dotnet/runtime/blob/main/src/libraries/System.Private.CoreLib/src/System/IO/FileSystem.Windows.cs#L250-L395)
- [Inno Setup uninstall deletion rules](https://jrsoftware.org/ishelp/topic_uninstalldeletesection.htm)
- [Makira typeface](https://yukitacreative.com/makira-sans-serif-family/)
- [Yukita Creative licensing FAQ](https://yukitacreative.com/faq/)
- [MyFonts Makira listing](https://www.myfonts.com/collections/makira-font-yukita-creative)

## Native SDK Documentation for Fable

Explore every official page below. They may describe a newer toolkit than this branch's distribution lock, so verify any recommendation against the locked toolchain, package lock, app manifest, build manifest, and reviewed renderer patch before changing dependencies.

1. [Introduction](https://native-sdk.dev/introduction)
2. [Quick start](https://native-sdk.dev/quick-start)
3. [App manifest](https://native-sdk.dev/app-zon)
4. [Skills](https://native-sdk.dev/skills)
5. [Native UI](https://native-sdk.dev/native-ui)
6. [Fonts](https://native-sdk.dev/fonts)
7. [Packaging](https://native-sdk.dev/packaging)
8. [Packaging and signing](https://native-sdk.dev/packaging/signing)
9. [Updates](https://native-sdk.dev/updates)
10. [Packages](https://native-sdk.dev/packages)

Questions to answer:

- Is sustained scroll/input latency or over-invalidation documented, especially for timer-driven models and scroll containers?
- Does the current toolkit expose input, frame, damage, queue, or presentation timing that can be used or carefully backported without changing the distribution lock?
- Are retained lists, histories, configured-entry hashing, or timers known to accumulate work?
- Does current packaging/signing guidance confirm the custom target-pinned wrappers or reveal a missing supported step?
- Can the official update model preserve every SD-300 owner and the two-step Cargo takeover, or does it only replace one app payload?
- Exactly how are embedded fonts copied or compiled into the package, and what licensing rights does that require?

## Known Issues and Limits

1. Severe minute-old scroll/input lag is operator-observed and release-blocking.
2. The long soak exited early with code zero and empty output; graceful-exit source unknown.
3. The managed PowerShell uninstall correction is local, untested, uncommitted, unpushed, and not hosted-proven.
4. Frame-time and input-response percentile evidence is missing.
5. Makira application-embedding rights are unproven.
6. Windows/Linux screen readers see a named canvas but not the GUI's internal controls; the terminal interface is the accessible fallback.
7. Physical Mac acceptance is unavailable; hosted compilation/preflight does not prove UI, accessibility, notarization experience, or long-lived lifecycle behavior.
8. The external Claude review check produced no independent review findings.
9. No immutable or public v3 release bytes exist.
10. Old actionlint/ShellCheck integration must not be rerun; use bounded parsing and separate shell checks.
11. UI automation must not remain attached during performance measurement.

## Task Board: Complete Operating Guide Without the Plugin

### What it is

This repository contains a self-contained, git-tracked SHAUGHV task and project-memory system under `.tasks/`. The plugin is convenient but not required: the Markdown, browser app, and zero-dependency Node server are committed in the repository.

The board is not a decorative to-do list. It is the release ledger and cross-session continuity layer. It prevents “CI passed” from erasing the distinction between code compilation, hosted native behavior, physical hardware, UAC, accessibility, performance, signing, licensing, provenance, public downloads, and long-lived installation behavior.

### Why this repository uses it

1. **Release honesty:** each evidence class remains a separate observable gate.
2. **Cold-session resumption:** active task status, verification, and activity tell a new agent exactly where to begin.
3. **Operator visibility:** the browser and agents read and write the same Markdown.
4. **Durable reasoning:** task details retain decisions, rejected alternatives, risks, acceptance, failures, and commands.
5. **Project memory:** hot context and deep project definitions survive chat/task boundaries.

### Files and what they track

```text
.tasks/
  TASKS.md                 authoritative task index and board columns
  MILESTONES.md            dated outcomes; tasks attach with an (ms #id) token
  tasks/<id>.md            exhaustive per-task handoff, verification, status, activity
  milestones/<id>.md       milestone status and archived completed child records
  CLAUDE.md                hot working memory: operator, terms, projects, preferences
  memory/glossary.md       durable terminology and ownership definitions
  memory/people/           durable people context
  memory/projects/         durable project architecture and release context
  memory/context/          durable organizational/environment context
  secure/                  local gitignored secrets/private notes; never commit contents
  config.json              board mode, title, hooks, and bundle version
  board-config.js          generated project title for browser/static use
  dashboard.html           browser board
  board-server.mjs         identity-bound live-sync server
  .board-version.json      committed bundle version
  .board-server.json       runtime port/PID/root identity; gitignored
```

### Current board configuration

- Title: `SD-300`
- Shared/git-tracked mode
- Bundle version: `1.0.1`
- Shared hooks: enabled in `.claude/settings.json`
- Asset install tier: full at this handoff
- Runtime identity at this handoff: port 4321, PID 10288, root `C:\Users\hey\git\qube-system-diagnostics\.tasks`

Never assume port 4317 or reuse another repository's port. A port is not identity.

### How to launch or repair it without the plugin

From the repository root:

```powershell
node .tasks\board-server.mjs status
node .tasks\board-server.mjs ensure --open
Get-Content -Raw .tasks\.board-server.json
```

Confirm the reported root is this repository's `.tasks` directory. If Node is unavailable, open `.tasks/dashboard.html` directly and select `.tasks/TASKS.md` plus the `.tasks` folder; static mode loses live two-way synchronization but the Markdown remains authoritative.

### Board grammar

- Preserve the existing columns and their order: Backlog, To-Do, Active, Done.
- A task is one bold Markdown checkbox line whose own bare `#id` is the final token.
- `(needs #id)` declares prerequisites; a blocked task must not move into Active.
- `(ms #id)` attaches at most one milestone.
- Indented checkbox rows are proper board-visible subtasks. They are flat; no sub-subtasks.
- Put exhaustive reasoning and evidence in `.tasks/tasks/<id>.md`, not on the one-line index.
- A detail file starts with a short `TT;DR:` and then records Why, Plan, Impact, Acceptance, Verification, Status, and Activity.
- `## Verification` contains observable `[ ]`, `[x]`, or `[~]` checks. An agent may waive only with a dated reason in both the item and Activity.
- A task cannot finish while a proper subtask or verification item remains open.
- A milestone cannot finish while any attached child task remains open.
- Attribute Activity entries on this shared board.

### Session-start procedure

1. Read `.tasks/TASKS.md` and `.tasks/MILESTONES.md`.
2. Read `.tasks/CLAUDE.md` and the relevant deep-memory files.
3. Read every Active task detail file in full.
4. Read the queued release task and open milestone detail.
5. Verify the live server's root and port.
6. Run `git status` before editing; browser changes may already be present.

### While working

1. Keep each Active task's `## Status` current with done/open/exact next action.
2. Tick verification only when observable evidence exists.
3. Append a timestamped attributed Activity entry after meaningful findings, edits, failures, state moves, or pauses.
4. Add a proper subtask when the operator needs a visible required step.
5. Use separate linked tasks for work large enough to need its own owner/status/history.
6. Keep the hot memory short; put durable detail in deep memory or task files.
7. Never store tokens, signing credentials, font bytes, private licenses, or secrets in tracked board or memory files. Use environment variables, OS keychain, or `.tasks/secure/`.

### Completion procedure

1. Check every proper subtask.
2. Pass or explicitly waive every verification item.
3. Mark the task checked, add its completion date, and move it to Done.
4. Append the final Activity entry.
5. Close a milestone only after all children are complete.
6. Before clearing an old Done task attached to a milestone, archive its line in the milestone detail so progress does not move backward.

### Current tracked work

- `#nsp` Active: pinned toolkit/shared engine/Corporate installer vertical slice. Published-old-binary terminal replay and final interaction/performance closure remain.
- `#gux` Active: complete desktop GUI. Minute-old scroll lag, graceful-exit attribution, frame/input percentiles, and replacement soak are explicit open gates.
- `#cpl` Active: all installer/update/repair/uninstall paths. Immediate resume point is independent review and focused testing of the two-file Windows correction.
- `#qv3` To-Do: final qualification and release, blocked by `#gux` and `#cpl`.
- `#giu` To-Do: post-release in-app/tray update coordinator, explicitly blocked by `#qv3`; do not pull it into release scope.
- `#v3n` Open milestone: zero of five children complete. This count includes the post-release follow-on for visibility.

### Memory model

- `.tasks/CLAUDE.md` is the hot cache: operator environment, key terms, active project status, and durable preferences.
- `.tasks/memory/projects/sd300-v3.md` is the durable v3 architecture, release order, evidence boundary, blocker, and reference-product record.
- `.tasks/memory/glossary.md` defines owner/channel, latest intent, observations, and platform language.
- Task detail files are also memory and are the most authoritative task-specific history.
- `.tasks/secure/` is local and ignored; it is the only board-owned place for private notes, though environment/keychain is preferred for secrets.

### Shared-board and Git behavior

- Browser stale-write protection works only on one machine; Git is the cross-machine synchronization layer.
- Pull/reconcile before long board sessions if someone else may have edited it.
- On same-task conflicts, keep the more truthful advanced state and union tokens.
- In detail-file conflicts, union and timestamp-sort Activity lines and merge unique current facts into Status.
- Commit meaningful board changes with the work they describe or in a focused documentation/handoff commit.

## Exact Next Actions

1. **Review the uncommitted product diff before touching anything else.**

   ```powershell
   Set-Location C:\Users\hey\git\qube-system-diagnostics
   git branch --show-current
   git status --short
   git diff -- src/update.rs scripts/validate-windows-self-update.ps1
   ```

2. **Resolve the three installer hardening questions from primary sources and code context.** Keep deletion nonrecursive and fail closed.
3. **Run focused installer validation.** At minimum:

   ```powershell
   [void][ScriptBlock]::Create((Get-Content -Raw scripts\validate-windows-self-update.ps1))
   cargo fmt --check
   cargo test managed_windows_cleanup_removes_integrations_when_gui_root_is_already_missing
   cargo test --locked
   git diff --check
   ```

4. **Commit and push the product correction separately** after review and local proof.
5. **Dispatch a new exact-head Windows Native Installers qualification** and require the unrelated-sibling proof. Run `29892216141` remains failure evidence only.
6. **Reproduce the minute-old scroll lag with warmed-state input-to-present instrumentation** and no observer attached.
7. **Attribute the early clean exit**, then run a replacement two-hour soak only after the close source is known.
8. **Capture explicit frame and input percentiles** outside deliberate scans.
9. **Resolve font rights** with App/Game embedding evidence or operator-approved open-font replacement.
10. **Only after every prerequisite passes:** independent review, unchanged merge to main, same-revision qualification, fresh immutable tag, unpublished signed/notarized candidate, attestations, crate/latest promotion, fresh-public-byte checks, website verification, and final physical Windows acceptance.

## Resume Test and Final State

A fresh agent should be able to resume without the prior transcript by reading this file, `AGENTS.md`, `.tasks/TASKS.md`, `.tasks/CLAUDE.md`, and the three Active task details. The exact next product action is review of the two-file Windows cleanup diff. The highest-priority product investigation is the minute-old scroll/input lag, followed by graceful-exit attribution and replacement soak.

**Exit state:** Directed. The work is neither complete nor blocked on vague information; the next evidence-producing actions are named.

**Confidence:** High for repository/board/CI/changelog state and PowerShell prompt cause; medium for the proposed hardening details until tested; low for all scroll-lag and early-exit causal hypotheses because no controlled trace exists.

**Sanity check:** Passed. This handoff does not claim release readiness, does not treat green ordinary CI as physical acceptance, preserves the uncommitted product diff, and puts direct end-user evidence ahead of average performance metrics.

**Strongest dissent considered:** The green six-target builds and passing average CPU/memory samples might suggest the branch is close enough to publish and handle interaction problems later. That is rejected because the operator can reproduce severe lag in the actual client, the long soak is invalid, one installer lane still fails, and licensing/signing/public-byte evidence is absent.
