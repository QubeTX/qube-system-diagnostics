# SD-300 native GUI information architecture

Status: implementation specification for the v3 Native GUI. This document does
not authorize collector, CLI, or TUI behavior changes.

## Product rule

Every resource page answers four questions in this order:

1. **Status:** Is this resource healthy, constrained, unavailable, or stale?
2. **Evidence:** What is its utilization, saturation, error state, and recent
   direction?
3. **Cause:** Which core, device, process, interface, sensor, or driver explains
   the condition?
4. **Inventory:** What hardware and provider produced the result?

The User and Technician views use the same collector topics. User view adds
plain-language interpretation and reduces initial density; Technician view
surfaces exact values, provenance, and larger tables. Neither view invents a
value when the collector reports unsupported, unavailable, permission denied,
not detected, stale, or scanning.

This ordering follows the USE method for hardware dashboards (utilization,
saturation, errors), Grafana's general-to-specific drill-down guidance, and
Carbon's recommendation to place the strongest signal first while preserving
search, sort, filter, and drill-down for exploratory dashboards.

## Shared interaction and chart rules

- Data collection keeps the existing TUI cadence. Foreground presentation may
  coalesce work but must show the newest completed sample at least once per
  second; it never queues old frames.
- One current value, a short bounded history, and an explicit state label form
  the primary answer. Color is supplementary and never the sole state signal.
- Line/area charts represent time; ranked bars represent comparisons. Axes use
  stable, meaningful ranges so visual movement is not exaggerated.
- Peaks, warnings, scans, provider changes, and gaps are annotated. A missing
  interval is not silently connected as if it were measured.
- Tables have meaningful initial sorting, keyboard focus, visible column labels,
  and a nonvisual text equivalent for chart-only conclusions.
- Large process, connection, device, and driver collections are virtualized or
  bounded. Filtering operates on the underlying bounded projection rather than
  producing additional unbounded render state.
- Every live panel exposes capture time/freshness on demand. Provider and
  capability provenance belongs in the detail rail, not ahead of the health
  answer.

## Page specifications

### Overview

Primary: overall collection health, actionable warnings, and the most likely
current bottleneck. Secondary: CPU, memory, disk, GPU, network, and thermal
summary cards with current value, direction, and freshness. Detail: recent
warning timeline, unavailable capabilities, ABI/schema/version, and provider
provenance.

Do not treat a large percentage as inherently unhealthy without context. A
resource card explains whether the reading is sustained, near capacity, or
paired with saturation/errors.

### CPU

Primary: total utilization plus a 60-second trace and current clock/load context.
Secondary: per-core small multiples or ranked bars, followed by the top CPU
consumers. Detail: processor model, physical/logical topology, frequency, and
provider provenance.

The process CPU label must state its denominator: percent of one logical core,
or percent of total machine capacity. The GUI must not present the first sample
after process discovery as an established sustained value.

### Memory

Primary: memory in use, expressed as `used of total`, with pressure/status.
Secondary: available memory, commit/page-file or swap state, and a bounded
history. Detail: physical module inventory, speed, manufacturer, and part number.

Physical capacity is context, not the lead metric. Page-file/swap use is not
described as a defect on its own; the interpretation combines pressure,
available memory, and sustained paging when those signals exist.

### Disk

Primary: separate activity, capacity, and health answers. Activity leads with
throughput, latency, queue/saturation, and errors when available. Secondary:
volume used/free capacity and read/write direction over time. Detail: physical
media, filesystem/mount, SMART or platform health, temperature, and provenance.

Capacity percentages must never be labeled as disk activity. Until activity and
latency collectors exist, the GUI explicitly marks those signals unavailable
instead of inferring them from used space.

### GPU

Primary per adapter: utilization, VRAM in use, and current telemetry
availability. Secondary: temperature, power, clocks, and fan state when the
provider supports them. Detail: driver, display linkage, memory capacity,
adapter identity, and source.

Integrated and discrete adapters remain separate. `Inventory only` is a first-
class state; zero is not substituted for unavailable telemetry.

### Network

Primary: active interface and current receive/transmit rate with short histories.
Secondary: gateway, DNS, and internet reachability/latency plus interface errors
when available. Detail: interface inventory and a searchable/filterable active-
connection table.

