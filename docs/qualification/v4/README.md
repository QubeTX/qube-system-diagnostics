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

The last candidate's exact binary is identified in its JSON; the source change
is the commit introducing this evidence. Memory peaks depend on whether the
sampling instant overlaps a transient helper. The next cycle targets idle worker
working sets; it must preserve cadence and bounded shutdown. Accepted gates stay
at 2% foreground CPU and 150 MiB working set; no waiver is implied.
