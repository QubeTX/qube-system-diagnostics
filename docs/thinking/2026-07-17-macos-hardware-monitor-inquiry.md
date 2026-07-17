# Critical Thinking Session — macOS Hardware Monitor Capability Inquiry

**Date:** 2026-07-17
**Framework:** Scientific Inquiry
**Mode:** Self-check
**Stacked skills:** logical-reasoning (Explain / inductive-evidence audit)

---

## Pre-Flight: Inputs Inspected

### Inputs brought to the session

- The current SD-300 repository at version 1.4.3: internal artifact; current implementation and documentation are evidence of what exists, not a specification for the redesign.
- The operator's situation description: this borrowed Mac is available only briefly; the Alienware/Windows environment will host most implementation work; this Mac should therefore be used now for evidence that a Windows-only agent cannot reproduce.
- The operator's clarified product direction: the TUI and CLI remain the primary interface; a cross-platform native GUI is an optional experiment, possibly using a Vercel-associated SDK with Zig underneath.
- Prior project memory: related macOS qualification work suggests that native execution and inspection of real output can expose platform-specific behavior missed by cross-platform unit tests. This is a search lead, not evidence about SD-300.

### Source pass findings

- Repository code and commands run on this host can directly establish behavior on this exact OS/hardware/user/permission combination, but cannot by themselves establish behavior on every Mac model or macOS release.
- Command output may contain owner identifiers, serial numbers, MAC addresses, IP addresses, device UUIDs, account names, and location-adjacent data. The durable report must record schemas and redacted examples rather than raw identifying values.
- Vendor documentation can establish API contracts and entitlement requirements, but not prove that this repository currently uses them or that a command behaves identically on this machine.
- “Native SDK” is not yet identified. Vercel affiliation, Zig usage, platform coverage, and whether it uses native widgets or a webview are open claims to test rather than assumptions to repeat.

### What's already decided (not revisiting)

- Do not implement the major monitor redesign during this inquiry.
- Preserve the TUI and CLI as the primary product surfaces.
- Treat a GUI as an optional experimental frontend over a reusable collector/core layer.
- Use safe, read-only, bounded probes; do not change permissions, request sudo interactively, install system extensions, run stress workloads, or expose secrets/unique hardware identifiers.
- Produce a durable report usable by the Windows-side implementation agent and a later Mac validation pass.

---

## Working Sections

### Facts

| Fact | Confidence | Source / surfaced at |
|---|---|---|
| The checked-out repository is on `main` at tagged version `v1.4.3`, initially clean and aligned with `origin/main`. | High | Git baseline, 2026-07-17 |
| The existing product is a Rust/Ratatui TUI with separate User and Technician render paths and platform-specific collector branches. | High | `AGENTS.md`, `CODEX_PROJECT.md`, source tree |
| This inquiry is intentionally limited to evidence collection and documentation, not redesign implementation. | High | Operator constraint |

### Assumptions

| Assumption | Status | Surfaced at | Notes |
|---|---|---|---|
| Most useful Mac hardware signals are available to an ordinary process through stable public APIs or bounded system commands. | open | Pre-flight | H1; test by breadth probes and documentation. |
| A substantial subset of high-value telemetry is privileged, entitlement-gated, private/unstable, model-specific, or unavailable. | open | Pre-flight | H2; test access failures and official API boundaries. |
| One Mac's outputs are sufficient to design a general macOS backend. | open, expected false | Pre-flight | H3 challenges this convenience-sample inference. |
| The recalled Vercel-associated, Zig-backed SDK is appropriate for a truly native cross-platform GUI. | open | Operator clarification | Identity and technical claims require primary-source verification. |

### Constraints

- **Time-limited Mac access:** prioritize observations impossible or awkward to obtain from Windows later.
- **Borrowed machine:** no invasive changes, elevated prompts, stress tests, persistent agents, or durable capture of personal identifiers.
- **Cross-platform future:** separate machine-specific observations from macOS-wide contracts and from architecture recommendations.
- **Primary TUI/CLI:** GUI research must not force collector logic into a GUI framework or weaken headless use.
- **Evidence quality:** command availability is not equivalent to a supported API; a successful observation is not proof of universal availability.

