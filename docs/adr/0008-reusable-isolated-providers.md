# 0008 — Reusable isolated providers

Status: Accepted, 2026-09-23. Extends ADR 0006 without moving native probes into UI threads.

## Decision

Each enabled non-fast lane owns at most one subprocess. The child accepts only a two-byte
sample/reset message for its fixed read-only topic. One request can be outstanding; no
input, output or reader-thread queue grows with time. Responses use a length-framed
in-memory pipe with an eight-mebibyte limit checked before payload allocation.
The parent owns the process group/job, deadline and cancellation; an error destroys the
worker before recovery. Child stderr/stdout remain bounded by the existing output monitor.

Session-local caches therefore survive between samples. Static graphics discovery is
keyed to the current DXGI device identities and expires after five minutes. Negative
optional-provider observations back off to five minutes, while successful telemetry is
always recollected. Explicit retry resets caches. Interface/disk identity changes trigger
discovery, and a long fast-sample gap resets rates and discovery after resume.

## Qualification

Fixtures prove negative caching, recovery and identity invalidation. A real collector
protocol test sends multiple requests to one PID, requests cache reset and verifies bounded
cancellation/shutdown. A hung-helper fixture exercises timeout without requiring native
return or pipe EOF. Hosted native runs validate each OS's process ownership and framed-pipe
behavior. Release CPU/memory/handle and two-hour soak gates remain separate from these tests.

## Windows startup cost, 2026-09-23

Measured helper startup previously enumerated every system thread to resume one
CREATE_SUSPENDED child. Use [PssCaptureSnapshot](https://learn.microsoft.com/en-us/windows/win32/api/processsnapshot/nf-processsnapshot-psscapturesnapshot)
with only PSS_CAPTURE_THREADS to obtain that child's initial thread. The job is
assigned before resuming; capture failure falls back to the prior Toolhelp path.
Snapshot and walk-marker descriptors are always released in the calling process,
following [PssFreeSnapshot](https://learn.microsoft.com/en-us/windows/win32/api/processsnapshot/nf-processsnapshot-pssfreesnapshot).
A native fixture exercises the PSS path directly so fallback cannot conceal a regression.

## Idle memory, 2026-09-23

Retain workers for sub-minute activity, slow telemetry, sockets and diagnostics.
Retire static inventory, drivers and SMART/reliability workers after each response;
the parent retains their samples for the unchanged five-minute/one-minute cadence.
These topics do not need a child-local numeric baseline between requests. Native
checks remain isolated and cancellation still joins owned workers before unloading.

The unchanged-binary foreground repeat attributed its working-set peak to
simultaneous static and driver workers at the five-minute refresh. Static,
driver and health lanes now share one session-local process permit. Their three
existing threads wait with cancellation; no request queue or replacement thread
is introduced. Each lane keeps its cadence and captures its actual acquisition
time. Counter, socket, diagnostic and live telemetry lanes remain independent.
Panic releases the permit, and shutdown never waits for the active probe merely
to cancel a lane waiting for that permit. Long resource gates still determine
whether the resulting peak is acceptable.

## Capture clocks, 2026-09-23

Disk baselines belong to the persistent activity worker, and rates are computed
at native counter capture before serialization. The envelope carries the actual
capture timestamp and monotonic interval. Parent IPC delivery latency cannot
alter a measurement denominator. Worker replacement and explicit retry start
with unavailable rates. Wall-clock discontinuities invalidate disk baselines on
platforms whose monotonic clock excludes suspend, but wall time never divides
counter deltas. Fast samples likewise record capture-to-capture intervals.

## Detached Windows helpers, 2026-09-23

Use DETACHED_PROCESS with CREATE_SUSPENDED for owned helpers whose standard
handles are already redirected. CREATE_NO_WINDOW still allocated headless console
hosts in measured runs. The detached flag avoids those hosts without changing the
job, deadlines, output limits or cancellation. The flags follow Microsoft's
[process creation contract](https://learn.microsoft.com/en-us/windows/win32/procthread/process-creation-flags).
Native fixtures verify no attached console and valid stdout/stderr through both
capture paths; the real persistent-worker protocol exercises redirected stdin.

## In-memory responses, 2026-09-23

This supersedes the original atomic temporary-file response transport. Each response
starts with `SD4\0` and a little-endian 32-bit length. The parent reads only bytes
already queued in the sole-reader pipe, accepts arbitrary header/payload fragmentation,
and bounds draining per iteration so output cannot starve cancellation. Stderr shares
the output budget and retains only a small diagnostic prefix. Invalid, oversized,
partial, timed-out or cancelled responses terminate and reap the owned process family.
An inherited pipe never requires waiting for EOF. Native fixtures cover these cases
and repeated large responses from the same worker on each supported platform.

This removes per-sample filesystem traffic and private payload files. It does not
change capture timestamps, cadences, provider isolation or the public CLI contract.
Performance acceptance still requires measurements of the complete process family.
