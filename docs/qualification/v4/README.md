# SD-300 v4 qualification evidence

The operator's 2026-09-23 release decision is recorded in
[ADR 0016](../../adr/0016-v4-responsiveness-release-decision.md): for 4.0.0 on
all six targets, frame/input p95 and ordinary-refresh maximum may reach 100 ms.
Original-target verdicts stay visible. The subsequent resource decision in
[ADR 0017](../../adr/0017-v4-resource-release-decision.md) permits 4% foreground
CPU, 3% hidden CPU and 200 MiB RSS for 4.0.0 only; private memory and functional
gates remain unchanged. All original goals are preserved in
[Next-version targets](../../next-version-targets.md). `native-interaction-ec5635d.json` retains all six native reports and a
separate assessment under that policy. Five targets meet the approved timing
limit. Intel Mac's first navigation cohort exceeds it (151.354 ms frame p95,
177.170 ms input p95; first refresh maximum 110.023 ms), so an unchanged-product repeat was required and is recorded below.

The unchanged-product repeat in
[35926413084](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35926413084)
passes the approved policy and complete native CI on all six targets. Retained
`native-interaction-ce77ebf.json` includes original-target verdicts, all twelve
cohorts per target, artifact identities and exact keyboard-completion counts.
Intel Mac now records frame/input p95 29.462/38.219 ms and refresh maximum
13.951 ms. Its prior slow first cohort did not reproduce; the earlier failure
is still retained rather than reclassified or removed.

The ec5635d native Mac fixtures pass on both architectures: passive semantic
publication emits no assistive actions, actual external focus remains actionable,
and unsupported legacy AppKit selectors do not throw. All four keyboard cohorts
on every platform now record exactly 20 completions for 20 actions. Both Mac
follow-up profiles lack the earlier unsupported-selector/exception-unwinding
frames. These findings resolve those specific defects, not every accessibility
behavior or the remaining resource limits.

The final four-host resource run
[35923985086](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35923985086)
is retained in `native-resources-ec5635d.json`. Each host ran independent,
unobserved 900-second TUI, Thermals foreground and hidden windows using hashed
normal release artifacts. All sessions shut down cleanly. The operator approved the separate v4-only resource policy.
`release-resource-assessment.json` assesses all nineteen retained native/local
windows against that policy and passes; original failures remain unchanged.

| Target | TUI CPU / RSS | Foreground CPU / RSS | Hidden CPU / RSS |
|---|---|---|---|
| Linux GNU ARM64 | 0.899% / 27.96 MiB | 1.061% / 145.12 MiB | 0.193% / 123.32 MiB |
| Linux GNU x86-64 | 1.625% / 34.94 MiB | 1.790% / **154.06 MiB** | 0.254% / 132.23 MiB |
| macOS Intel | 1.564% / 34.66 MiB | 1.766% / 119.33 MiB | **1.025%** / 109.59 MiB |
| macOS Apple Silicon | **2.047%** / 54.06 MiB | **2.152% / 187.63 MiB** | **1.787%** / 148.09 MiB |

CPU is percent of one logical core including owned descendants. Bold values
exceed an original limit. The unchanged Linux musl runtime is covered by the
preceding native matrix below (foreground CPU 2.090%, RSS 79.09 MiB).

`windows-resources-ee939df.json` retains the local 4.0.0 release-build identity,
quarantine declaration and all three completed windows. The two-hour Processes
soak records 1.488% CPU, 77.80 MiB RSS and 228.86 MiB private memory; fifteen-minute
Thermals foreground records 0.988%, 141.46 MiB and 257.73 MiB; thirty-minute hidden
records 0.298%, 109.50 MiB and 248.75 MiB. All original resource gates and clean
shutdown pass. Subsequent product changes affect macOS only. These are qualified
candidate bytes; final public artifact identity is verified during publication.

