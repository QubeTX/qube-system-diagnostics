# Handoff: SD-300 v3 Native GUI, CI, Installer, and Performance Closure

**Date:** 2026-07-21
**Session:** Session 1 of the day
**Agent:** Codex
**Task/ticket ID(s):** milestone `#v3n`; active tasks `#nsp`, `#gux`, `#cpl`; queued release task `#qv3`; draft PR #4

---

## Resume Here First

This work is not release-ready. The exact checkout is:

| Item | Value |
|---|---|
| Repository/worktree | `C:\Users\hey\git\qube-system-diagnostics` |
| Git branch | `codex/sd300-v3-native-gui` |
| Pushed product HEAD | `8aaeef7b1b3d009ba5d18f0e14e6cf57d20a09d0` |
| Local HEAD at pause | Focused `docs(handoff)` commit directly above `8aaeef7`; run `git rev-parse HEAD` for its local SHA |
| Upstream | `origin/codex/sd300-v3-native-gui` |
| Draft PR | `https://github.com/QubeTX/qube-system-diagnostics/pull/4` |
| Board | `http://127.0.0.1:4321/` |
| Board source | `.tasks/TASKS.md` |
| Exact final Windows app | SHA-256 `d83de90b2bf05a3f23d755ac0fb8d9d9b6a4b98acbe44011fc6f7292797acaa2` |
| Exact final Windows engine | SHA-256 `cbd79f8c1205a6aeb4e4b6c4a2e7a4d6f60c86fe90228947e831e1e325da6d7c` |

The immediate product blocker is the operator's physical observation: after the installed/end-user GUI has been open for roughly one minute, scrolling through sections with scrollable content becomes severely laggy. This has not been reproduced under controlled instrumentation and is not cleared by prior average-CPU results. Treat it as release-blocking interaction latency.

The second immediate performance blocker is that the attempted two-hour Processes soak did not complete. It started at 2026-07-21 23:38, the GUI ran as PID 4260, reached approximately 31.06 CPU seconds, 86.41 MiB working set, and 226.18 MiB private memory, then exited with code 0 at approximately 00:03. The harness threw at `scripts/measure-native-gui-performance.ps1:257`. There was no Windows Application Error event. The result JSON is empty:

- `target/performance-final-soak-20260721-233810.json`
- `target/performance-final-soak-20260721-233810.stderr.log`

Do not simply restart the soak first. Determine what requested the app's graceful exit and reproduce the minute-old scroll lag with input-to-present instrumentation. A steady-state soak can pass while the actual end-user interaction remains unusable.

The immediate installer blocker is an uncommitted two-file managed-uninstall fix:

- `src/update.rs`
- `scripts/validate-windows-self-update.ps1`

Review these before editing anything else. They replace a prompting `Remove-Item` call on a potentially nonempty receipt parent with a nonrecursive empty-directory deletion and add an unrelated-sibling preservation fixture. The change has not been parsed, tested, committed, pushed, or rerun in hosted Windows qualification.

## Session Narrative

The operator asked this session to inspect and finish the other Codex task in this repository. That prior task, titled **Vercel Native SDK** (thread `019f7e10-080a-7f62-bdc0-37ef63a670ff`), had run for more than 15 hours and appeared trapped in loops. Its original goal was to add a native cross-platform GUI companion to SD-300 while preserving the existing CLI/TUI, make every existing diagnostic surface available in the GUI, package CLI plus GUI together, and ensure current users receive the GUI through the established owner-preserving update paths.

This session reconstructed the task from the tracked board, worktree, old task, commits, local artifacts, physical machine state, and GitHub Actions. It did not restart the implementation. The apparent loop had two concrete causes:

1. Six `actionlint.exe` processes hung through ShellCheck integration instead of completing the intended bounded workflow audit.
2. A Codex Computer Use/UI Automation observer remained attached during GUI performance measurements and substantially contaminated CPU results. Closing the observer returned the unchanged app from a false 5.39% reading to approximately 1.61% of one logical core.

After reconstructing state, the session finished the interrupted local Windows lifecycle work, reran the exact Cargo/MSI rollback and successful takeover paths, physically exercised the native GUI, committed the accumulated implementation, opened/updated draft PR #4, and investigated new hosted failures instead of treating them as generic CI flakiness.

The operator then explicitly asked for deeper CI analysis, internet/GitHub research, use of a Terra x-high research agent, comparison with the latest Honk300 v1.3.4 work in `C:\Users\hey\git\goose`, and secondary comparison with TR-300 and ND-300. The closest reference proved to be Honk300's provenance-preserving, immutable-slot updater/release model. A Terra x-high subagent was started for independent Windows installer research, but the operator requested this handoff before it returned; it was interrupted and produced no final report. The authoritative research captured below came from official Microsoft/.NET documentation, dotnet/runtime source, local proof, and direct examination of the Goose repository and thread.