### Open questions

- Which telemetry domains are observable unprivileged on this exact Mac, through which interfaces, with what latency and output stability?
- Which signals require root, Full Disk Access, special entitlements, private frameworks, model-specific keys, or third-party helpers?
- Which existing collectors silently degrade, mislabel, block, or omit data on macOS?
- How do Apple Silicon and Intel Macs, desktops and laptops, and current vs older macOS releases differ?
- What framework matches the Vercel/Zig clue, and is it suitable only as an experimental view layer?

### Tensions

- **Exhaustive technician data vs safe public interfaces:** the most detailed readings may rely on private or privileged mechanisms that are unsuitable for a broadly distributed monitor.
- **Convenient command wrappers vs durable product APIs:** shell probes are fast to validate but can be localized, schema-unstable, slow, deprecated, or privilege-gated.
- **Native GUI experimentation vs a stable cross-platform core:** framework enthusiasm must not duplicate or strand collector logic.

### Deferred items

- Major collector/UI architecture changes: deferred to the Alienware implementation phase.
- Any root-only proof requiring an actual sudo prompt: deferred to a later explicitly supervised Mac test.
- Intel-Mac and other-model validation: cannot be established from this host unless the host happens to provide that architecture; retain as a named fleet gap.

### Connections

- The durable output should function as both a capability matrix and a test-fixture plan: interface, privilege, cadence, cost, sensitivity, fallback, and confidence per signal.

---

## Framework Steps

### Step 1: Observe / Question

**Sub-questions asked:** What exactly is observed? When and where is it measured? What expectation is being challenged? What falsifiable question can observations settle?

**Responses:** The observed constraint is not a monitor defect but an evidence-window problem: a macOS host is temporarily available while most future coding will happen on Windows. The expectation under test is that generic documentation and existing cross-platform crates provide enough macOS knowledge. The falsifiable inquiry is: **On this exact Mac, which bounded interfaces expose each hardware-monitor signal to an ordinary process; which fail, degrade, leak sensitive data, require privilege/entitlements/private APIs, or vary in shape; and what evidence is still required before generalizing to macOS as a platform?**

**Insights:** A complete result is not “a list of Mac commands.” It must preserve command/API provenance, access level, observed output shape, performance/cadence, privacy risk, portability limit, and a later validation recipe.

**Mode:** Convergent

### Step 2: Research — inquiry plan

**Sub-questions asked:** What internal and external sources can settle each claim? What is the system of record? Could the evidence be stale?

**Responses:** Internal sources are current repository code, tests, CI, dependencies, and real execution. External systems of record are current Apple developer/manual documentation and the identified GUI framework's own source/docs. Empirical command results are dated host snapshots. Secondary articles may suggest probes but cannot settle API-support or entitlement claims.

**Insights:** Cross-check the highest-impact claims with two evidence classes where possible: observed host behavior plus a primary contract/source. Treat undocumented/private interfaces as explicitly unstable even when they work today.

**Mode:** Divergent

### Step 3: Competing Hypotheses

| Hypothesis | Prediction | Falsifier | Prior |
|---|---|---|---|
| **H1 — public/unprivileged sufficiency:** stable public APIs or ordinary bounded commands cover most useful monitoring signals. | Broad probes succeed without elevation and expose documented, parseable schemas. | Core domains such as thermal sensors, fan RPM, per-process energy, SMART/NVMe health, or radio state prove inaccessible/undocumented. | Medium |
| **H2 — layered access is necessary:** valuable depth is split among public APIs, privilege-gated tools, private/model-specific interfaces, and unavailable signals. | A capability matrix shows clear tiers and required fallbacks; some failures are access-policy rather than implementation defects. | Nearly all desired signals have stable documented unprivileged APIs with adequate cadence. | High |
| **H3 — convenience-sample danger:** this host cannot justify universal macOS claims. | Architecture/model/form-factor/OS-specific branches and missing device classes remain after exhaustive local probing. | Official contracts and multiple independent fixtures establish invariant behavior across supported Macs. | High |
| **H4 — optional GUI can remain decoupled:** the candidate SDK can consume a stable Rust core or IPC boundary without replacing TUI/CLI. | It supports Rust FFI/sidecars or a clean local protocol, plus viable platform packaging. | It requires moving collector/business logic into framework-specific code or lacks supported target OSes. | Medium |

