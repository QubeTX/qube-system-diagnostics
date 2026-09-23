# SD-300 v4 qualification evidence

These initial measurements are diagnostic runs, not release acceptance. The
foreground/hidden/soak matrix, native comparisons and installer qualification
remain open. Do not infer hardware accuracy from parser fixtures or builds.

## Windows TUI process-tree measurements

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

## Native terminal interaction

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
measurement. Existing Linux/macOS providers continue through the shared sampler;
their aggregate explicitly describes the sum of reported interfaces and possible
duplicate traffic across layers.

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