Finally, the operator asked for a comprehensive Fable handoff, an entirely current task board, and a pause. During handoff capture the supposedly active two-hour soak was discovered to have exited normally and left empty output. The operator also supplied the severe minute-old scroll-lag observation. Both are now explicit board blockers, not hidden in prose.

## The Plan & Where It Stands

1. **Reconstruct the interrupted task, board, worktree, local artifacts, and evidence — done.**
   - Read `.tasks/TASKS.md`, `.tasks/MILESTONES.md`, `.tasks/CLAUDE.md`, and all active detail files.
   - Read the prior Codex thread without opening or mutating it.
   - Identified the loop causes, exact branch, dirty state, completed proofs, and remaining gates.
2. **Audit and finish the Cargo/MSI transaction and physical Windows lifecycle — done for the previously known paths.**
   - Exact legacy Cargo rollback restores binary, receipt, `.crates.toml`, and `.crates2.json` bytes.
   - Successful Corporate MSI takeover, missing-engine repair, GUI launch/focus/export, supported uninstall, unrelated export preservation, and exact v2.0.6 fixture restoration passed locally.
3. **Freeze and push the reconstructed implementation — done.**
   - Commit `b08b22556b16550e5af806e123064c03b9f87320` (`Complete native GUI lifecycle qualification`).
   - Commit `8aaeef7b1b3d009ba5d18f0e14e6cf57d20a09d0` (`Stabilize large-output collector test on CI`).
   - Draft PR #4 exists.
4. **Analyze and close exact-head CI failures — in progress.**
   - Ordinary Windows CI timeout was diagnosed and fixed; exact-head CI is green.
   - Managed Windows uninstall prompt was diagnosed; a safe two-file correction is staged but untested/unpushed.
   - The external Claude review workflow continues to fail before performing a review; it has produced no code findings.
5. **Close user-visible GUI performance and interaction gates — in progress and newly blocked.**
   - Required 15-minute foreground and 30-minute hidden average CPU/memory gates passed on the prior exact release-shaped build.
   - The two-hour replacement soak failed to complete because the GUI exited normally early.
   - Severe scrolling lag after about one minute is operator-observed and uninvestigated.
   - Frame-p95 and input-p95 evidence is still open.
6. **Complete license, signing, provenance, immutable release, and public-byte acceptance — not started because prerequisites remain open.**
   - Do not merge, tag, publish the crate, publish release assets, or change `latest` yet.

## What Was Accomplished

- Pushed implementation commit `b08b225` with the following changed files. Use `git show --name-status b08b225` for the exact frozen diff:
  - `.github/workflows/linux-native-gui.yml`
  - `.github/workflows/macos-installer.yml`
  - `.github/workflows/release-qualify.yml`
  - `.github/workflows/windows-installers.yml`
  - `.tasks/TASKS.md`
  - `.tasks/milestones/v3n.md`
  - `.tasks/tasks/cpl.md`
  - `.tasks/tasks/gux.md`
  - `.tasks/tasks/nsp.md`
  - `.tasks/tasks/qv3.md`
  - `CODEX_PROJECT.md`
  - `README.md`
  - `docs/thinking/2026-07-21-native-gui-local-release-gate.md`
  - `gui-engine/src/lib.rs`
  - `gui/README.md`
  - `gui/app.zon`
  - `gui/src/app.native`
  - `gui/src/main.zig`
  - `gui/src/platform/window_visibility.zig`
  - `gui/src/platform/window_visibility_linux.c`
  - `gui/src/platform/window_visibility_macos.m`
  - `gui/src/projection.zig`
  - `gui/src/tests.zig`
  - `scripts/build-native-gui.ps1`
  - `scripts/managed-installers/sd300-installer.ps1`
  - `scripts/managed-installers/sd300-installer.sh`
  - `src/cli.rs`
  - `src/migrate.rs`
  - `src/settings.rs`
  - `src/update.rs`
  - `wix-corporate/corporate.wxs`
  - `wix/main.wxs`
- Fixed the hosted Windows test `collectors::command_tests::command_helper_drains_output_larger_than_a_pipe_buffer` in commit `8aaeef7` by using the existing 7.5-second `CommandTimeout::Slow` budget instead of the two-second normal probe deadline. The test validates pipe draining under hosted load; it does not define a two-second product collection cadence.
- Exact-head GitHub evidence at `8aaeef7`:
  - CI `29892152552`: success across Rust/security and the six Native GUI contract targets.
  - Release workflow `29892152578`: success for its branch-safe planning/build behavior; it did not publish a release.
  - macOS universal package preflight `29892224525`: success on Intel and Apple Silicon preflight. This is not physical signed/notarized Mac acceptance.
  - Windows Native Installers `29892216141`: failure in **Exercise real same-channel version transitions**, reproducing the managed PowerShell uninstall prompt described below.
  - Claude review `29892152564`: failed before review with no findings, zero meaningful review output, and no code verdict.