**Mode:** Divergent

### Step 4: Experiment — predictions committed before execution

**Sub-questions asked:** What is the cheapest decisive test? Is it safe and reversible? What results distinguish the hypotheses? What is the timebox?

**Responses:** Run a broad but bounded read-only probe matrix as the current user; use noninteractive privilege checks only; record exit status, latency, schema/field presence, and sanitized samples. Build/run existing code and targeted tests without altering system state. For APIs not exposed by existing tools, compile temporary throwaway probes outside tracked source where the evidence value justifies it. A mix of successes and policy/model-specific failures supports H2/H3; broad stable documented success supports H1; framework source/docs and a minimal integration model distinguish H4.

**Safety:** Do not invoke interactive sudo, change TCC/SIP/entitlements, load extensions, install monitoring agents, scan personal file content, or persist raw identifiers.

**Mode:** Convergent

---

## Visual Models In Play

### Evidence classification matrix

| Evidence class | Can establish | Cannot establish alone |
|---|---|---|
| Current repo/code | Existing behavior and intent | Runtime availability on every Mac |
| This-host observation | Exact behavior here, now, under this user | Universal macOS behavior |
| Official public docs/headers | Supported contract and declared access requirements | Actual behavior on this machine |
| Open-source/private-interface implementation | Practical technique and field clues | Stability, App Store suitability, future compatibility |
| Cross-machine fixture | Model/OS contrast | All possible Mac configurations |

---

## Steel-Manned Dissent — initial

- **The case against this breadth:** a Windows agent can read Apple docs later; spending limited Mac time on dozens of probes risks producing a command catalog rather than product insight.
- **What would have to be true:** documented APIs would be complete and reliable, existing cross-platform abstractions would faithfully expose them, and local execution would add little information.
- **How it is handled:** prioritize probes that reveal actual permissions, missing fields, output shapes, cadence/cost, localization, model dependence, or current collector degradation; keep generic background research subordinate to reproducible evidence.
- **Confidence in this handling:** High.

---

## Evidence Batch 1 — host baseline and current implementation

### Observed host facts

| Finding | Confidence | Evidence |
|---|---|---|
| The host is a MacBook Pro `Mac14,7` with an Apple M2, running macOS 26.3.1 / Darwin 25.3.0 natively as arm64 (not Rosetta). | High | `sw_vers`, `uname`, `arch`, `sysctl`, sanitized `system_profiler` |
| It has 8 CPU cores: 4 Performance and 4 Efficiency; the clusters expose different L1/L2 sizes through `hw.perflevel*`. | High | Local `sysctl -a` |
| It has 8 GiB unified memory and a 10-core integrated Apple M2 GPU; `system_profiler` reports Metal 4 family support. | High | Local `sysctl` and sanitized `system_profiler` |
| The internal display is a built-in Retina LCD at 60 Hz, but three resolution concepts disagree: a 2560x1600 panel label, a 2880x1800 backing-pixel field, and a 1440x900 logical/scaled mode. | High | Local sanitized `SPDisplaysDataType`; CoreGraphics probe still pending |
| The internal 500 GB Apple SSD is on `Apple Fabric`; `diskutil` and `SPNVMeDataType` report solid state, SMART `Verified`, TRIM `Yes`, model/firmware, and size without elevation. | High | Local sanitized `diskutil` and `system_profiler` |
| The battery was healthy, at 100%, with 114 cycles and 88% maximum-capacity health; AC adapter metadata and charge state were readable without elevation. | High | Local sanitized `SPPowerDataType` |
| macOS reports no recorded thermal, performance, or CPU-power warning through `pmset -g therm`. | High | Local command; this is a status flag, not sensor telemetry |
| `powermetrics` advertises tasks, battery/backlight, network, disk, interrupts, CPU power/frequency, thermal pressure, GPU power/frequency, and ANE power/frequency samplers plus machine-readable NUL-separated plist output, but requires superuser on this host. | High | Local `powermetrics -h` and non-root access test |
| I/O Registry contains one `AppleSMC` service and `AppleSMCKeysEndpoint`, `AppleSMCPMU`, `AppleSMCSensorDispatcher`, and `AppleSMCChargerUtil` classes. No fan-specific class or RPM value was found in the first unprivileged class scan. | High for observed registry; Low for any conclusion about physical fan count | Local sanitized `ioreg` |
| Wi-Fi profiler data exposes live interface state, PHY, channel width, MCS, negotiated rate, RSSI/noise, security, capabilities, firmware, supported channels, and nearby-network count. The full local pass took about 7 seconds. | High | Local sanitized `SPAirPortDataType` |
| Bluetooth profiler exposes controller chipset, firmware, transport, state/services, and connected/remembered devices. | High | Local sanitized `SPBluetoothDataType` |
| `system_profiler` is privacy-dangerous by default: Bluetooth device names are JSON object keys, and multiple profiler types include serials, UUIDs, network names/addresses, scheduled wake metadata, or other owner-specific data. “List JSON paths” is not safe because names can occur in keys. | High | Direct local schema inspection; raw identifiers intentionally excluded from durable files |

