# ADR 0023: accept the measured Windows overview CPU result

Date: 2026-09-27
Status: Accepted by the operator for the recorded 4.1.0 candidate result
Related: #ov6, #r17, docs/qualification/overview-4.1.0.md

The operator annotated "Windows overview CPU measured 2.24%" with "That's okay."
Accept the specific 2.23884384% one-core result from the observer-free 330-second
Windows Overview measurement of 0b1e53d as nonblocking for this candidate.

Retain its failed original 2% verdict, unchanged measurement, artifact identity,
memory verdicts and the original engineering target. This is not a general
increase in the Windows CPU ceiling and does not accept hidden-mode costs,
other workloads/platforms, memory overruns, later materially different results,
frame/input timing failures or missing functional evidence. #r17 retains the
original resource target. No global resource-policy code changes are needed to
record acceptance of this particular local result.

The same operator reply asks to investigate the ARM64/Mac frame-time failures.
Those remain blocking; no timing exception is granted. Release still requires
the remaining functional, installer, platform and exact-candidate CI checks.
