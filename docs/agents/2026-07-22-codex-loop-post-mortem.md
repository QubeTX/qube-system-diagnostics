# Post-mortem: why SD-300 v3 took 24+ hours to close, and how it closed

**Date:** 2026-07-22
**Author:** Fable (orchestrating agent), at the operator's request
**Audience:** Codex agents and any future agent working this repository
**Scope:** The v3.0.0 native-GUI effort — roughly 24 hours of Codex work that
did not converge, followed by one session that did.

---

## Read this first

This is not a criticism of the agent that came before. **The prior work was
substantially correct.** Three independent adversarial reviews of the engine
ABI, the GUI, and the installer/updater surface — run during the closing
session — returned *zero* release-blocking correctness defects in the
implementation. The architecture held. The evidence discipline was real. The
qualification harness was strict enough to catch defects that would otherwise
have shipped.

The problem was never capability. It was **convergence**: a set of process
patterns that let a nearly-finished project stay nearly-finished for 24 hours.
Those patterns are the subject of this document, and every one of them is
fixable with a rule you can apply immediately.

---

## What actually happened

### The shape of the stall

By 2026-07-21 the branch contained a complete Native SDK desktop companion, a
shared Rust engine, six-target builds, and a composite installer lifecycle. CI
was green on the ordinary lanes. And yet the project could not finish, because
the closing gates kept producing failures that were *diagnosed but not
converted into landed fixes*, and because several gates were unfalsifiable in
practice.

Three concrete environmental traps were identified during the effort:

1. **Six hung `actionlint.exe` processes.** A workflow-audit integration
   spawned processes that never exited, so a "quick lint" became an unbounded
   wait that consumed an entire work cycle.
2. **An attached Computer Use / UI Automation observer contaminated CPU
   measurements.** The unchanged release GUI measured 5.39% of one logical core
   with the observer attached and 1.61% without it. Hours were spent optimizing
   against a number that was measuring the observer.
3. **A two-hour soak that ended at 25 minutes with exit code 0.** This was
   treated as a mysterious release blocker. It was a person closing the window:
   on Windows the close policy is compile-time `.hide`, and with tray disabled
   the next one-second tick converts a hidden window into a graceful quit. The
   harness cannot cause it, no watchdog exists, and the operator confirmed the
   interaction. (ADR 0001.)

### What the closing session found

Once the closing session started landing fixes rather than only diagnosing, the
qualification lane produced **eleven distinct real defects in sequence**, each
of which had been invisible behind the one before it:

| # | Defect | Why it stayed hidden |
|---|---|---|
| 1 | `Remove-Item` prompts on a non-empty receipt parent under `-NonInteractive` | Needed a real hosted run to hit |
| 2 | Windows PowerShell 5.1 `-Command` exit code mirrors the last statement's `$?` | Local proof ran under pwsh 7, which does not behave this way |
| 3 | `tool_exists("powershell.exe")` probes with `--version`, a 5.1 parse error | Every dev/CI machine has pwsh 7 installed |
| 4 | pwsh 7's `PSModulePath` shadows 5.1's built-in modules in spawned children | Only appears when the parent is pwsh 7 |
| 5 | Receipts written by 5.1 `Set-Content -Encoding utf8` carry a BOM that `serde_json` rejects | Only real-shell fixtures produce it |
| 6 | Retired MSI Cargo journals stranded an empty `Transactions` directory | Only visible once the receipt-root check got strict |
| 7 | Inno self-deletion race stranded an orphan `unins000.exe` | Timing-dependent, fast runners only |
| 8 | `usPostUninstall` PATH removal races owned-state verification | Fast runners only |
| 9 | Elevated worker stdio is lost across the UAC boundary | Made #8 undiagnosable rather than causing it |
| 10 | GUI-subsystem executables produce no output through PowerShell pipe capture on a consoleless runner | Silent empty string, not an error |
| 11 | Release producers fetched `refs/tags/vX` for an unpublished draft | GitHub only materializes the tag at publish; the hardening had only ever run in qualification-only mode |

Plus, in the product itself: the warmed-state scroll lag (full-viewport
repaints colliding with mandatory chart ticks on one queue), a wheel-momentum
regression introduced by its own fix, a tray-toggle lifecycle bug, and a
stale-data-on-restore bug.

**None of these are exotic.** Every one was findable in one cycle *once the
failure was allowed to speak.* Which points at the actual root cause.

---

## Root causes of the stall

### 1. Diagnosis was repeatedly substituted for landing a fix

The single largest factor. The prior sessions correctly root-caused the
managed-uninstall prompt, wrote the fix, wrote the fixture, documented the
reasoning — and then **paused with the fix uncommitted**, deferring "review and
qualification" to a future session. That fix was correct. Had it been landed
and dispatched, defect #2 would have surfaced within twenty minutes instead of
a day later.

A staged-but-unlanded fix produces zero information. The hosted runner is the
only oracle for an entire class of these defects, and you cannot query it with
an uncommitted working tree.

> **Rule: a diagnosed fix that is validated locally gets committed, pushed, and
> dispatched in the same working session. "Ready for review" is not a resting
> state for a one-line, test-covered correction on a feature branch.**

### 2. Failures were allowed to stay silent

Six of the eleven defects were masked by a *diagnostic* deficiency rather than a
product one: swallowed stdout, a lost `$?`, stdio destroyed by a UAC boundary,
PowerShell's ConciseView truncating a long `throw` message, an assertion
comparing against an empty string that was never printed.