### Current SD-300 runtime findings on this Mac

| Finding | Confidence | Evidence |
|---|---|---|
| The existing collectors completed a sanitized full pass in about 4.1 seconds after build; second fast refresh was about 19 ms, slow refresh 92 ms, TCP connections 116 ms, connectivity 207 ms, disk health 611 ms, and drivers 1.35 s in that run. | High for this run; Medium for representative latency | Temporary path-dependent probe against the current library |
| The app enumerated 33 temperature components through `sysinfo`, yet derived CPU and GPU temperatures were both `None` because labels are `PMU*`, `NAND CH0 temp`, and `gas gauge battery`, not the current `cpu|tctl|coretemp|package` or `gpu|nvidia|radeon` patterns. | High | Live collector output plus `thermals.rs` label logic |
| Live temperatures included 16 `PMU tdie*` / `PMU2 tdie*`, 14 `PMU tdev*` / `PMU2 tdev*`, calibration readings, NAND, and battery. The hottest initial reading was about 80.9 C. Two `tdev` readings were physically implausible at roughly -1.4 C, and the collector currently accepts them. | High for observed values; Low for mapping each opaque sensor to a component | Live sanitized sensor probe |
| The `sysinfo` Apple Silicon component backend obtains these readings from `IOHIDEventSystemClient` temperature services and returns product labels; this is already present transitively in the dependency, not in SD-300's own macOS code. | High | Locally installed `sysinfo 0.39.1` source |
| All eight reported per-core frequencies were exactly 3504 MHz. Local dependency source shows Apple Silicon `sysinfo` reads the final `voltage-states5-sram` entry and returns a shared/max value, not live per-core clocks. | High | Runtime output plus local `sysinfo 0.39.1` source |
| The current app labels that repeated value as current speed and can say “Running at full speed,” which is misleading on this host. | High | Live output semantics plus current CPU UI/source |
| The existing GPU collector only invokes `nvidia-smi`; therefore it reported no GPU, 0% utilization, no memory, and no temperature despite the locally confirmed 10-core Apple M2 GPU. | High | `gpu.rs` and live current-library probe |
| The thermal collector returned no fan, no battery, and unknown power source because non-Windows battery collection is hard-coded to `None` and fans are empty outside Windows. | High | `thermals.rs` and live current-library probe |
| Current disk health returned one SSD but health `Unknown`, no temperature, firmware, power-on hours, or I/O statistics even though unprivileged native commands reported SMART `Verified`, TRIM, model, and firmware. | High | Live current-library probe vs local native commands |
| The current driver scan populated network/Bluetooth/audio/input only, left display/storage/USB/system empty, returned `NotScanned` after completion, and marked both hard-coded daemons stopped even though the actual Bluetooth and Core Audio daemons were running. | High | Live probe, process check, and macOS driver source |
| The network collector returned 28 interfaces, including many `utun`, AWDL, bridge, loopback, and Apple-internal interfaces. The app's heuristic does not identify physical `en0` as Wi-Fi and treats interface presence too loosely as connectivity. | High | Live probe and current UI/source |
| The macOS TCP collector returned 92 connections and 19 listeners but no PIDs/process names; this is a limitation of the current `netstat` parser route. | High | Live probe and current source |
| macOS `ping -W` uses milliseconds; the current non-Windows `-W 3` intends three seconds but actually requests about three milliseconds. The displayed latency is subprocess wall time rather than parsed ICMP RTT. | High | Local `man ping` contract and current source |
| `cargo test --locked --all-targets` passed all 21 tests, but coverage is chiefly parser/CLI/update fixtures and does not validate live Mac collectors or TUI rendering. | High | Local test run and test inventory |

