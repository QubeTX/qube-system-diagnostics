# Native GUI local release gate

Date: 2026-07-21
Status: directed; implementation and local experiment in progress
Confidence: high on the gate, medium on the current implementation until the destructive-path trial passes
Revisit: immediately after the real Cargo-v2 MSI install and injected rollback trials

## Pre-flight

The inputs are recent internal source, task-board evidence, official Windows Installer
guidance, a clean hosted run, and a real failed Corporate MSI transition on the target
Windows machine. The hosted evidence proves buildability and modeled lifecycle behavior;
it does not cover the legacy-only Cargo ownership shape found locally, and no current
artifact yet proves an MSI-spanning rollback after ownership transfer.

Mode: self-check. Framework: Problem-Solving, stacked with Logical Reasoning to audit
the load-bearing claim that a candidate is safe to push. The `.tasks` board remains the
execution ledger; this file is the lossless reasoning canvas.

## Step 1 - Define and reframe the problem

### Current state, desired state, and gap

- What is happening: the integrated GUI, engine, and MSI build, and the GUI self-test
  passes. A real Corporate MSI install correctly refused to retire the live Cargo v2.0.6
  command because the installed ownership was recorded only in authoritative
  `.crates.toml`, not `.crates2.json`.
- What should happen: the second v3 update should transfer a proven Cargo-owned command
  into one composite CLI+GUI installation. Any later installer failure or cancellation
  must restore the exact prior Cargo binary, receipt, and ownership manifests.
- Gap: ownership recognition is incomplete, and the MSI currently has no paired
  rollback/commit action for external Cargo state changed by its deferred custom action.

This is release-critical because an incomplete success strands existing users without
the promised GUI, while an incomplete failure can remove the command they already trust.
The defect started in the v3 takeover implementation and must be closed before the next
feature-branch push. It affects current Cargo v2 users first, but the transaction rule
also protects alternate-user, unattended, repair, and failure paths.

### Reframes retained

1. Paraphrase: this is not a GUI-build failure; it is an incomplete model of legacy
   ownership and installer transaction boundaries.
2. 180-degree: the fastest way to make it worse is to keep debugging on clean runners,
   accept a successful install as sufficient, rely on inherited user environment, or
   allow rollback-disabled MSI execution.
3. Broaden: the unit of correctness is not `sd300.exe`; it is the composite product plus
   every pre-existing ownership record and the ability to reverse the transfer.
4. Redirect: the decision is not whether CI can pass. It is what evidence makes a
   feature-branch candidate eligible to consume CI at all.
5. Why: CI missed the real state because its Cargo fixture modeled `.crates2.json` as
   sufficient; that assumption came from our implementation rather than Cargo's actual
   legacy/current ownership behavior.

Selected problem statement: **SD-300 lacks a locally proven, MSI-spanning transaction
for transferring a real Cargo v2 installation into the v3 CLI+GUI composite product.**

## Step 2 - Analyze causes and boundaries

### Components

1. Ownership proof: binary, optional cargo-dist receipt, authoritative `.crates.toml`,
   synchronized `.crates2.json`, foreign owners, multi-binary entries, and path types.
2. Apply transaction: validate all inputs, create restricted backups/journal, compare
   original bytes before each mutation, and retain recovery material.
3. MSI transaction: register rollback before deferred apply, reject rollback-disabled
   execution, pass explicit invoking-user paths, and clean recovery material only at
   commit.
4. Product verification: exact CLI version, GUI self-test, application registration,
   shortcut, and absence of duplicate Cargo ownership.
5. Evidence: real-host dry run, real-host install, injected failure after cleanup,
   repair, no-op, GUI/TUI/CLI interaction, uninstall, and residue checks.

### Five whys

1. Why did the real install fail? The cleanup could not prove ownership from
   `.crates2.json`.
2. Why was that treated as the proof source? The initial implementation and fixtures
   modeled the newer JSON as complete.
3. Why did the fixtures not reveal the mismatch? They were synthetic and derived from
   the implementation, while this machine's Cargo state uses `.crates.toml` as the
   authoritative legacy record.
4. Why is adding the parser alone insufficient? Cleanup mutates files outside MSI's
   component database, so a later package rollback cannot automatically restore them.
5. Why is a successful retry insufficient evidence? It exercises only commit; the
   dangerous claim is that every post-takeover failure restores prior ownership.

Root cause: the first design treated ownership cleanup as a helper operation inside an
MSI instead of a nested transaction whose state must participate explicitly in MSI
rollback and commit.

### Edge cases and unknowns

- v1-only, both consistent, v2-only, conflicts, foreign owner, multi-bin owner, missing
  binary with metadata, exact receipt, invalid receipt, symlink/special path.