- Physical Alienware evidence now includes:
  - All nine GUI destinations.
  - User and Technician modes.
  - Keyboard navigation.
  - Maximized scaling.
  - Redacted export.
  - Singleton focus/open behavior.
  - Hidden startup and same-PID reopen.
  - Repeated close-to-tray.
  - Launch-at-login add/remove.
  - Restored default close behavior and ordinary exit.
  - Exact legacy Cargo rollback, successful Corporate takeover, repair, supported uninstall, export preservation, and original v2.0.6 restoration.
- Updated the task system for Fable:
  - Added an explicit GUI subtask for the minute-old scroll/input lag.
  - Added a concrete `<=50 ms` input-p95 verification gate outside scans.
  - Recorded the early soak exit and empty result.
  - Recorded exact CI run IDs and the uncommitted installer correction.
  - Added `.tasks/memory/projects/sd300-v3.md`.
- Staged, but did not verify or commit, the managed-uninstall correction:
  - `src/update.rs`: after removing the owned receipt, call `[IO.Directory]::Delete(parent,$false)` and ignore only Win32 error 145 (`ERROR_DIR_NOT_EMPTY`), preserving unrelated siblings without a prompt or recursive deletion.
  - `scripts/validate-windows-self-update.ps1`: create an unrelated sibling beside the receipt, require exact byte preservation via Base64 equality, remove the fixture, assert no unexpected owned remnants, then delete the now-empty fixture root.

## Current Uncommitted Worktree

The board, memory, and this handoff are committed together as a focused local `docs(handoff)` commit. It is intentionally not pushed, so the remote product/PR head remains the proven `8aaeef7`. At handoff, exactly these product files should remain modified and uncommitted; they are intentional and must not be discarded:

- `scripts/validate-windows-self-update.ps1`
- `src/update.rs`

The two product files require Fable's independent review and verification before a product commit. Preserve the branch and working directory exactly. Start with:

```powershell
Set-Location C:\Users\hey\git\qube-system-diagnostics
git branch --show-current
git status --short
git diff -- src/update.rs scripts/validate-windows-self-update.ps1
```

## Key Decisions

- **The CLI/TUI remains unchanged and authoritative.** Bare `sd300` still opens the existing chooser. `sd300 gui` is the only additive public launch command.
- **CLI plus GUI is one installed product, but not one runtime.** The GUI dynamically loads its bundle-relative Rust engine and owns its own snapshot/runtime. The TUI keeps its existing process, state, keybindings, cadence, and output contracts.
- **Keep the six-target contract unchanged.** Windows ARM64 is not added merely because a toolchain can compile it. The v3 release contract remains Windows x86-64; macOS Intel/Apple Silicon; Linux GNU x86-64/ARM64; Linux musl x86-64.
- **A release build must preserve one-second foreground data.** Do not hide a renderer or queue problem by reducing collector fidelity. Latest-only bounded presentation and subscription-specific collection are allowed; unbounded queues are not.
- **Do not recursively delete a receipt parent.** The receipt file is owned; the directory may contain unrelated user or tool state. `Remove-Item -Recurse` would turn a CI fix into an ownership violation.
- **The staged empty-directory cleanup ignores only the documented nonempty outcome.** `[IO.Directory]::Delete(path,$false)` removes only an empty directory. Win32 `ERROR_DIR_NOT_EMPTY` is 145. Other I/O/access failures must still fail the transaction rather than be silently swallowed.
- **Installer rollback remains strict.** The hosted failure correctly returned nonzero and restored the installed executable. Do not weaken rollback or turn the noninteractive PowerShell failure into a warning.
- **Performance averages do not prove interaction.** The scroll-lag report means the current foreground CPU/memory results are necessary but insufficient. Input-to-present latency, frame p95, queue growth, and the one-minute warm state must be measured directly.
- **Keep measurement observers out of release performance.** Codex Computer Use/UI Automation materially changed CPU. Use automation for functional acceptance, close it, then run isolated performance collection.
- **Makira bytes are not license evidence.** The source face is a commercial Yukita Creative Makira build. CI secrets can reconstruct bytes but cannot prove app embedding/redistribution rights. Do not ship until the operator supplies App/Game license evidence or authorizes a distributable open-font replacement.
- **Release integrity follows the proven family pattern.** Reviewed source reaches `main` unchanged, then receives a fresh immutable tag. Never retag, force-push a tag, replace released bytes, or call a draft/preflight/publication complete before fresh public-byte checks.
- **Use Honk300 as a pattern source, not a code transplant.** Its ownership, immutable-slot, exact-manifest, deferred-cleanup, and release-gating invariants are valuable; its product paths, receipts, tray helper, and no-crates.io choices are not automatically SD-300's contract.

## How the Current Product Works

### GUI/runtime