### Privacy boundary discovered

- Do not preserve raw `system_profiler` Bluetooth, Wi-Fi, hardware, display, NVMe, power, network, camera, USB, or Thunderbolt JSON as default diagnostics.
- A future export layer needs field-level allowlists, deterministic redaction, and an explicit sensitive-data opt-in; removing values while retaining object keys is insufficient.
- Process names, mount/volume labels, socket endpoints, network names, Bluetooth names, hardware serials, display IDs, MAC addresses, UUIDs, and scheduled events are sensitive by default.

## Step 5: Analyze — interim hypothesis matrix

| Evidence | H1 public/unprivileged sufficiency | H2 layered access necessary | H3 convenience-sample danger | H4 optional GUI decoupling |
|---|---|---|---|---|
| 33 unprivileged temperature channels | Consistent | Consistent | Diagnostic: opaque/model labels | Neutral |
| No unprivileged `powermetrics` | Inconsistent for deepest telemetry | Diagnostic support | Consistent | Neutral |
| Rich unprivileged battery/disk/radio/profile data | Support | Consistent with public tier | Consistent | Neutral |
| Private/opaque IOHID and `pmgr` dependency techniques | Weakens stability claim | Diagnostic support | Diagnostic support | Neutral |
| Existing collectors omit/misclassify locally observable data | Inconsistent with abstraction sufficiency | Support | Support | Supports keeping views decoupled from collector internals |
| Profiler latency and identifier exposure | Inconsistent with naïve command polling | Diagnostic support for cadence/privacy tiers | Support | Neutral |

**Interim inference-to-best-explanation:** H2 currently has the greatest scope and least inconsistent evidence: a serious macOS backend needs explicit capability/access tiers rather than one abstraction or one command family. H3 also survives strongly: this M2 MacBook Pro reveals Apple Silicon/laptop behavior but cannot settle Intel, fanless, desktop, multi-GPU, external-device, or newer-SoC cases. H1 survives only in the qualified form “many useful static and moderate-cadence signals are unprivileged.” H4 remains open pending framework identification.

**Logical-reasoning audit:** Passing one local probe confirms possibility on this host, not universal support (`observed here -> possible here`, not `observed here -> all Macs`). A failed derived metric does not imply absent hardware when raw channels or independent interfaces contradict it. These guard against hasty generalization and argument from ignorance.

**Mode:** Convergent

---

## Evidence Batch 2 — direct native/private probes

### Observations that changed the interim view

