TT;DR: Make the release chain immune to a `main` push that lands while a release is in flight, so the human/agent operating it can't desynchronize the producers and qualify gate from the release commit the way the v3.1.1 release scare did.

## Why
During the v3.1.1 patch, a one-line board commit was pushed to `main` while the Release workflow was mid-flight. Because the native-installer producers and the qualify gate are `workflow_run`-triggered and GitHub associates such runs with the branch head at trigger time (and they check out the branch head), the release's own producers appeared under the newer commit and the qualify gate failed its identity check (`release vX targets <A>, but this qualification checkout is <B>`). The public bytes were never corrupted, but recovery cost several rebuild cycles. Full account: the addendum in `docs/agents/2026-07-22-codex-loop-post-mortem.md`.

The operational discipline (never push during a release) is documented in `AGENTS.md` and is the current safeguard. This task adds a *technical* safeguard so the discipline is not the only line of defense.

## Plan
1. Pin every `workflow_run` consumer to the triggering Release's commit rather than the branch head: in `windows-installers.yml`, `macos-installer.yml`, `linux-native-gui.yml`, and `release-qualify.yml`, resolve and check out `github.event.workflow_run.head_sha` (fall back to the draft's target commitish) instead of the default branch head, so a later `main` push cannot change what these jobs build/verify.
2. Add a `concurrency` group to `Release` (`group: release-${{ github.ref }}`, `cancel-in-progress: false`) so a second push queues behind an in-flight release instead of racing it.
3. Consider a small `codesign` timestamp retry in the macOS signing step (transient Apple timestamp-server failures) so a one-off flake self-heals without a manual re-run.
4. Prove it with a full local dry-run plus a real hosted branch-only qualification cycle that deliberately pushes a second same-version commit mid-release and confirms the chain stays pinned and green.

## Impact
Intended: a mid-release push, or an operator glancing at branch-head-associated runs, can no longer desynchronize or be misread into cancelling the release. Risk if done carelessly: the release system is deliberately customized and load-bearing; a wrong checkout ref or concurrency setting could strand or mis-target a real release. This must be a standalone, reviewed change with dry-run evidence — never rushed onto a live release.

## Acceptance
- A second same-version `main` push during an in-flight release does not change the producers'/qualify's build-or-verify commit, and the release still publishes correctly from its original commit.
- The immutable-tag and draft-identity guards remain intact; single-commit provenance is preserved.
- macOS signing survives a simulated single timestamp failure without manual intervention.

## Status
Backlog. Owned as the systemic follow-up to the v3.1.1 release-operations incident; the AGENTS.md discipline is the interim safeguard.

## Activity
- 2026-07-23 — filed from the v3.1.1 release scare. Root cause and interim discipline recorded in AGENTS.md + the post-mortem addendum; this task carries the durable technical fix (agent: opus)
