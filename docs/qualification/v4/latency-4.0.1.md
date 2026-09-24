# Corrective-release input latency investigation

The operator requested a fix before publication on 2026-09-24. Keep the accepted
100 ms input/frame p95 and ordinary-refresh maximum. All original next-version
targets and failed reports remain retained; no timing waiver applies.

## Diagnostic cycle 1: distinguish work from waiting

Baseline: candidate efbab9e, native CI 36037699515. Intel Mac input p95 is
104.133 ms; musl input p95 is 498.065 ms. The six consecutive musl keyboard
delays reach 666.377 ms while that cohort's synchronous frame maximum is only
10.488 ms. Both Intel slow inputs in the first navigation cohort open Network.
All expected inputs complete once and shutdown is clean.

Hypotheses: synchronous input work, time waiting for the host's responding frame,
or automation snapshot publication between input and that frame. These are
possibilities, not established causes. The normal software renderer and native
input scheduling remain unchanged in the first diagnostic candidate.

Candidate 9718301 adds three profiling-only monotonic stages and latest numeric
samples to the existing bounded report. End-to-end input timing and thresholds
remain unchanged. The wait split starts after input dispatch and ends at the
responding frame callback; publication is measured separately because it runs
outside synchronous frame-work accounting. These stages may overlap in elapsed
time and must not be summed as independent costs. Snapshot publication's current
cost appears in the next snapshot because its duration is known only afterward.

Local validation: 73 native GUI tests pass, with two expected platform skips;
12 interaction-report fixtures pass; strict model/binding, dependency-lock and
17-surface product-version checks pass. The patch applies and reverses against
the reviewed pristine SDK with both preparers' source hashes updated. The npm
tarball, integrity and Zig content hash remain unchanged: no upstream package
version or source archive was substituted.

Authoritative next oracle: native CI
[36042261345](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/36042261345),
plus a local Windows interaction run. No third unchanged timing retry: if the
new stage data cannot distinguish the hypotheses, investigate the missing
boundary before another product candidate.

The GTK source already schedules GPU frame emissions below layout and paint.
GLib explicitly runs a ready source only when no higher-priority source is
ready ([source priority](https://docs.gtk.org/glib/method.Source.set_priority.html)).
This makes scheduling a plausible hypothesis, but does not prove starvation in
this app. Raising the frame source above GTK paint without attribution can make
an expensive continuous frame loop prevent the visible repaint it is meant to
accelerate; do not use that as an unmeasured fix.

## Attributed contributor and diagnostic-cycle correction

The first GNU ARM64 report records input #77 at 515.014 ms, with only
0.333 ms in its last input dispatch and 511.663 ms waiting for the frame.
The intervening automation-publication stage is 504.933 ms. That stage includes
snapshot preparation and file I/O, so this identifies publication as a concrete
contributor without claiming that disk latency alone explains it. The same
cohort reaches 716.138 ms publication maximum. GNU x86-64 also retains a
1,102.741 ms individual input even though its p95 passes; do not discard it.

Move automation publication after a pending input's requested frame. Preserve
invalidation while yielding, then enqueue one later publication turn after GPU
completion; do not re-enter dispatch, spawn workers or change toolkit priority.
An input without a requested repaint must still publish, avoiding a deadlock.
Normal builds have no automation server and retain their existing frame path.

This is a correction to observer interference, not proof that the ordinary
release app had a half-second renderer stall. Retain the end-to-end measurement
and publication-cost diagnostics after the change and qualify all native
targets again. Intel's remaining result still determines whether a separate
ordinary-app rendering correction is necessary.

All six diagnostic reports are retained in
`native-interaction-latency-9718301.json` with artifact IDs, source and binary
identities, complete cohort metrics and numeric inputs exceeding 50 ms. Final
Intel results also attribute its two worst keyboard responses (127.434 and
126.208 ms) to publication (122.512 and 122.620 ms); frame waits are 122.694 and
122.695 ms. Worst cohort frame p95 remains 76.828 ms and ordinary-refresh
maximum is 83.514 ms. This supports the same observer correction on macOS.

The corrected candidate also walks retained views by reference in the two
read-only frame checks. Copying those large fixed-capacity values overflowed
the new Debug integration fixture's stack. Reference walks pass on the normal
test stack, without increasing stack limits or moving the fixture to a worker.
The final local suite passes 74 native tests (two platform skips), including
deferred publication, exactly one completion wake, no-repaint liveness,
preserved frame numbering and unchanged ordinary-build lifecycle behavior.