| Finding | Confidence | Evidence / implication |
|---|---|---|
| A direct ordinary-user IOHID probe matched 38 temperature services and read all events in roughly 46–55 ms, while the current `sysinfo` view returned 33 channels. | High on this host | Private IOHID calls; the transitive dependency is useful but filtered, and neither label set is a universal semantic map. |
| Read-only AppleSMC access worked without root. `FNum` reported one fan; actual/target/min/max were about 2985/2998/1199/7199 RPM, mode automatic. | High on this host | Direct IOKit user-client calls; contradicts the weaker interim expectation that fan RPM would necessarily require root or a third-party executable. |
| AppleSMC key casing and ABI placement matter: `F0Md` existed while lowercase variants did not; placing data size in the wrong struct member yielded SMC status `0x89` despite IOKit success. | High on this host | Direct controlled read-only probe; supports an exact ABI specification and two-layer error checks. |
| Private `libIOReport` worked unprivileged: 7,923 channels/119 groups were enumerable; a narrow Energy Model subscription and one-second deltas returned energy and CPU/GPU state residency. | High on this host | Direct dynamic-library probe; private does not imply privileged, but breadth makes unrestricted collection costly and privacy-dangerous. |
| A loaded one-second CPU Energy delta was about 14,905 mJ over 1.008 s (~14.8 W). | High as a sampled calculation; Low as a baseline | IOReport counter delta. This establishes a live route, not device health or cross-model comparable accuracy. |
| IODeviceTree DVFS tables exposed exact E/P/GPU frequency states, while frequency sysctls were empty/unsupported. | High on this host | Local `pmgr` properties plus IOReport state counts; supports frequency residency but is firmware/model-specific. |
| Public Metal, CoreGraphics/AppKit, CoreWLAN, IOPowerSources-like data, registry block counters, and private AGX performance properties supplied a far broader unprivileged baseline than SD-300 exposes. | High on this host | Temporary native probes and allowlisted registry reads. |

### Revised access inference

The early broad scan correctly found no fan property in a normal registry dump but was not decisive. The direct AppleSMC user-client experiment falsified “ordinary unprivileged fan telemetry is unavailable on this host.” The narrower supported claim is:

> Fan RPM is locally available to an ordinary process through an unpublished, model-sensitive read-only AppleSMC ABI; public/support/distribution portability remains unproven.

Likewise, root `powermetrics` remains useful but is not the only path to deep energy/frequency data because unprivileged private IOReport succeeded. This strengthens H2 (layered access) while changing the boundary between its tiers.

### Exact-host sensor/fan conclusions

- Physical and SMC evidence agree on one fan.
- A missing ordinary registry property did not mean absent hardware or unavailable low-level access.
- The current TUI’s empty fan list is an implementation gap, not a host limitation.
- Two temperature channels remained around -1.3 C and require validity state rather than threshold coloring.
- Pressure remained nominal under the observed loaded temperatures; temperature, load, fan speed, and pressure are separate observations.

### Privacy result reinforced

IOReport’s full dictionary was approximately 4.68 MB before serialization filtering. The correct production pattern is narrow group/channel subscription, not “collect everything and redact later.” Private source discovery and data extraction require allowlists at the source boundary.

---

## Evidence Batch 3 — adversarial implementation and payload audit

### Late observations that changed or sharpened the report

| Finding | Confidence | Evidence / implication |
|---|---|---|
| On macOS 26.3.1, unprivileged `diskutil info -plist disk0` returned `SMARTDeviceSpecificKeysMayVaryNotGuaranteed` with NVMe spare, threshold, percent-used, composite temperature, data/command counters, media/error-log counts, power-on hours/cycles, unsafe shutdowns, and controller-busy fields. | High on this host; low as an Apple compatibility promise | Direct typed plist inspection. This falsified the earlier local conclusion that detailed Apple-internal NVMe wear/temperature was unavailable, while the dictionary name itself keeps H2/H3 intact. |
| NVMe-standard semantics explain 512,000-byte data units and percent life consumed, but Apple does not document that its `_0/_1` keys are low/high limbs. | Medium for the mapping, high for the need to preserve raw values | NVM Express primary specification plus local field widths/values. The report now makes limb joining a versioned experimental derivation rather than a fact. |
| Public Dispatch memory-pressure events are the production route; free percentage and undocumented `kern.memorystatus_vm_pressure_level` are only cross-checks. | High | Apple Dispatch contract plus local command observations. This prevents a generic memory percentage from being relabeled as Apple's pressure state. |
| `sysinfo::Process::tasks()` is unavailable on macOS; `proc_listallpids` plus `PROC_PIDTASKINFO` yields a permission-limited thread lower bound. One live sample had roughly half of PIDs readable and a known lower bound above 3,000 threads. | High on this host | SDK ABI inspection and direct safe calls. The product must report count plus coverage, not `0` or a false exact total. |
| `proc_listallpids` returns an element count even though its buffer-size argument is bytes. | High | SDK/local ABI test. The implementation handoff explicitly prevents a divide-by-`sizeof(pid_t)` bug. |
| Exact IOReport group requests still return many channels; filtering the `IOReportChannels` array before subscription is necessary. | High on this host | CF object inspection. The report now pins exact group/subgroup/name/unit/state allowlists and forbids fallback broadening. |
| Real command fixtures can be useful without exposing the owner if the request envelope, schema, and allowlisted DTO are persisted instead of raw output. | High | Sanitized live `system_profiler`, `diskutil`, and I/O Registry captures. Bluetooth/Wi-Fi device names, serials, UUIDs, network identities, and lifecycle fingerprints remain excluded/coarsened. |