Rates and cumulative totals use distinct labels and units. Connection rows show
protocol, local/remote endpoint, state, owning PID/process when resolved, and
capture freshness.

### Processes

Primary: searchable, sortable consumers with CPU, memory, PID, and state. The
default order is CPU descending, with stable tie-breaking to prevent constant
row jitter. Secondary: filters for active/user/system processes and a selected-
row detail panel. Detail may include executable identity, threads, handles/file
descriptors, start time, I/O, and command line only when collected and safely
redacted.

Newly observed or just-reset CPU samples are labeled `warming up` rather than
promoted as a reliable spike. SD-300's own row is measured with the same process
collector semantics as every other row. A renderer-performance diagnostic may
show foreground repaint cost separately, but never edits the process reading.

### Thermals

Primary: hottest meaningful sensor, status, and available thermal headroom.
Secondary: bounded histories and grouped CPU/GPU/storage/board sensors. Detail:
fan readings, thresholds, provider, permission, and availability state.

Thresholds are provider/hardware-specific. When authoritative thresholds are
not known, the GUI shows the measured temperature without inventing a health
classification.

### Drivers

Primary: attention count and actionable problem rows. Secondary: scan state,
filters, search, category, problem code/status, and remediation context. Detail:
version, provider, device identifiers where safe, and SetupAPI/sysfs/IOKit
provenance.

Attention rows precede healthy inventory. Hundreds of healthy devices remain
available through drill-down rather than obscuring the action state. Manual
rescans stay asynchronous and expose scanning, completion, and failure states.

## Performance contract for live data

Collection and presentation are separate clocks with latest-value semantics:

```text
existing collectors -> versioned latest-only topics -> current GUI model
                                              |-> one demand-driven presentation
                                                  for the newest changed state
```

- The engine continues the existing 1/3/5/15/60-second collector schedule.
- The foreground UI consumes the newest fast sample every second.
- A slow renderer drops superseded visual work; it never delays the collector or
  accumulates a frame queue.
- Retained command comparison, dirty rectangles, memoized text/paths, partial
  pixel conversion, and partial OS invalidation are the optimization levers.
- Hiding the window suspends presentation work and selects only tray-required
  topics. This is not used to disguise foreground performance.
- Qualification measures release binaries in the foreground and hidden/tray
  states independently. A short diagnostic sample is evidence, not a substitute
  for the 15-minute, 30-minute, and two-hour release gates.

## Field-parity audit

This is the release checklist, not a claim that an export alone constitutes a
usable GUI representation. A field may live on a more appropriate GUI page
than its TUI location, but it must remain discoverable. A bounded table must
state its total and provide filtering, paging, or an export path rather than
silently implying that the visible rows are the complete inventory.