- `gui/src/main.zig` owns the Native SDK `Model`, message update loop, settings, bounded histories, and engine bridge.
- `gui/src/app.native` is the compiled declarative view hierarchy.
- `gui-engine/` exposes a panic-contained, caller-buffer C ABI over the shared Rust collectors.
- The GUI loads `sd300_engine.dll`, `libsd300_engine.dylib`, or `libsd300_engine.so` from an absolute bundle-relative path.
- Projection state is bounded/latest-only. No Rust references, allocations, panics, or borrowed buffers cross the ABI.
- Windows/macOS can hide to tray when enabled. Linux SDK 0.5.4 has no tray and exits normally. Startup-hidden is explicit and never inferred on Linux.

### Composite lifecycle

- Native MSI/EXE, managed PowerShell, managed shell, and macOS PKG paths stage and verify both CLI and GUI payloads.
- Update preserves the proven owner/channel. A fresh official installer represents the user's latest intent.
- Existing Cargo-owned v2 users intentionally update twice: first to obtain the v3 CLI, then again for the managed CLI+GUI takeover.
- Lifecycle mutation asks the GUI to exit through the authenticated endpoint, verifies the companion, and rolls back exact ownership files when the transaction has not committed.
- Uninstall removes only proven product-owned files/integrations and preserves user exports and unrelated siblings.

### Performance history and present failure

- Early rendering was very expensive because the SDK used full software-pixel presentation, repeated command-list replay, and development-default event tracing.
- Implemented mitigations include bounded damage regions, stable fragments/glyph coverage, a persistent Windows DIB/memory DC, release `trace=off`, a fixed-layout process-summary ABI, sampler/ranking buffer reuse, direct envelope serialization, and a repeating timer.
- Proven prior release-shaped measurements:
  - 15-minute foreground Processes: 1.58% of one logical core; 84.78/90.36 MiB average/max working set; 227.2/232.74 MiB average/max private memory.
  - 30-minute hidden: 0.18%; 64.98/66.84 MiB working set; 206.65/208.81 MiB private memory.
- These results do **not** clear the new scroll-lag issue. The likely investigation surfaces include accumulated Native SDK model/view work, histories, scroll-state invalidation, timer/message backlog, retained renderer damage, row/table projection churn, and input events waiting behind one-second updates. These are hypotheses, not findings.
- The failed soak's exit code 0 suggests an orderly close/quit rather than a process crash. Investigate window/lifecycle messages, named quit endpoint ownership, stray automation cleanup, and harness/app logs before assuming a renderer crash.

## CI Failure Analysis

### 1. Ordinary Windows test timeout — fixed and proven

Failure:

- `collectors::command_tests::command_helper_drains_output_larger_than_a_pipe_buffer`
- 106 of 107 tests passed.
- A large PowerShell output fixture exceeded the two-second `CommandTimeout::Normal` deadline on a hosted Windows runner.

Why it was not a production regression:

- The test proves both stdout and stderr drain without a pipe deadlock.
- Its PowerShell payload is deliberately large and was measured around 2.8 seconds under load.
- Production probe deadlines were not changed.

Fix:

- Commit `8aaeef7` assigns the existing 7.5-second `CommandTimeout::Slow` budget to this large-output test on Windows and Unix.
- Exact-head CI `29892152552` passed, including Windows tests, release build, and target checks.

### 2. Managed Windows uninstall prompt — root cause proven, fix unproven

Failure:

- Exact-head Windows installer run `29892216141`.
- Job step: **Exercise real same-channel version transitions**.
- Channel: `powershell-installer` uninstall.
- Windows PowerShell error: `Windows PowerShell is in NonInteractive mode. Read and Prompt functionality is not available.`
- SD-300 returned one JSON object with exit 2, `requires_user_action: true`, and `the installed executable was restored`.

Root cause:

- `windows_managed_cleanup_commands` removed `install-receipt.json` and then emitted `Remove-Item -LiteralPath '<receipt parent>' -Force -ErrorAction SilentlyContinue`.
- The parent was nonempty. PowerShell's `Remove-Item` on a nonempty directory without `-Recurse` asks for confirmation. `-Confirm:$false` does not change the requirement to specify recursion. In noninteractive mode, prompting fails.
- Adding `-Recurse` is unacceptable because the parent is not proven exclusively owned.

Staged correction:

- Use `[IO.Directory]::Delete(parent,$false)`.
- Ignore `DirectoryNotFoundException`.
- For `IOException`, ignore only low-word HResult 145 (`ERROR_DIR_NOT_EMPTY`); rethrow everything else.
- Hosted fixture must create and exactly preserve an unrelated sibling in the receipt root.

Local evidence:

- PowerShell 7/.NET on this Windows host returned `System.IO.IOException`, HResult `-2147024751`, low word `145`, and preserved the nonempty directory/file.
- Attempts to invoke nested Windows PowerShell 5.1 through the shell wrapper were blocked by the command policy, so hosted Windows PowerShell 5.1 is still the authoritative runtime proof after push.