### Adversarial consistency corrections

- Removed a nonexistent public Disk Arbitration “volume role” field; APFS roles come from the structured `diskutil apfs list -plist` `Roles` array.
- Made `ProtocolMismatch` a real `Availability` variant rather than prose referring to a type that could not compile.
- Made monotonic timestamps internal/non-serialized and retained last-good monotonic time only for same-process age calculations.
- Replaced serializable free-form error/path strings with bounded reason/tool enums and validated source-contract IDs.
- Corrected Dispatch memory-pressure pointer construction for the immutable extern static and prohibited a source/handler retain cycle.
- Corrected Mach OOL deallocation to convert returned integer count to bytes.
- Kept public/App-Store and enhanced/private build profiles distinct, including `sysinfo` feature behavior that could otherwise hard-link a private IOHID symbol.
- Recorded the current artifact's macOS 11 deployment floor and required runtime selector/symbol availability checks for every newer public API.
- Added real sanitized request/response examples and explicit serde field names so the Windows implementation does not need to guess Apple payload casing or nesting.

### Revised storage inference

The earlier statement “detailed internal NVMe wear/temperature was not obtained”
was true at the time of the first probe but is no longer the final evidence. The
best-supported conclusion is narrower:

> This macOS 26 M2 host exposes unusually detailed internal-NVMe health through
> an unprivileged, undocumented, explicitly non-guaranteed `diskutil` plist
> dictionary. It is immediately useful as an experimental source, not a stable
> cross-Mac contract.

This revision is important scientific hygiene: later contrary evidence updates
the conclusion instead of being forced into the earlier narrative.

---

## Step 5: Analyze — final hypothesis comparison

| Hypothesis | Final disposition | Reasoning |
|---|---|---|
| **H1 — public/unprivileged sufficiency** | Partially supported only after splitting “unprivileged” from “public” | Public APIs cover a strong baseline, but the richest locally proven sensors/fans/energy require unpublished interfaces; other desired fields remain root/private/unavailable. |
| **H2 — layered access is necessary** | Strongly supported | Public, Apple-command, unprivileged-private, root, TCC/entitlement, third-party, and different-Mac-only tiers all have distinct examples. |
| **H3 — convenience-sample danger** | Strongly supported | The host supplies exact M2 laptop keys/tables/labels but cannot settle fanless/multi-fan, Pro/Max/Ultra, Intel, desktop, sandboxed, or future-OS behavior. |
| **H4 — optional GUI can remain decoupled** | Supported as an architecture direction | Vercel Labs Native can consume a long-lived Rust sidecar/FFI boundary; its pre-1.0 and uneven platform status argues against moving the canonical core into it. |

### Inference to the best explanation

The evidence is explained with the fewest contradictions by a provenance-first layered backend. A single cross-platform abstraction loses Apple topology and semantics; a shell-only backend is too slow/private; a private-only backend is brittle; a root-only backend harms usability. Selecting per-metric sources and retaining access/confidence/failure state explains both the successful deep probes and the persistent portability limits.