| TUI / collector surface | GUI destination | Current state | v3 release action |
|---|---|---|---|
| Mode chooser and User / Technician interpretation | Settings plus page-specific hierarchy | Implemented; GUI state is isolated from TUI defaults | Verify restart persistence and both modes on every page |
| Temperature unit | Settings, Thermals, drive health | Implemented and GUI-only | Verify Celsius/Fahrenheit across every temperature |
| Warnings: source, severity, message, deduplicated count | Overview health panel and capability detail | Implemented with explicit shown/total bounded count and redacted export | Test every severity/unavailable state and omitted-row count |
| Topic schema, sequence, capture time, freshness, availability, provenance | Top rail and contextual detail | Implemented in the global live-topic rail with age/stale evaluation and Technician provenance/target | Automate current/stale/unavailable states at cadence boundaries |
| System identity: OS/version/kernel, host, manufacturer/model, BIOS, architecture, uptime, hypervisor | Overview | Implemented, including known/unknown hypervisor state in Technician mode | Verify unavailable and present states on native targets |
| CPU total, model, physical/logical topology, frequency, per-core load, top consumers, 60-s history | CPU plus Processes | Implemented with a CPU-ranked cause jump into the full process table | Verify the jump and one-core denominator in both modes |
| Memory used/total/available, pressure, swap, module capacity/type/speeds/vendor/part, top consumers, 60-s history | Memory plus Processes | Implemented with physical/swap history and memory-ranked cause jump | Verify zero-swap and partial-module-provider states |
| Partition name/mount/filesystem/type/removable and used/free/total | Disk | Implemented with explicit removable-media labels | Verify removable and zero-capacity fixtures |
| Physical drive model/media/health/temp/wear/power hours/source | Disk | Implemented | Test every unknown/unavailable combination |
| Disk throughput, queue depth, read/write latency, reliability/error availability, read/write history | Disk | Implemented from existing health I/O/reliability fields with explicit unavailable state and separate capacity/activity panels | Verify each provider combination and the 60-second health cadence |
| GPU inventory/status, per-adapter load/VRAM/temp/driver/resolution/refresh/source | GPU | Implemented with bounded primary-adapter utilization history when telemetry exists | Verify inventory-only adapters never contribute false zeroes |
| Display active state, connection, brightness, physical dimensions, source | GPU | Implemented with active/inactive/unknown labels | Verify unknown is never converted to inactive |
| Network aggregate/interface rates, cumulative totals, operational state, IP/MAC | Network | Implemented with bounded download/upload rate histories and distinct cumulative totals | Verify scale behavior for idle and burst traffic |
| Platform adapter description/status/link speed/hardware classification | Network Technician | Implemented with the provider status labeled as media state | Verify unsupported and unknown provider states |
| Gateway, DNS, internet targets/results/errors/latency | Network | Implemented | Avoid rendering `0 ms` when latency is unavailable; test success/failure/timeout |
| Active/listening connections: protocol/endpoints/state/PID/process | Network Technician | Implemented as a filterable 20-row bounded view with matched/shown/total counts | Verify filtering and the three-second cadence with more than 20 fixture rows |
| Process name/friendly name/PID/CPU/memory/state, total count/threads, CPU/memory/PID/name sorting | Processes | Implemented as stable/filterable top 8/16 with explicit first-sample warming state | Ensure SD-300's own row is never special-cased and automate text editing |
| Thermal CPU/GPU summary, sensor label/kind/temp/critical/source, fans/RPM/source | Thermals | Implemented with bounded primary temperature history and provider availability | Verify both units, partial providers, and unknown thresholds |
| Battery percent/charging/AC/time/capacity/design voltage/cycles/provider and power source | Thermals | Implemented | Test no-battery, partial-provider, and permission-denied states |
| Driver scan state; every device category; attention precedence; name/version/date/status/detail | Drivers | Implemented as attention-first, searchable, attention-filterable 32-row projection with shown/total semantics and snapshot export | Add fixture automation for category/status combinations and scan failure |
| Platform service name/display name/running state associated with driver categories | Drivers Technician | Implemented with running/not-running distinction | Verify platform-specific empty and partial service inventories |
| Help/key descriptions and lifecycle instructions | Settings/About | Implemented for GUI navigation, focus, activation, charts, tray/close behavior, CLI lifecycle, and unchanged TUI help | Physically verify keyboard focus and screen-reader semantics |
| CPU, memory, swap, network down/up, disk read/write, GPU, and temperature history visuals | Resource pages | Implemented as fixed 60-sample arrays updated only when each topic changes | Verify missing intervals, provider loss, scaling, and two-hour bounded growth |
| Redacted snapshot and capability/provenance export | Settings | Implemented asynchronously | Verify output permissions, corruption handling, path reporting, and uninstall preservation |

The checklist is complete only when automation proves each implemented row in
both audience modes and exercises unsupported, unavailable, permission-denied,
not-detected, stale, and scanning states. Hosted builds alone do not close
visual or physical interaction evidence.

## Primary references

- [Grafana dashboard best practices](https://grafana.com/docs/grafana/latest/visualizations/dashboards/build-dashboards/best-practices/)
- [IBM Carbon dashboard guidance](https://carbondesignsystem.com/data-visualization/dashboards/)
- [IBM Carbon accessibility guidance](https://carbondesignsystem.com/guidelines/accessibility/developers/)
- [GOV.UK data visualization guidance](https://brand.design-system.service.gov.uk/data/charts/)
- [Atlassian data visualization color](https://atlassian.design/foundations/color/data-visualization-color)
- [Apple Activity Monitor process guidance](https://support.apple.com/guide/activity-monitor/view-information-about-processes-actmntr1001/mac)
- [Microsoft memory performance information](https://learn.microsoft.com/en-us/windows/win32/memory/memory-performance-information)
- [Microsoft page-file introduction](https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/introduction-to-the-page-file)