### 3. Claude review workflow — external/tool failure, not a code finding

- Multiple runs fail before producing review turns or findings.
- Latest exact-head example: `29892152564`.
- Do not treat it as a green review, but do not misreport it as a discovered code defect.
- Fable is being asked to provide the missing independent product/code review.

### 4. Performance soak — invalidated by clean early exit

- The JSON result is zero bytes.
- Stderr ends at `scripts/measure-native-gui-performance.ps1:257` with `SD-300 GUI exited during measurement with code 0.`
- No related process remained and no Application Error event matched `sd300-gui` or `sd300_engine`.
- This is not a passing partial sample. The harness intentionally fails if the app exits before duration.

## Goose, TR-300, and ND-300 Comparison

### Honk300 v1.3.4: primary comparator

Repository: `C:\Users\hey\git\goose`
Relevant Codex thread: `019f7c6f-93fc-7d91-9554-3ffded4a358b`
Released v1.3.4 commit/tag: `ee4d1f5fa135093591f802e828e86a0f03482bfc` / `v1.3.4`

Read these first:

- `C:\Users\hey\git\goose\AGENTS.md`
- `C:\Users\hey\git\goose\docs\adr\0031-provenance-preserving-slot-self-update.md`
- `C:\Users\hey\git\goose\docs\adr\0038-cross-platform-tray-update-helper.md`
- `C:\Users\hey\git\goose\docs\readiness\v1.3.4-readiness.md`
- `C:\Users\hey\git\goose\src\install.rs`
- `C:\Users\hey\git\goose\src\update.rs`
- `C:\Users\hey\git\goose\.github\workflows\release.yml`

Reusable principles:

- The protected receipt is authoritative; do not guess owner from a path when evidence is ambiguous.
- Windows uses immutable versioned payload slots and stable selectors, so a running old image does not have to overwrite itself.
- Activation/receipt commit is separate from deferred cleanup. Cleanup failure can become `cleanup_pending`; it does not erase or reinterpret the newly committed user intent.
- `latest` is discovery only. Mutation downloads exact-tag bytes and requires platform, architecture, kind, size, and SHA-256 agreement.
- Updater JSON is exactly one final stdout object; progress belongs on stderr.
- Candidate exact SHA -> unchanged `main` -> same-SHA CI -> one immutable tag -> atomic publication -> fresh-public-byte lanes -> physical install is a disciplined sequence, not one generic “CI passed” claim.
- Honk300 v1.3.4 ultimately passed exact candidate `29890799498`, same-SHA main CI `29891307523`, atomic publication `29891756284`, all fresh-public-byte lanes, production page verification, and installed Windows acceptance before being called complete.

Important caution:

- Goose's task/readiness files were briefly stale after publication while its thread moved into a possible v1.3.5 UI patch. Use the actual tag/commit and GitHub runs as release truth.
- Goose's specific receipt schema, immutable slot layout, no-crates.io decision, tray helper, and product aliases are not automatically SD-300 requirements.

### TR-300 and ND-300: secondary comparators

- TR-300: `C:\Users\hey\git\qube-machine-report`
- ND-300: `C:\Users\hey\git\qube-network-diagnostics`
- Use them to cross-check installer-origin preservation, transactional backups, rollback, stable wrappers, and conservative cleanup.
- Honk300 remains closer because SD-300 v3 also combines an installed GUI runtime with an owner-preserving updater and native control-surface concerns.

## Official Research Used

Use primary sources for this technical boundary:

- PowerShell `Remove-Item`: https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.management/remove-item?view=powershell-7.6
  - A nonempty directory requires `-Recurse`; `-Confirm:$false` does not turn a nonrecursive delete into an empty-only operation.
- .NET `Directory.Delete`: https://learn.microsoft.com/en-us/dotnet/api/system.io.directory.delete?view=net-9.0
  - `Directory.Delete(path, false)` deletes only an empty directory; nonempty directories throw `IOException`.
- Win32 system error codes: https://learn.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-
  - `ERROR_DIR_NOT_EMPTY` is 145 (`0x91`).
- dotnet/runtime `FileSystem.Windows.cs`: https://github.com/dotnet/runtime/blob/main/src/libraries/System.Private.CoreLib/src/System/IO/FileSystem.Windows.cs
  - Windows directory removal explicitly handles `ERROR_DIR_NOT_EMPTY`.
- dotnet/runtime `Interop.Errors.cs`: https://github.com/dotnet/runtime/blob/main/src/libraries/Common/src/Interop/Windows/Interop.Errors.cs
  - Defines `ERROR_DIR_NOT_EMPTY = 0x91`.
- Yukita Creative Makira page: https://yukitacreative.com/makira-sans-serif-family/
- Yukita Creative licensing FAQ: https://yukitacreative.com/faq/
- MyFonts Makira listing: https://www.myfonts.com/collections/makira-font-yukita-creative
  - Treat these as the current licensing boundary: app embedding requires suitable rights; a desktop font license is not sufficient evidence.