### Logical fallacy audit

- **Hasty generalization avoided:** exact M2 SMC/IOReport success is tagged exact-host/private, not “all Macs.”
- **Argument from ignorance corrected:** the first fan search failure was not treated as proof of no fan/RPM route; the deeper test found one.
- **Equivocation avoided:** “native” for Vercel Native is separated into native executable/window/custom renderer vs OS-native widgets.
- **Affirming the consequent avoided:** a Metal capability flag or registry engine presence is not used to infer dedicated hardware units or live utilization.
- **False equivalence avoided:** raw temperature, thermal pressure, load, throttling, and hardware condition are separate.
- **Base-rate neglect avoided:** normal virtual interfaces, high loaded utilization, and unsupported device fields are not automatically anomalous.

### Steel-manned dissent — final

The strongest case against implementing the private tier is distribution and maintenance risk: unpublished ABI use can change, fail App Store review, break under hardened/sandboxed contexts, or produce wrong-but-plausible values. A robust product could stop at public APIs and root `powermetrics` opt-in.

The response is not that private APIs are harmless. It is to keep them optional, dynamically feature-detected, precisely versioned, source-labeled, validity-checked, and replaceable—while retaining a complete public baseline. If those safeguards or fleet tests are unaffordable, ship the public tier and mark deep metrics unsupported.

---

## Step 6: Report

**Artifact:** `docs/research/2026-07-17-macos-hardware-monitor-capability-report.md`

The report contains:

- sanitized exact-host evidence and a full 33-channel current-dependency temperature series;
- the direct 38-service IOHID, SMC fan, IOReport, DVFS, Metal/AGX, battery, storage, network, display, and device findings;
- the late macOS 26 private `diskutil` NVMe SMART dictionary, typed normalization rules, and explicit cross-version caveat;
- current-repository correctness gaps and extension points;
- privacy/access/cadence/different-Mac matrices;
- a concrete observation/capability/health architecture;
- a Rust implementation blueprint with modules, dependencies, FFI ownership, function signatures, SMC layout/commands, sampling algorithms, parser/fixture contracts, and CI/hardware gates;
- an optional Vercel Labs Native GUI assessment that keeps CLI/TUI canonical;
- sanitized real request/response envelopes and payload examples that remove stable identifiers and owner/network data;
- reproducible safe probes, primary sources, open questions, and later Mac validation.

The operator clarified during reporting that the Windows agent must be able to implement the backend without guessing. The report was expanded from capability inventory to implementation-ready specification accordingly; no production implementation was performed.

### Sanity check

- Host-specific values are labeled and sanitized.
- Raw profiler/registry/IOReport payloads are not committed.
- Fan and IOReport claims reflect the successful direct probes, superseding the initial broad-scan limitation.
- The initial NVMe-detail limitation is explicitly superseded by the later typed `diskutil` discovery rather than left as a contradiction.
- Public, private-unprivileged, root, entitlement, and unavailable routes remain distinct.
- Exact frequency/temperature/RPM values are snapshots or model tables, not health thresholds.
- GUI research is subordinate to the Rust core/TUI/CLI direction.
- Deferred invasive/permission/other-Mac tests are explicit.

### Spaced revisit prompt

Before the implementation phase is considered complete, revisit after the first Windows-built backend compiles and answer:

1. Which report assumptions became compile errors or ambiguous bindings?
2. Which fixtures were insufficient to reproduce actual Apple payloads?
3. Did the first real-Mac run preserve every unavailable/error state, or did defaults reappear?
4. Do private sources still work from the signed/notarized distribution artifact?
5. Which private metrics should be removed rather than maintained?

### Exit state

**Directed after adversarial audit.** The inquiry has enough exact-host evidence, sanitized payload structure, and implementation detail for the Alienware agent to begin the architecture and fixture-driven work. Release confidence still requires the named Mac fleet, privilege, TCC, sleep/wake, private-ABI, and signed-artifact gates.