- failure after each staged/replaced artifact; cancellation; commit cleanup failure;
  concurrent Cargo mutation; custom `CARGO_HOME`; alternate credentials.
- A successful local Corporate lane does not prove Global elevation, macOS, Linux, EXE,
  notarization, or sustained performance. Those remain separate gates after the local
  candidate is formed.

## Step 3 - Diverge on solutions

1. Patch only `.crates.toml` recognition and retry the MSI.
2. Delegate cleanup to `cargo uninstall` and trust Cargo's own locks.
3. Remove Cargo takeover from MSI and require a separate manual migration command.
4. Keep the custom cleanup, but add a durable journal, paired MSI rollback and commit
   actions, explicit user/Cargo paths, and forced-failure qualification.
5. Stage Cargo cleanup before starting MSI and restore it from the updater if MSI fails.
6. Defer the GUI for Cargo users while shipping it to every other owner.
7. Preserve the existing Cargo binary as a side-by-side rollback copy indefinitely.

## Step 4 - Converge and select

| Option | Existing-user safety | Lifecycle fit | Recoverability | Testability | Result |
|---|---:|---:|---:|---:|---|
| Parser-only | low | high | low | medium | reject |
| `cargo uninstall` | medium | medium | low | medium | reject as sole mechanism |
| Manual third workflow | high | low | medium | medium | reject for v3 UX contract |
| Journal + MSI rollback/commit | high | high | high | high | select |
| Updater-owned pre-clean | medium | medium | medium | low | reserve fallback |
| Defer Cargo GUI | high | low | high | high | reject for release contract |
| Permanent side-by-side copy | medium | low | medium | medium | reject |

Selected solution: reconcile authoritative v1 and synchronized v2 metadata fail-closed,
then wrap the exact cleanup in an idempotent restricted journal with MSI rollback and
commit phases. Use explicit paths, refuse rollback-disabled execution, and require both
a real successful takeover and an injected post-cleanup failure before pushing.

Steel-manned dissent: the extra transaction machinery increases code and installer
surface, and Cargo's own uninstall path would be simpler. That objection wins if Cargo
can provide an atomic, reversible API under every installed-user/elevation shape. It
does not: invoking a subprocess still does not let MSI restore the prior binary and
metadata after a later MSI failure. The journal remains necessary.

## Logical audit of the push gate

Argument in standard form:

P1. If a candidate is eligible for push, it must preserve the prior installation under
every locally testable post-takeover failure.

P2. The current candidate has not yet demonstrated restoration after a failure that
occurs after Cargo ownership cleanup.

P3. A missing demonstration of a release-blocking invariant means the invariant is not
yet established.

Therefore, the current candidate is not yet eligible for push.

Dictionary:

- `P` = the candidate is eligible for push.
- `R` = restoration after post-takeover failure is established locally.

Symbolic form: `P -> R`, `not R`; therefore `not P` by modus tollens. Plain reading: a
candidate that requires rollback proof cannot be pushed while that proof is absent.

Validity: valid by modus tollens. Soundness: P1 is the approved compatibility contract;
P2 is directly evidenced by the missing trial; P3 defines the release gate. Confidence:
high.

Important non-inference: once `R` is proven, `P` does not follow by itself. Concluding
that would affirm the consequent. GUI interaction, CLI/TUI regression, package repair,
uninstall, performance, cross-platform CI, signing, provenance, and public-byte checks
remain additional necessary conditions.

## Directed next actions

1. Complete and review dual-manifest ownership reconciliation and the MSI journal.
2. Run the real-host dry run, successful Corporate takeover, and forced rollback; compare
   the original Cargo binary and both manifests byte-for-byte.
3. Build and exercise the complete local Windows candidate, then push once all local
   gates pass and use hosted runners for the unsupported local platform matrix.

Exit state: **Directed**.

## Recovery takeover — 2026-07-21 21:28

### Information triage

| Input | Provenance | Finding | Disposition |
|---|---|---|---|
| Interrupted `Vercel Native SDK` task | Codex task `019f7e10-080a-7f62-bdc0-37ef63a670ff`, interrupted after 11,543 seconds in its latest turn | Ended while four Cargo/MSI agents were still being reconciled; it did not produce a final handoff | Keep as history, not proof |
| Live branch and worktree | `codex/sd300-v3-native-gui` at `e9c6a61` plus uncommitted recovery work | Contains the legacy-manifest parser, full package-ID arbitration, regular-file checks, CAS staging, and paired MSI journal | Keep and verify |
| Task board | Active `#nsp`, `#gux`, and `#cpl` | GUI sustained foreground/hidden gates already pass; real legacy Cargo MSI replay remains the immediate local lifecycle gate | Keep as execution truth |
| Current Alienware ownership | `C:\Users\hey\.cargo` inspected locally | v2.0.6 exists in `.crates.toml` while `.crates2.json` has no SD-300 entry; this is the exact state that defeated the prior synthetic fixture | Keep as physical acceptance fixture |
| Pushed CI | PR #4 head `e9c6a61` | All ordinary Rust and six Native GUI target jobs pass; those results predate the uncommitted MSI journal and do not prove it | Keep, but do not overclaim |