The complete hosted resource comparison at 309d2f3 is retained in
`native-resources-309d2f3.json` with all six targets, eighteen paired 900-second
windows, exact executable identities and before/after values. It precedes the
coordinated v4 version and later Mac focus correction, so it is evidence about
that candidate rather than final release acceptance. All candidate windows
shut down cleanly. Windows and Linux ARM64 pass resource gates. Remaining
candidate failures are GNU x86-64 foreground RSS (156.10 MiB), musl foreground
CPU (2.09% of one core), Intel Mac TUI/foreground/hidden CPU
(2.39%/3.27%/1.93%), and Apple Silicon TUI/foreground/hidden CPU
(2.18%/2.23%/2.19%) plus foreground/hidden RSS (188.78/162.41 MiB).

The first full native interaction matrix is retained in
`native-interaction-d2c89da.json`. Refresh stalls and shutdown pass, but every
target has a frame or input failure. The unchanged diagnostic repeat
[35917363148](https://github.com/QubeTX/qube-system-diagnostics/actions/runs/35917363148)
adds bounded per-input traces. Mac focused semantic snapshots were calling the
assistive-action setter, recording duplicate input and clearing keyboard focus
visibility. The later ec5635d evidence above qualifies the correction on both
native Mac runners. The original report remains unchanged; only the explicit
ADR 0016 timing decision changes the release assessment.

The initial measurements below are diagnostic runs, not final release acceptance.
The completed windows above preserve remaining resource-limit failures. Final
composite installer qualification and public verification remain open. Do not infer hardware accuracy from parser fixtures or builds.

The 0069088 Windows automation candidate passes the full synchronous work,
input and refresh-stall gates across 1180×760 and 950×760 viewports in both
audience modes. Worst cohort p95 is 14.503 ms for work and 40.591 ms for input;
the largest ordinary Processes refresh is 9.816 ms. The retained report names
all three executable hashes. These are runtime event receipt-to-present timings,
not physical input-device or display-scanout measurements. Automation is compiled
only into a separate qualification stage and is never attached to resource runs.
The following instrumentation fix also closes no-damage cycles; it prevents
unrelated idle updates accumulating into an invented long frame. Requalify the
coordinated 4.0.0 candidate before publication.

The 2c3a2ad font candidate passes the local 15-minute foreground and 30-minute
hidden Windows windows. Hidden mode measures 0.280 percent of one core,
112.40 MiB peak summed working set and 234.76 MiB private memory, with required
collectors and clean shutdown. The report retains exact executable hashes.
Hosted comparison 35895796996 passes Windows and Linux ARM64. Intel/ARM Mac
CPU and ARM Mac RSS, GNU x86-64 foreground RSS/CPU and musl foreground CPU still
require qualification; all measured candidate sessions shut down cleanly.

Mac thread attribution initially used psutil's task-port queries, which hosted
permissions denied. The diagnostic now reads public `PROC_PIDTHREADID64INFO`
for kernel thread IDs from the bounded native stack report. Apple's
[fill_taskthreadinfo](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/bsd_kern.c)
returns these thread counters in nanoseconds, unlike the task counters below.
A native fixture compares a bounded CPU workload with Python's independent
current-thread clock. The following five-second thread window reports readable
coverage and whole-process CPU; it is diagnostic evidence, not resource acceptance.

## macOS process counters

The pinned sysinfo implementation can retain the previous percentage when its
CPU delta is zero and discards libproc read failures. The shared replacement
uses one `PROC_PIDTASKALLINFO` query per process, checks the returned byte count,
and preserves denied/error fields. The combined BSD/task record binds creation
identity and counters to the same process. A failed inventory clears baselines;
PID reuse, counter rollback, retry/resume and long gaps require a new baseline.

Apple's [proc_info interface](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info.h)
defines resident bytes and BSD creation seconds/microseconds.
[proc_pidtaskinfo](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/proc_info.c)
copies [fill_taskprocinfo's Mach absolute CPU ticks](https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/bsd_kern.c).
Convert with `mach_timebase_info`, then divide by the actual monotonic interval;
100 percent means one logical processor, without machine-core normalization.
Zero counter delta is a measured zero, not a reason to retain the prior value.

Native Intel and Apple Silicon tests run an isolated CPU workload and compare
converted CPU seconds against `getrusage(RUSAGE_SELF)` over bracketed windows.
The declared tolerance is 20 ms for accounting granularity and call boundaries;
the following idle interval must consume less than 20 ms. This proves the
kernel-counter conversion on hosted machines, not physical-device telemetry.
Stage profiles use the same sampler as both frontends. Whole-product CPU gates
remain separate from this calculation check.

## Windows TUI process-tree measurements

The CI workflow accepts an explicit `resource_seconds` dispatch input for
before/after native TUI and foreground/hidden GUI measurements on all six targets. It builds the immutable
f83ae429490aecb165921037b2f9ed994327634f baseline and finishes all compilation and
visual interaction checks before sequential measurement. Ordinary pull-request
runs do not launch the long resource windows. Candidate gate failures retain
all reports and fail the lane; baseline gate failures remain comparison evidence.
The GUI baseline uses checksum-verified public v3.1.3 payloads. Its immutable tag
must resolve to d4896546f190c0e5afe176f36544dca7aa806227 and product source must
match f83ae42 before use; their observed differences are task records. Each
comparison records both source identities and every measured executable hash.
Archive checks reject checksum mismatches, escaping paths and oversized payloads.

Run 35890605196 on 78cc519 records a native musl TUI pass (1.017 percent of one
core, 17.68 MiB peak RSS), then the immutable public GUI baseline exits with
SIGSEGV during orderly shutdown. The original runner aborted before measuring
the candidate GUI. The corrected runner retains that explicit baseline failure
and continues the candidate windows; missing, malformed, oversized or unproven
failure reports and timeouts remain fatal. Native GUI reports preserve the
completed sampling window before shutdown and retain final CPU accounting when
wait4 succeeds. Failed shutdown is still a failed observation. The new reporting
and continuation behavior has eleven deterministic fixtures; native musl
remeasurement is required before claiming a GUI resource result.

`measure-tui-unix.py` uses completed [wait4 resource accounting](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/wait4.2.html).
On Linux, descendant totals require each intervening parent to reap its children;
the [kernel accounting contract](https://man7.org/linux/man-pages/man2/getrusage.2.html)
and a native waited-grandchild fixture validate that assumption. Startup and
orderly-shutdown CPU are included conservatively. Memory/fd counts sum live
descendants every quarter-second, with shared mappings counted more than once
and short-lived peaks potentially missed. This does not substitute for GUI,
physical-device, independent network-wire or long-soak qualification.

Use an isolated development virtual environment with `psutil==7.2.2` and
`pywinpty==3.0.5`. Run `scripts/measure-tui-windows.py` against immutable release
bytes; record the source revision and executable SHA-256. The script accepts
`--seconds`, `--warmup`, `--columns`, `--rows`, `--revision` and `--output`.
The checked-in smoke runs use 30 measured seconds after 10 seconds of warmup,
User Overview at 80 columns by 24 rows, with ordinary live collection enabled.
There were no attached visual observers, other SD-300 instances or concurrent
local compiler/build workloads. Each run starts fresh isolated settings.

The shell is assigned to an ancestor Windows Job Object before it starts the
monitor. Job CPU includes user/kernel time of nested jobs and terminated
descendants, as documented in [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).
100% means one logical core. The idle shell is included consistently. Working
sets and private bytes are summed over live job members every 250 milliseconds;
shared pages can be counted more than once, and short transient peaks between
polls are not captured. Per-role CPU is diagnostic only: the total job accounting
is authoritative because it also includes children exiting between polls.
The script exercises normal quit and alternate-screen restoration, then cleans
up its owned job. No installation, speed test, repair or elevated read is requested.

| Exact candidate | One-core CPU | Peak summed working set | Result |
| --- | ---: | ---: | --- |
| Baseline f83ae42 | 3.74% | 53.1 MiB | CPU gate fails |
| ddbe0eb, first run | 7.52% | 189.2 MiB | CPU and memory fail |
| ddbe0eb, unchanged repeat | 8.08% | 184.9 MiB | Regression confirmed |
| ddbe0eb + PSS-only product change | 4.35% | 219.1 MiB | Startup CPU improves; both gates remain open |
| ebf36c5 + idle worker retirement | 4.82% | 159.5 MiB | Idle memory improves; both gates remain open |
| 7589af5 + native socket collection | 3.11% | 132.9 MiB | CPU remains open; short memory run passes |
| aef8f2e + reusable NVML | 1.92% | 137.3 MiB | Short CPU/memory run passes; full matrix remains open |
| fc3b163 + detached helpers | 1.66% | 100.7 MiB | Short CPU/memory run passes; slower cadence verification follows |

The last candidate's exact binary is identified in its JSON; the source change
is the commit introducing this evidence. Memory peaks depend on whether the
sampling instant overlaps a transient helper. Longer measurements must include
the infrequent inventory/health lanes and preserve cadence and bounded shutdown. Accepted gates stay
at 2% foreground CPU and 150 MiB working set; no waiver is implied.

The 330-second cadence run of unchanged 538c538 bytes crosses health and inventory
refreshes. Its summed working set peaks at 117.3 MiB, but average CPU is 2.65 percent
and fails the gate. `candidate-detached-tui-cadence.json` retains this result.
Short-run acceptance must not substitute for the longer foreground/soak windows.

An unchanged repeat records 2.64 percent CPU and a 152.4 MiB peak during overlapping
infrequent checks. Both gates remain open. Stage profiling on 6b5acc4 identifies
process enumeration (5.57 ms mean) and network collection (4.41 ms mean) as the
largest fast-lane costs; physical topology, preparation and TestBackend drawing
each average under 0.3 ms. Stage wall time is diagnostic, not process CPU.

The bounded-pipe candidate bc13468, measured with the same 330-second method,
reduces average process-family CPU to 2.09 percent and peaks at 133.5 MiB RSS.
`candidate-pipes-tui-cadence.json` identifies its exact release bytes. CPU still
exceeds the two-percent gate; this improvement is not full performance acceptance.

The native-network candidate 5fa678f records 2.07 percent CPU and 116.5 MiB RSS
over the same 330-second window. Its stage time improves substantially but whole
process CPU remains above the gate. `candidate-native-network-tui-cadence.json`
retains that result; further changes must target a measured remaining cost.

The native ICMP candidate 059c3f5 records 2.0206 percent CPU and 139.7 MiB RSS
over the same window. Child creations fall from 73 to seven, but CPU still fails.
`candidate-native-icmp-tui-cadence.json` retains the exact bytes and failed gate.

The native PDH candidate df1a608 completes the same 330-second run at 1.59 percent
CPU, 142.8 MiB RSS and 60.4 MiB private memory, including the five-minute inventory
refresh. All three resource gates pass in this window; the longer prescribed
foreground/hidden/soak qualification is still required. The exact executable hash
and owned-process accounting are in `candidate-native-pdh-tui-cadence.json`.

## Windows GUI process-family measurements

The first full 15-minute foreground Thermals run of aligned 866bebb bytes passes
CPU at 1.17 percent and private memory at 257.7 MiB, but fails summed RSS at
178.7 MiB with eight live processes. It shuts down normally with no owned
children. `candidate-aligned-gui-foreground-15m.json` retains the failure; hidden
and soak windows were not started after the failed gate. The next unchanged
measurement adds peak-role attribution before selecting a product fix.

The native Unix GUI smoke harness verifies complete bundle identity, expected
workers, boundary window visibility and socket-driven normal shutdown. macOS
counts on-screen layer-zero windows through CoreGraphics without capturing their
contents. Linux uses a private Xvfb session and PID-scoped X11 queries; its hidden
check unmaps only the owned window after startup, because Linux has no tray
startup route. This is headless X11 runtime evidence, not physical-display or
Wayland acceptance. Short startup-inclusive smoke CPU is not a long-window gate.

Linux visibility now follows GTK's [mapped surface state](https://docs.gtk.org/gdk4/method.Surface.get_mapped.html)
and [minimized flag](https://docs.gtk.org/gdk4/flags.ToplevelState.html), both
available in the supported GTK baseline. A native GTK fixture exercises hidden
and restored windows. An unavailable startup surface keeps foreground sampling;
the change does not add tray support or treat minimizing as a request to quit.

`scripts/measure-gui-windows.py` requires the GUI, adjacent engine, and the
bundle-relative CLI collector. It records all three hashes and starts the GUI
suspended, assigns its ancestor Job Object, then resumes. Native accounting
fixtures prove exited-child CPU remains counted and owned descendants die on
cleanup. Existing user sessions are left untouched; settings are isolated.
No screenshot, pointer observer, compilation or other measured SD-300 session
runs concurrently. Visibility and required collector processes are checked at
both ends. Normal exit uses the application's existing quit endpoint.

Overview and Processes intentionally subscribe only to fast data plus static
identity; other visible sections enable the full probe lanes. Hidden mode keeps
the lower-frequency thermal/health summary contract. Results name their section
and cannot be generalized to a different subscription. The old PowerShell
main-PID sampler is retained as a diagnostic, not a v4 resource gate.

The initial complete Thermals bundle smoke records 1.40 percent CPU, 127.7 MiB
summed working set and 236.1 MiB private memory. All four persistent probe lanes
are present at both ends and shutdown leaves no owned children. This is a short
diagnostic result; the prescribed long foreground, hidden and soak runs remain
open. An earlier GUI-only stage lacked its collector CLI and is excluded from
composite qualification.

The complete hidden smoke records 0.45 percent CPU, 93.0 MiB working set and
229.1 MiB private memory, with its slow worker present and clean shutdown.
Both smoke JSON reports are retained beside this document. Sampled memory
includes all live job processes, with the same shared-page and polling limits
as the TUI method.

The release `profile-monitor` example accepts `--slow` after its sample count.
It times providers sequentially with two warmup samples and ordinary cadence.
Windows stage CPU uses GetProcessTimes and is quantized; a zero per-stage total
does not establish zero work, and neither external services nor child CPU is
included. Whole-job accounting remains authoritative. Current slow timings
identify GPU collection at 340 ms mean wall time; disk and thermal providers
average 1.3 and 27.6 ms respectively. The retained fast profile identifies process
inventory as the main fast-stage CPU cost. Neither profile is an end-to-end gate.

## Native terminal interaction

Windows lightweight connectivity uses [IcmpSendEcho](https://learn.microsoft.com/en-us/windows/win32/api/icmpapi/nf-icmpapi-icmpsendecho)
inside the owned diagnostic worker. It supplies a three-second native timeout,
checks the reply's IP status and address, and reads only the common address/status/RTT
prefix of the documented reply layouts. The fixed aligned reply buffer includes
the required payload and error-message margin. [Reply RTT is in milliseconds](https://learn.microsoft.com/en-us/windows/win32/api/ipexport/ns-ipexport-icmp_echo_reply);
a reported zero stays below-resolution/unavailable rather than claiming exact zero.
[GetBestRoute](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getbestroute)
selects the route to the lightweight IPv4 probe target; a direct route without a
next hop does not invent a gateway. Native loopback tests validate the API call,
while fixtures cover unreachable/TTL/timeout statuses and mismatched reply addresses.
These tests do not establish end-to-end internet health or sub-millisecond accuracy.

`scripts/qualify-tui-native.py` runs `qualify-tui-pty.py` sequentially in Unicode
and ASCII/no-color modes, followed by the release `profile-monitor` example.
Dependencies are pinned in CI; musl runs inside the native Alpine build container.
Each session uses isolated settings and explicitly enables mouse interaction.
The harness checks all nine sections in both modes at compact and wide sizes,
inspection, full-inventory filtering, Unicode input, repeated navigation, sort
direction, pause/resume, resize recovery, mouse navigation, optional-action consent
dismissal, and normal terminal restoration. It never starts an optional check.

Input timing runs from key write to the decoded terminal change, including PTY
transport and two-millisecond observer polling. Window/terminal setup is reported
separately from monitor-to-chooser startup. Stage drawing uses TestBackend and
does not measure terminal transport. These observers are never attached during
process CPU, memory or soak measurements. Reports contain assertions and timing
metadata, not captured host/process/connection text. Release resource acceptance
and physical display/input checks remain separate.

On Windows, 6b5acc4 passes both modes at both sizes with input p95 near 14 ms and
the chooser visible about 42 ms after launch. The emulator must answer cursor and
device-status requests; omitting those replies falsely added a three-second
ConPTY startup delay. The retained PTY results use the corrected emulator.


## Endpoint inventory qualification

### Windows interface counter qualification

The interface sampler uses [GetIfTable2](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getiftable2)
at its normal statistics level, with the full GUID and LUID as baseline identity.
It does not substitute raw counters below filter modules or a statistics-free table.
The [row flags](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/ns-netioapi-mib_if_row2)
exclude duplicate filter-module and loopback rows. Hardware interfaces contribute
to the aggregate; other virtual/tunnel interfaces retain their own rates. This
scope is visible, including on hosts with no eligible aggregate interfaces.
[Unicast addresses](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getunicastipaddresstable)
join by LUID, with network byte order and IPv6 scope preserved. Address-query
failure is independent of counter availability. Allocated tables are released.

Shared fixtures cover irregular intervals, identity replacement, denied reads,
recovery warmup, measured zero and aggregate exclusion. The native Windows fixture
brackets a separate GetIfEntry2 read between table captures and requires its byte
counter to lie inside that exact interval, skipping identities reset or removed
during the bracket. This is OS API/units consistency, not independent wire-level
measurement.

### Linux and macOS interface counters

Linux reads the kernel's bounded [/proc/net/dev table](https://docs.kernel.org/networking/statistics.html),
which exposes 64-bit byte statistics; it does not use getifaddrs' narrower link
statistics. macOS reads each interface's IFMIB_IFDATA/IFDATA_GENERAL structure
using the published [Apple interface MIB](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/net/if_mib.h)
and libc's typed ifmibdata/if_data64 layout. Names must still match after each
query. This avoids cached values surviving an unreadable counter query.

[getifaddrs](https://man7.org/linux/man-pages/man3/getifaddrs.3.html) supplies
native interface indexes, link addresses and IP addresses, including IPv6 scope.
Its allocation is released and rows are bounded. Baselines include native index,
name and link address; Linux also uses the sysfs node identity when readable.
Counter failures stay per-interface, reset only that baseline, and leave other
readable interfaces available. Cumulative bytes and rates are unavailable in
both frontends and nullable schema-2 exports when their read failed. The default
schema-1 field contract remains unchanged.

The aggregate excludes loopback and names its remaining layer-duplication limit:
bridge/VPN/backing interfaces may account for the same traffic. Native tests on
each Unix runtime keep loopback inspectable and send a bounded known UDP payload
between local sockets. Byte-counter growth must cover that payload; headers and
other traffic prevent treating it as an exact wire-byte comparison. Fixtures
cover values above 32 bits, malformed tables, denial, partial success and recovery.

Windows uses [GetExtendedTcpTable](https://learn.microsoft.com/en-us/windows/win32/api/iphlpapi/nf-iphlpapi-getextendedtcptable)
and its UDP counterpart. Ports and IPv6 scope IDs follow the documented network
byte order; a TCP listener's undefined remote endpoint is not treated as measured.
Allocation and retries are bounded, while table failures retain other successful
families with an incomplete observation. Owned jobs still isolate native calls.

Linux uses [ss](https://man7.org/linux/man-pages/man8/ss.8.html) with numeric
endpoints and no header. On missing iproute2 it reads the documented
[procfs endpoint tables](https://docs.kernel.org/networking/proc_net_tcp.html)
in the current network namespace. This is a compatibility fallback, as the kernel
prefers tcp_diag; owning PIDs are unavailable on that fallback. No automatic
package installation is needed. macOS numeric TCP and UDP parsing follows
[Apple's netstat implementation](https://github.com/apple-oss-distributions/network_cmds/blob/main/netstat.tproj/inet.c).

The native worker fixture holds known loopback TCP listeners/client and UDP
sockets open while collecting, with IPv6 checked when the host supports binding
it. Linux additionally forces missing ss without modifying the host installation.
Parser fixtures cover states, byte order, absent PID, malformed/oversized tables
and partial failure; GUI fixtures preserve the same observation beside rows.


## NVIDIA telemetry

### Windows GPU engine utilization

The Windows worker retains one language-neutral query using
[PdhAddEnglishCounterW](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhaddenglishcounterw)
and reads matching wildcard instances through
[PdhGetFormattedCounterArrayW](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhgetformattedcounterarrayw).
The query uses two completed captures, with at least one second between them as
required by [PDH rate collection](https://learn.microsoft.com/en-us/windows/win32/perfctrs/collecting-performance-data).
Fractions are retained. Process contributions sum within one physical engine;
the busiest engine represents adapter utilization, consistent with
[Microsoft's GPU utilization explanation](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/).
Different engines and adapters are never summed into that percentage.

The provider records its actual interval, resets on changed adapter identities,
explicit retry or clock discontinuities, and backs off native failures. Array
sizes, item counts and string pointers are checked within an aligned bounded
buffer. Invalid instance values make an adapter incomplete rather than adding
zero; departed instances are excluded. WMI remains available when PDH fails,
and existing NVIDIA fields survive missing native values. Both frontends receive
the same per-field observation and source through existing projections.

Native Windows worker qualification exercises first sample, valid second sample
and explicit reset under the owned process deadline. Both local adapters supplied
valid second samples. This verifies the native API path and units, not physical
GPU accuracy; CI without a suitable GPU retains deterministic aggregation,
malformed-buffer, unavailable-field and clock/reset fixtures.

The same eight-sample release-stage diagnostic records GPU collection falling
from 340.4 to 2.9 ms mean wall time. `profile-slow-native-pdh.json` retains the
result. This does not establish the whole-product CPU gate; that requires the
unchanged process-family measurement method on the committed candidate.

### NVIDIA provider

Bindings follow the [NVIDIA NVML API](https://docs.nvidia.com/deploy/nvml-api/nvml-api-reference.html)
and [published header](https://github.com/NVIDIA/go-nvml/blob/main/pkg/nvml/nvml.h).
The driver session stays in the isolated worker. Windows loads only from System32;
Linux uses the platform loader's versioned libnvidia-ml.so.1. Missing/older driver
libraries retain nvidia-smi fallback; no driver is installed or changed.

ABI assertions cover the PCI, memory and utilization structures. Synthetic
function tables cover identical model names with distinct UUID/PCI identities,
permission denial, unsupported fields, true zero utilization, allocation versus
reservation, and invalid results. Inaccessible measurements remain nullable.

Local Windows comparison with nvidia-smi first identified a reserved-memory
semantic mismatch in NVML v1. The accepted path requires nvmlDeviceGetMemoryInfo_v2
and uses its allocated bytes, converted to MiB. Three bracketed observations are
retained in nvml-windows-consistency.json with predeclared tolerances. They agree
within one MiB and exactly in temperature. Both interfaces use the same driver;
this establishes interface/units consistency on one GPU, not independent physical
accuracy or proof for unavailable hardware. macOS retains its Metal provider.

## Windows process CPU units

[GetProcessTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocesstimes)
reports summed thread durations in 100-nanosecond units. Divide their delta by
the monotonic interval of that process's captures, with 100 percent meaning one
logical processor and multithreaded values allowed above 100. Do not rescale using
the monitor's available parallelism: [GetSystemTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getsystemtimes)
sums all processors on smaller hosts but only the calling thread's primary group
above 64 processors, while affinity can independently restrict the monitor.

Deterministic fixtures cover irregular intervals, counter rollback, PID reuse,
first readings, long gaps, real zero and multithreaded work. A child-only native
fixture restricts its own affinity, performs a bounded workload, and compares the
batched sampler with a separate GetProcessTimes bracket. The predeclared absolute
tolerance is ten percentage points for clock quantization and the surrounding
inventory calls. This verifies API/unit consistency, not hardware instrumentation.
