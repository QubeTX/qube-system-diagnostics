# ADR 0018: updater recovery and the corrective release

Date: 2026-09-23
Status: Accepted for the operator-authorized corrective update
Related: ADRs 0016/0017, task #p4h, PR #11

## Evidence

The actual public Windows 4.0.0 updater returned an empty release response.
Eight direct runtime cases isolate PowerShell's detached launch with null stdin:
both Windows PowerShell and PowerShell 7 exit successfully without executing the
command, through either file or pipe output. Hidden-window launch executes it.
The first corrected live GitHub check then exposed non-UTF-8 output in release
metadata; explicit UTF-8 output passed the real public API check.

Microsoft documents the distinct [process creation flags](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags)
and [PowerShell encoding behavior](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding?view=powershell-5.1).
These references support the flags/encoding contracts; the silent exits above
are observed runtime evidence, not a claim made by those documents.

Separately, the immutable 3.1.3 updater imposes its private engine ABI/schema on
the newly installed GUI. That rejects v4 even after the new GUI successfully
loads its own engine. An installed product should validate its own engine;
an older updater must validate the stable self-test envelope, product identity,
target version and successful result rather than require the old private ABI.

## Decision

- Give PowerShell hosts hidden-window launch and preserve suspended job
  assignment, child ownership, output/deadline bounds and cancellation. Native
  monitoring workers retain their detached, console-free path.
- Emit UTF-8 explicitly, treat HTTP/PowerShell failures as nonzero, and validate
  the entire release response before accepting a transport. Empty, malformed,
  incomplete, draft/prerelease and invalid-version responses try the next
  bounded transport. Do not turn exit zero into success without usable data.
- Preserve schema-1 lifecycle JSON keys and exit codes. Errors identify the
  release-check step, bounded transport details, unchanged installation state
  when checking failed before mutation, and an actionable recovery route.
- Keep exact ABI/schema checks for same-version companion validation. For an
  updated product, require the stable self-test schema, matching product/version,
  successful self-test and nonzero ABI/schema identifiers; the new GUI itself
  still enforces its exact engine contract. This cannot retroactively change
  immutable older executables; affected installations need the official installer
  once to acquire the correction.

## Publication authority and performance scope

After the explicit proposal of a corrective 4.0.1 using the same v4 performance
ceilings, the operator requested investigation and then said: "Go ahead and
push, merge, and deploy as an update." The operator also requested installation
of the production update on their machines. This authorizes the corrective
release despite the earlier one-release plan. Publish new immutable artifacts;
never replace or retag 4.0.0.

For this narrow correction, extend the same ADR 0016/0017 ceilings to **4.0.1**:
4% foreground CPU, 3% hidden CPU, 200 MiB summed RSS, 300 MiB private memory,
100 ms frame/input p95 and 100 ms ordinary-refresh maximum. No limit increases.
This supersedes only their version-expiry boundary; every version after 4.0.1
defaults to the original targets. Keep all original goals and failed verdicts.

The patch changes lifecycle checking/validation, PowerShell launch selection,
version metadata and diagnostics, without changing periodic collectors, GUI
rendering or scheduling. Retain the completed v4 resource/soak evidence as that
monitoring baseline, explicitly identified by its original artifact hashes;
do not relabel it as a new 4.0.1 soak. Run new six-target native functional/input
checks and composite lifecycle qualification, including real public HTTP checks
outside the synthetic candidate override. Public installed-byte and updater
verification remain required before completion.

On 2026-09-24 the operator extended the correction to investigate missing CLI
PATH and GUI discovery on a separate work computer, and requested an equivalent
Linux audit. The installer fixes cover configured application-menu locations,
verified discovery, custom-prefix consistency and clear failures; they do not
change monitoring, rendering or scheduling. Requalify the affected lifecycle
paths on the resulting candidate. This extension does not waive the unresolved
GNU x86-64 input gate or claim access to the work PC's endpoint-security evidence.

## Verification

Real PowerShell file/pipe capture proves execution, stdout, stderr and nonzero
outcomes. Deterministic updater fixtures cover empty/invalid responses, HTTP
failure, fallback, missing helpers, deadlines and bounded safe error text.
The opt-in live public-release test runs in Windows CI and cannot use the
candidate-version shortcut that hid the original defect. Companion fixtures
cover new private ABIs, same-version mismatches, wrong identity, failed self-tests
and unsupported envelope schemas. Record hosted and production outcomes in #p4h.