The moment a failure was given a voice, its cause was obvious. In every case the
diagnostic patch cost minutes and the answer came back in the very next cycle.

> **Rule: when a check fails without telling you *why*, your next commit fixes
> the reporting, not the guess. Print payloads with `Write-Host` before
> throwing; capture GUI-subsystem output via file redirection; relay
> cross-privilege failures through a report file. Never spend a second cycle
> guessing at a silent failure.**

### 3. Success criteria were maximalist with no renegotiation path

The task board encoded release gates as absolutes: a two-hour soak, exhaustive
automation coverage, formal frame-p95/input-p95 percentiles, published-binary
PTY replay. Each is defensible. Together, with no waiver mechanism, they meant
**the project could not be declared finished by an agent working alone** — and
an agent that will not renegotiate criteria will grind forever against them.

The operator resolved this in one sentence ("as long as it works, launches, and
functions fine, we can do touch-ups later"). Four gates were waived with dated
reasons and moved to tracked backlog tasks. Nothing was lost — the evidence debt
is written down and owned — and the release shipped.

> **Rule: success criteria have an owner, and that owner is the operator, not
> the board. When a gate is expensive, blocking, and non-essential to "does it
> work," surface it explicitly: "this gate costs N hours and blocks release;
> here is what we lose by deferring it." Then honor the answer with a dated
> waiver and a backlog task. Do not silently drop it, and do not silently grind
> on it.**

### 4. The environment was not treated as a variable under test

Hung linters, an attached observer inflating CPU by 3×, a machine shared with
another product's installer acceptance — these were all *conditions of the
measurement*, and they were discovered by accident rather than by design.

> **Rule: before any measurement you intend to trust, state what else is running
> and eliminate it. Close observers. Verify with a process check, not a memory
> of having closed it. Quarantine the machine for unattended runs. If a number
> surprises you, suspect the environment before you optimize the product.**

### 5. Tooling blind spots were re-encountered instead of recorded

Computer Use cannot see or activate the Windows system tray. The operator
observed this producing a loop in a *different* repository on the same night.
It is a hard capability boundary, and hitting it repeatedly with the same tool
is unbounded work.

> **Rule for this repository: use Computer Use freely for GUI acceptance —
> it is genuinely good at it — with one exception: tray interactions are
> verified by programmatic dispatch plus a manual operator check. More
> generally: bound your retries, distinguish "the tool succeeded but the
> objective did not," and when a tool cannot perceive the thing you are testing,
> change tools instead of repeating.**

### 6. Local proof was mistaken for proof on the target runtime

The 5.1-vs-7 family of defects (#2, #3, #4, #5) all share one shape: a claim was
verified on the developer's shell and assumed to hold on the shell the product
actually spawns. PowerShell 7 and Windows PowerShell 5.1 differ in exit-code
semantics, module resolution, cmdlet availability, and default encoding.

> **Rule: when the product spawns a specific runtime, prove behavior on *that*
> runtime. Exception semantics and host exit-code contracts are separate claims,
> and each needs its own proof.**

---

## What the closing session did differently

Nothing exotic — the same tools, the same repository, the same standards:

1. **Read the handoffs completely, verified their key claims independently, and
   trusted the verified parts.** No re-derivation of settled work.
2. **Landed every validated fix immediately** and dispatched the hosted run that
   would test it. Roughly forty commits, each one a single reviewed change with
   its tests, changelogs in lockstep, and a board entry.
3. **Fixed reporting the instant a failure was mute.** Six diagnostic commits;
   each one produced its answer in the following cycle.
4. **Asked the operator to arbitrate scope once**, then executed against the
   answer without revisiting it.
5. **Delegated width, kept judgment.** Independent agents ran the three surface
   reviews, the scroll investigation, the fix implementations, and the docs;
   every finding was spot-verified against the code before being acted on.
6. **Wrote the reasoning down as it happened** — ADRs 0001–0003, board activity
   entries with evidence, dated waivers — so the next agent inherits conclusions
   instead of a mystery.

---

## The checklist

Before you pause a session, ask:

- [ ] Is there a validated fix sitting uncommitted? **Land it.**
- [ ] Did anything fail without telling me why? **Fix the reporting first.**
- [ ] Am I blocked on a gate the operator could waive? **Ask, with the cost.**
- [ ] Did I measure anything with an observer attached or a shared machine? **Redo it clean.**
- [ ] Did I prove runtime-specific behavior on the actual runtime? **Or on mine?**
- [ ] Have I retried the same tool against the same objective more than twice? **Change tools.**
- [ ] Would the next agent inherit conclusions, or a mystery? **Write the ADR.**

And the shortest version of all of it:

> **Every cycle must produce information. A cycle that ends with a staged fix,
> a silent failure, or an unanswerable gate produced none.**

---

## Related records

- ADR 0001 — soak early-exit attribution (the "mystery" that was a window close)
- ADR 0002 — warmed-state scroll latency root cause and fix ladder
- ADR 0003 — managed uninstall receipt-parent cleanup, plus the 5.1 exit-code addendum
- `docs/agents/handoff/2026-07-21-001-*` and `2026-07-22-002-*` — the inherited handoffs, which were accurate and are the reason the closing session could move fast
- `.tasks/tasks/{cpl,gux,nsp,qv3}.md` — the cycle-by-cycle activity log of the closing session
- Backlog tasks `#sok`, `#ext`, `#hrd`, `#mkl`, `#mac`, `#acc` — the evidence debt, written down and owned