Decision needed from the recovery agent: is the worktree eligible for a new candidate push?
Doing nothing leaves a tested GUI branch stranded on an unproven real-current-user ownership transfer. There is no deadline that justifies skipping rollback proof.

### Assumptions checked

- `tested, high confidence`: the prior task was looping because it repeatedly broadened review/CI after each fix. Recovery therefore freezes product scope and admits new work only when an existing verification item fails.
- `tested, high confidence`: the four findings from the completed Cargo review are present in the current source, but prior commentary alone is not proof. Targeted tests pass for package-ID reconciliation, malformed ownership, special files, and CAS behavior.
- `dismissed, high confidence`: fixing the legacy parser alone makes the candidate safe. The MSI transaction also requires an independent rollback action that restores external Cargo state after a later package failure.
- `open, medium confidence`: the WiX rollback/commit schedule behaves correctly with the freshly installed helper under a real Windows Installer rollback. This must be tested through the built MSI, not inferred from XML.

### Logical audit of the recovery conclusion

Argument in standard form:

P1. A new hosted candidate is justified only if every locally testable release invariant passes on the exact local candidate.

P2. Legacy Cargo ownership transfer and post-transfer MSI rollback are locally testable on this host.

P3. Those paths have unit proof but not yet built-MSI proof on this candidate.

Therefore, the worktree is not yet eligible for push.

Dictionary: `P` = eligible to push; `L` = all locally testable invariants pass; `M` = the built MSI legacy transfer/rollback trial passes.

Symbolic form: `P -> L`, `L -> M`, `not M`; therefore `not P` by hypothetical syllogism plus modus tollens. The argument is valid. P1 is the operator's explicit local-first gate, P2 is established by the current v2.0.6 legacy-only install, and P3 is directly observed. Soundness confidence: high.

Guard against the earlier loop: once `M` passes, `P` still does not follow by itself; asserting it would affirm the consequent. The bounded remaining local regression/interaction/soak gates must also pass, but no new architecture or feature audit is authorized unless one of those concrete gates fails.

### Recovery action and checkpoint

The current source now has passing unit trials that simulate the MSI phases directly:

- prepare followed by rollback restores the Cargo binary, managed receipt, `.crates.toml`, and `.crates2.json` byte-for-byte and removes recovery residue;
- prepare followed by commit leaves the exact Cargo ownership retired and removes the journal, marker, and backups.

Next action: rebuild one complete Corporate MSI, run the injected post-Cargo failure against the real legacy-only v2.0.6 install, verify byte-exact restoration/no MSI residue, then run successful takeover, current/repair, GUI/CLI/TUI interaction, and supported uninstall.

Exit state remains **Directed**. Confidence: high on scope and gate; medium on the built MSI until the physical trial passes.

## Physical checkpoint — 2026-07-21 21:56

The missing built-MSI condition `M` is now established. The final Corporate package
executed the Cargo prepare action against the real legacy-only v2.0.6 owner, then a
deferred qualification action failed. Windows Installer invoked the registered rollback
action, and the Cargo binary, `.crates.toml`, and `.crates2.json` matched their original
SHA-256 hashes afterward. No product registration, payload, shortcut, journal, marker,
or backup survived.

The successful path then retired only the proven Cargo owner, installed and registered
v3.0.0, passed CLI and engine checks, restored a deliberately absent engine through MSI
repair, presented and navigated the live native GUI, focused the existing singleton on a
second launch, exported a redacted report, closed normally, and uninstalled while
preserving the export. The workstation was restored to the exact original v2.0.6 Cargo
binary and manifest hashes after qualification.

This satisfies `M` but does not by itself establish push or release eligibility; that
would be the affirming-the-consequent error identified above. The remaining bounded
conditions are exact-head regression and hosted target execution, the final soak,
license/signing evidence, provenance, and immutable public-byte verification. No new
architecture or feature review is admitted unless one of those gates fails.

## Accessibility boundary — 2026-07-21 22:08