## Native SDK Documentation Map for Fable

Explore and use all of these official pages. They may document a newer SDK than this branch's locked `@native-sdk/cli` 0.5.4 and Zig 0.16.0, so verify each recommendation against `gui/toolchain-lock.json`, `gui/package-lock.json`, `gui/build.zig.zon`, `gui/app.zon`, and the reviewed renderer patch before changing code or packaging.

1. Introduction: https://native-sdk.dev/introduction
2. Quick start: https://native-sdk.dev/quick-start
3. `app.zon`: https://native-sdk.dev/app-zon
4. Skills: https://native-sdk.dev/skills
5. Native UI: https://native-sdk.dev/native-ui
6. Fonts: https://native-sdk.dev/fonts
7. Packaging: https://native-sdk.dev/packaging
8. Packaging/signing: https://native-sdk.dev/packaging/signing
9. Updates: https://native-sdk.dev/updates
10. Packages: https://native-sdk.dev/packages

Questions these pages should help answer:

- Is there a current documented cause or mitigation for scroll/input latency after sustained model updates?
- Are scroll containers invalidating too much of the software-rendered surface?
- Are there version-specific timer, retained-node, history, list/table, damage, or trace behaviors relevant to Native SDK 0.5.4?
- Does the current SDK offer an official input/frame timing hook that can be backported or used without changing the distribution lock?
- Does `app.zon`/packaging/signing guidance change any of the custom wrappers, or merely confirm the existing target-pinned composite flow?
- Does the official updates model solve only app payload updates, or can it preserve SD-300's multiple existing installer owners and Cargo migration contract? Do not replace proven owner logic without full parity.
- What exact font licensing/package behavior does Native SDK perform, and does it copy/compile font bytes into the application?

## Known Issues & Limitations

1. **Severe minute-old scroll lag:** operator-observed in the end-user app on scrollable sections. No controlled trace yet. Release-blocking.
2. **Two-hour soak exits early with code 0:** output JSON empty; source of graceful exit unknown. Release-blocking.
3. **Managed PowerShell uninstall prompt:** root cause proven; fix staged but untested/unpushed. Exact-head Windows qualification fails until rerun.
4. **Frame/input p95 evidence missing:** budgets are `<=16.7 ms` frame p95 and `<=50 ms` input p95 outside scans; averages do not substitute.
5. **Makira embedding license unproven:** public release cannot proceed on bytes/secrets alone.
6. **Screen reader limitation:** Windows/Linux accessibility exposes the named Native SDK canvas but not its internal widget tree. The unchanged TUI is the honest fallback.
7. **Physical Mac acceptance unavailable:** hosted builds/preflight cannot prove user-facing app behavior, accessibility, notarization experience, or long-lived Mac lifecycle.
8. **Claude review workflow broken externally:** no independent review findings from that check.
9. **No release/public bytes exist for v3.0.0:** branch workflows and draft PR are not publication evidence.
10. **NVIDIA tooling is limited:** `nvidia-smi.exe` exists; Nsight Systems (`nsys`), Nsight Compute (`ncu`), and NVIDIA Profile Inspector were not found. A bounded `nvidia-smi` utilization/memory sample can supplement, but never replace CPU/frame/input evidence.
11. **Do not rerun the old actionlint/ShellCheck integration:** it left six hung processes. Use bounded actionlint parsing and separate ShellCheck passes as already recorded in task activity.

## Task Board: What It Is and Why We Use It

This repository has a self-contained, git-tracked SHAUGHV task and workplace-memory system under `.tasks/`. The next agent does not need the SHAUGHV Tasks plugin to use it. The files and zero-dependency Node server are in the repository.

The board exists for four reasons:

1. **Release honesty:** it keeps build, hosted native, physical hardware, signing, licensing, provenance, and public-byte evidence as separate checkable gates.
2. **Cross-session continuity:** every Active task's `## Status`, `## Verification`, and newest `## Activity` entries tell a cold agent exactly where to resume.
3. **Operator visibility:** the browser board renders the same tracked Markdown the agents edit, so the operator sees current tasks and subtasks without reading the whole repository.
4. **Durable project memory:** `.tasks/CLAUDE.md` is the hot cache and `.tasks/memory/` is the deeper project/glossary/context store. The task descriptions hold exhaustive decision and handoff context.

### Board structure

```text
.tasks/
  TASKS.md                 authoritative task index and Kanban columns
  MILESTONES.md            milestone index
  tasks/<id>.md            rich per-task Why/Plan/Impact/Acceptance/Verification/Status/Activity
  milestones/<id>.md       rich milestone status and archived completed children
  CLAUDE.md                working memory/hot cache
  memory/glossary.md       durable terminology
  memory/people/           deep people notes
  memory/projects/         deep project notes; now includes sd300-v3.md
  memory/context/          durable context notes
  secure/                  local, gitignored secrets/private notes; never commit secrets
  config.json              durable board settings; title is SD-300
  board-config.js          generated title companion
  dashboard.html           browser UI
  board-server.mjs         zero-dependency live-sync server
  .board-server.json       runtime identity/port record; gitignored
  .board-version.json      tracked board bundle version
```