Physical Windows UI Automation inspection exposed only the named GPU canvas and the
native title bar. Source inspection explains that result: Native SDK 0.5.4 publishes
the retained canvas-widget accessibility tree to its macOS platform service, while the
Windows and Linux platform services do not register a widget-tree publisher. The SD-300
roles, names, focus order, text chart equivalents, and deterministic automation tree
remain real, but they are not an operating-system screen-reader tree on those two hosts.

The release claim is therefore narrowed explicitly: macOS carries the system semantic
bridge; Windows and Linux carry keyboard interaction and internal semantic automation,
with the unchanged TUI retained for system screen-reader use. This is platform-unavailable
evidence under the parity contract, not a successful physical screen-reader result. A
future SDK upgrade must add and physically prove the missing bridges before the limitation
can be removed.

## Loop isolation and performance correction — 2026-07-21 22:35

The first rebuilt two-hour Processes run was stopped after nineteen minutes because the
GUI averaged about 6% of one logical core. A three-minute repeat measured 5.39%, split
between the Native SDK window thread, the Rust engine thread, and a UI Automation worker.
Overview reproduced 4.49%, proving the excess was global rather than process-table work.

The controlled physical-interaction pass had left the Codex Computer Use observer alive.
That observer consumed about 15% of a core and kept a UI Automation client attached to
each new GPU window. Closing the observer through its supported session API removed the
automation worker. The same unchanged release binary then measured 1.61% across a fresh
90-second Processes sample: 0.67% window and 0.94% engine, with 77.50 MiB average working
set and 220.33 MiB average private memory. The earlier failing samples are retained as
contaminated diagnostic evidence and cannot be cited as product performance.

The 15-hour validation loop had a second concrete cause: six `actionlint.exe` processes,
including copies started at 19:24 and 22:31, were waiting indefinitely in its Windows
ShellCheck integration. They were terminated by exact PID. Actionlint's native workflow
parser passes all eight workflow files with external linters disabled; a separate bounded
ShellCheck pass over 80 extracted Bash workflow blocks plus eight repository shell scripts
passes after LF normalization and substitution of GitHub expression placeholders. This
preserves both checks without another unbounded child process.

Refreshing the Native SDK model contract also exposed fourteen strict warnings hidden by
the stale generated contract. They were dead bindings from the deliberately removed bottom
topic rail. The unused active-topic view fields/functions were removed, internal live/stale
summary fields were declared view-unbound, and the regenerated contract now passes strict
validation plus 30/31 Native tests (one expected platform skip).

The candidate is ready for one fresh distribution build and a quiet final soak, but not for
public release. Makira app-embedding license evidence and hosted signing/candidate gates
remain independent release conditions.

## Final Windows lifecycle convergence — 2026-07-21 23:38

Physical testing of the fresh distribution exposed three connected lifecycle facts that
source/unit review had not proved. Native SDK fixes the scene-first main window close policy
from `app.zon` when the host window is created, so changing the later runtime scene could not
turn close-to-tray on. Once the host was correctly fixed to hide on Windows/macOS, an ordinary
launch could inherit the prior policy-hidden state, and the existing singleton listener's
thread-only timer message was filtered by the SDK's window-bound message pump.

The converged design keeps the host close policy at hide only on tray-capable Windows/macOS;
Linux retains quit. The model converts a policy hide to graceful quit whenever the persisted
tray setting is off, while minimizing remains a distinct non-quit state. Explicit
`--startup --hidden` wins only when tray is enabled; every ordinary launch forces the monitor
visible. Windows installs a private window subclass from the SDK UI thread, and the singleton
listener posts a window-bound Open message through it. The typed `Msg.open_window` path then
restores both host pixels and SDK lifecycle state before resuming one-second sampling.

The exact final ReleaseFast/baseline executable has SHA-256
`d83de90b2bf05a3f23d755ac0fb8d9d9b6a4b98acbe44011fc6f7292797acaa2`; its adjacent engine
matches the build receipt at
`cbd79f8c1205a6aeb4e4b6c4a2e7a4d6f60c86fe90228947e831e1e325da6d7c`. Physical acceptance
on the Alienware proves ordinary visible launch, launch-at-login registration with exact
`--startup --hidden`, hidden startup without a visible top-level window, same-PID singleton
reopen, resumed one-second sample progression, repeat close-to-tray, registration removal,
restored tray/startup defaults, and ordinary close/exit. The Computer Use observer was closed
through its supported API before performance measurement.

The distribution wrapper now performs a second strict check after `native test` emits the
exact staged model contract, preventing a stale preflight contract from reducing a successful
distribution receipt to structural-only checking. The exact final two-hour Processes soak
started at 23:38 local; its result remains the only open local performance datum.