This board is configured as git-tracked/shared in `.tasks/config.json`, version 1.0.1, title `SD-300`. Runtime files and `secure/` are ignored; the task, memory, server, and dashboard assets are tracked.

### How to launch or recover it without the plugin

From the repository root:

```powershell
node .tasks\board-server.mjs status
node .tasks\board-server.mjs ensure --open
Get-Content -Raw .tasks\.board-server.json
```

Do not assume port 4317. This board currently reports port 4321 and PID 10288. The server is identity-bound to `C:\Users\hey\git\qube-system-diagnostics\.tasks`; it refuses stale-tab writes if the port belongs to another repository. The live URL is `http://127.0.0.1:4321/` until the runtime identity file says otherwise.

If Node is unavailable, open `.tasks/dashboard.html` as a static file. Static mode loses live two-way file synchronization but the Markdown remains authoritative.

### Source-of-truth rules

- `.tasks/TASKS.md` is the task index and source of truth for board columns.
- The existing columns are `Backlog`, `To-Do`, `Active`, and legacy completion column `Done`. Preserve their names and order.
- One task is one bold Markdown checkbox line. The task's own ID is the final bare `#id` on the line.
- `(needs #id)` declares prerequisites. A task must not move Active while a prerequisite remains open.
- `(ms #id)` attaches one milestone. IDs are unique across tasks and milestones.
- Indented checkbox lines are proper, dashboard-visible subtasks. Do not hide required checklist work only in prose.
- Detail-file `## Verification` is the completion gate. Every item must be `[x]` passed or `[~]` waived with a dated reason before the task can be completed.
- Every proper subtask must be checked; subtasks cannot be waived by silently completing the parent.
- `## Status` must say what is done, what is open, and the exact resume point.
- Append a timestamped `## Activity` entry for meaningful findings, edits, moves, pauses, failures, or evidence. On this shared board, attribute the agent.
- Keep `TASKS.md` concise. Put exhaustive reasoning and evidence in `.tasks/tasks/<id>.md`.
- Do not mark a milestone done while any tagged child remains open.
- When clearing old Done tasks, first archive milestone-tagged task lines in the milestone detail so progress remains accurate.

### How to update it during work

At the start of every session:

1. Read `.tasks/TASKS.md`.
2. Read `.tasks/MILESTONES.md`.
3. Read `.tasks/CLAUDE.md` and relevant `.tasks/memory/` files.
4. Read the full detail file for every task in Active.
5. Read the queued task/milestone that owns the next release gate.
6. Confirm `node .tasks/board-server.mjs status` identifies this repository.

While working:

1. Update the active task's `## Status` as the real state changes.
2. Tick or add concrete `## Verification` lines only on observable evidence.
3. Append an `## Activity` entry immediately after a meaningful finding or state transition.
4. Add a proper subtask to `TASKS.md` when the operator needs a visible required step, as done for scroll lag.
5. Keep `.tasks/CLAUDE.md` limited to current high-value working memory; move durable detailed facts into `.tasks/memory/`.
6. Never write tokens, font secret bytes, signing credentials, or private evidence into tracked task/memory files. Reference environment variables, GitHub secret names, or `.tasks/secure/` only.

At completion:

1. All subtasks checked.
2. Every verification line passed or explicitly waived with reason.
3. Change parent `[ ]` to `[x]`, add `(done YYYY-MM-DD)`, and move to Done.
4. Add the final `## Activity` line.
5. Close milestone only after every child is complete.

### What this board currently tracks

- `#nsp` Active: pinned SDK/shared engine/Corporate MSI vertical slice. All main slice checks pass except published-v2 hosted PTY and final performance/interaction closure.
- `#gux` Active: complete GUI. Explicit open subtasks now include severe minute-old scroll lag and final keyboard/scaling/tray/performance qualification.
- `#cpl` Active: every installer/updater/repair/uninstall path. Current immediate blocker is the managed receipt-parent cleanup correction and hosted rerun.
- `#qv3` To-Do: final v3.0.0 qualification/release, blocked by `#gux` and `#cpl`.
- `#giu` Backlog: post-v3 in-app/tray-driven update coordinator. Do not silently pull this into v3 scope.
- `#v3n` Open milestone: SD-300 v3 additive native GUI.

### Memory layers

- `.tasks/CLAUDE.md`: hot working cache for operator, terms, current projects, and preferences. It now records v3 branch/PR/current blockers and the rule that CI is not physical/license/release proof.
- `.tasks/memory/glossary.md`: durable definitions for install channel, latest intent, observation, and platform acronyms.
- `.tasks/memory/projects/sd300-v3.md`: durable v3 architecture, release order, evidence boundaries, blockers, and reference products.
- Per-task detail files are also memory. They contain the most exhaustive history and should be preferred for task-specific facts.
- `.tasks/secure/` is local and gitignored for secrets/private notes. Never put secrets into tracked memory.

### Git/shared-board behavior

- Pull/reconcile before a long board session when another operator may edit it.
- Commit meaningful board changes with the feature work they describe, or as a focused docs/task commit.
- If the same task line conflicts, keep the more advanced truthful state and union tokens.
- In detail-file conflicts, union and timestamp-sort `## Activity`; merge unique current facts into the latest `## Status`.
- The browser's stale-write protection is local only; git is the cross-machine collaboration layer.

## Important Context for Future Sessions

- Repository instructions in `AGENTS.md` are mandatory. Read it before any edit.
- The original Codex task is `019f7e10-080a-7f62-bdc0-37ef63a670ff`. This session is intentionally taking it over; do not send more work into the stalled task.
- The Goose comparison task is still active and may contain later v1.3.5 work. Read only what is needed; do not modify Goose or its thread while working on SD-300.
- The font source was deliberately removed from public Git and reconstructed only on trusted CI from secrets. Do not re-add the font file or secret values.
- The exact Native SDK distribution graph rejects `.path` dependencies and developer/profile/global npm paths. Use the owned wrappers and hash-verified patch flow.
- `cargo-dist` release workflow is intentionally customized. Do not regenerate it over project-specific release gates.
- The Windows local host previously had an orphaned test MSI record from deliberate failure injection; it was repaired exactly. Do not weaken installer verification to work around host state.
- A physical UI check, hosted package build, code signing preflight, notarization, attestation, and public-byte smoke are distinct evidence classes.

## What's Next

1. **Reproduce the minute-old scroll lag first, on the exact final bundle, without the Computer Use/UI Automation observer attached.**
   - Open a genuinely scrollable page such as Drivers/Processes/Network at an overflowing viewport.
   - Let the app run at least 60–90 seconds.
   - Record input event timestamps, update/message queue depth, frame start/end/present timings, dirty regions, history/model sizes, and scroll offset changes.
   - Compare immediate scrolling versus the one-minute state.
   - Determine whether the lag is input delivery, message backlog, model recomputation, retained-tree/damage growth, software raster, or present.
   - Do not lower collection fidelity or hide the issue with a slower refresh.
2. **Explain the early code-0 soak exit before launching a replacement.**
   - Read `target/performance-final-soak-20260721-233810.stderr.log` and `scripts/measure-native-gui-performance.ps1` around lines 240–270.
   - Audit the app's normal close/quit paths, singleton/private message, named lifecycle endpoint, test automation cleanup, and any scheduled process that may have requested graceful exit around 00:03.
   - Add bounded logging sufficient to identify the close source if it cannot be reconstructed.
3. **Independently review the two product-file installer diff.**
   - Confirm low-word HResult 145 is safe for the exact Windows PowerShell 5.1/.NET runtime used by the updater.
   - Confirm missing parent succeeds, nonempty parent preserves unrelated data, empty parent is removed, access denial fails, and no recursive deletion exists.
   - Confirm the PowerShell fixture's Base64 byte comparison is compatible with hosted PowerShell.
4. **Run focused validation only after review.** Suggested minimum:

   ```powershell
   [void][ScriptBlock]::Create((Get-Content -Raw scripts\validate-windows-self-update.ps1))
   cargo fmt --check
   cargo test managed_windows_cleanup_removes_integrations_when_gui_root_is_already_missing
   cargo test --locked
   git diff --check
   ```

5. **Commit and push the installer correction plus truthful board/handoff updates on the existing branch.** Do not create a new branch or worktree unless the operator asks.
6. **Dispatch a new exact-head Windows Native Installers qualification and require the unrelated-sibling proof.** The old run `29892216141` is failure evidence only.
7. **Fix the scroll/input regression, then capture frame/input p95 and rerun the full exact two-hour soak.** Keep functional automation and performance collection isolated.
8. **Use `nvidia-smi` only as a bounded supplementary GPU sanity sample if useful.** It cannot clear CPU, memory, frame, or input budgets.
9. **Resolve the Makira licensing decision with the operator.** Ask for App/Game embedding-license evidence or explicit authorization to replace Makira with a distributable open font.
10. **Only after every gate is green:** independent review, unchanged merge to `main`, same-SHA qualification, fresh immutable `v3.0.0` tag, draft candidate, signed/notarized packages, attestations, crate/latest publication, fresh-public-byte checks, website verification, and physical Windows acceptance.

Do not call the goal finished merely because ordinary CI is green. The current truthful state is: substantial implementation and local/hosted evidence exist, but severe end-user scroll lag, an invalid soak, one known Windows uninstall defect, font licensing, final signing/provenance, and public-byte acceptance remain open.
