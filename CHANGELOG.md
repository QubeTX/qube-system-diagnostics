# Changelog

All notable changes to SD-300 will be documented in this file.

## [4.0.2] - 2026-09-24

- Verify production release abf55bd: all native producers and exact-merge CI pass under ADR 0021; final publication and public lifecycle run 36072546804 passes. Confirm 59 public assets, crates.io/latest identity, and the complete EULA in Global, Corporate and compatibility MSI downloads with matching ProductVersion, SHA-256 sidecars and attestations. Both latest MSI routes resolve to the verified bytes.

- Record the operator-authorized EULA-only performance exception (ADR 0021). Keep numeric thresholds and failed timing reports unchanged; separately accept complete functional interaction/shutdown evidence for exactly 4.0.2. Retain the exact-candidate CI publication barrier and all installer/signing/integrity checks.

- Replace the WiX placeholder EULA in Global and Corporate MSI installers with the complete PolyForm Noncommercial 1.0.0 license generated from LICENSE.md. Check source parity and the embedded LicenseAgreementDlg text during Windows packaging; preserve the acceptance checkbox and navigation.

## [4.0.1] - 2026-09-23

- Close v4 and corrective-update delivery records after the explicit refresh acceptance, deployed installation-method guidance (website #19), and validated CI request correction (#13). The complete hosted Windows core job returns real public v4.0.1 metadata with the scoped test token; the installed public CLI separately passes an anonymous already-current check. Preserve the historical 403's unknown cause and distinguish this focused verification from the routine ongoing GUI rerun.

- Post-release qualification: record the operator's 2026-09-24 acceptance of the measured Intel Mac 116.583 ms refresh for 4.0.1 only (ADR 0020), preserving the failed 100 ms verdict and original future targets. Separately authenticate the opt-in Windows CI release check with the step-scoped read-only Actions token and report allowlisted numeric quota headers on HTTP failure. CI 36056348704 returned HTTP 403 through both shells without enough evidence to establish its cause; all six GUI jobs passed. Customer update behavior and immutable published artifacts are unchanged.

- Verify immutable public 4.0.1 across 59 assets, 28 checksums, 26 attestations, six stable routes and crates.io. Install the public wrapper on the identified Windows machine; 12 payload files match, settings are byte-identical, GUI self-test and Start/PATH discovery pass, and actual JSON/text update checks succeed. Verify deployed website guidance and screenshot interaction on desktop/mobile. Retain the separate failed merged-CI Intel refresh result; public lifecycle success does not waive performance acceptance.

- Post-release operations: add an exact-candidate CI barrier before crates.io or GitHub publication. The independent merged-commit CI reported an Intel refresh maximum of 116.583 ms twenty seconds before the installer-driven chain published 4.0.1. Retain this failed second matrix alongside the passing pre-merge matrix; do not waive the limit or alter immutable artifacts. Nine gate fixtures and a live read-only rejection of the failed source run verify newest-run precedence and bounded waiting (ADR 0019).

- Retain final six-target observer-ordering qualification for 08b7fe5: all 171 inputs per target, clean shutdown, unchanged 100 ms input/frame p95 and ordinary-refresh maximum pass. Preserve Intel's individual 121.092 ms navigation response and original next-version targets. Windows composite lifecycle 36046014017 passes; PR #11 merges with source-tree identity intact. Public-release and installed-byte verification remain separate required steps.

- Defer filesystem-backed automation publication until a pending input's requested GPU frame completes, then enqueue one publication turn. Native profiling attributes large Linux and Intel Mac input delays to snapshot preparation/I/O on the UI thread. Preserve full latency measurements, frame numbering/lifecycle callbacks, command acknowledgment, no-repaint liveness and the non-automation product path. Walk large retained views by reference in the two read-only frame checks, avoiding Debug stack overflow. Retain all six attribution reports; do not label observer interference as an ordinary-release renderer defect.

- Add profiling-only monotonic input-dispatch/frame-wait and automation-publication attribution with bounded latest-sample metadata. Preserve end-to-end latency, original/approved limits and failed results. The operator selected latency correction before publication; this diagnostic cycle does not claim a fix.

- Retain final native shell discovery and six-target timing evidence for efbab9e. Linux passes 24 shell cases; macOS passes 23 with fish skipped. Preserve Intel/musl input-gate failures and bounded delayed-input trace data, while separating successful Windows lifecycle and all-target functional/shutdown checks. Hold publication and local installation pending the explicit gate-owner decision or an attributed GUI correction; do not restart unchanged qualification solely to save evidence.

- Preserve pre-existing symlinked shell-configuration directories by recording their link identity. Reject new or retargeted links before completing profile integration, and skip rollback below changed links instead of touching a replacement target. Add native symlink fixtures for config/fish/conf.d targets and sibling preservation.

- Record the expanded discovery candidate's successful Windows composite matrix and native Linux Bash/fish tests, preserving native timing reports and the Apple Silicon refresh overrun separately from the corrected PowerShell test-wrapper status.

- Report successful Windows discovery fixtures explicitly after expected child failures and cleanup, so GitHub's PowerShell wrapper cannot mistake an intentionally exercised child exit for the test suite's result. Preserve terminating assertion and cleanup failures.

- Repair managed shell command discovery: align custom-prefix resolution with the generated child, prevent a temporary process PATH entry from suppressing persistent setup, create startup integration for a missing Bash rcfile and relocated fish configuration, preserve PATH opt-outs and existing profiles, and track exact additions for rollback without overwriting concurrent edits. Require native Linux fish startup fixtures and exercise a fresh Bash session in the actual Linux composite install lifecycle. Installation completion on every platform now identifies the desktop app as well as the CLI.

- Verify managed Windows CLI PATH and GUI discovery before reporting complete installation. Resolve redirected Programs folders for shortcut creation and uninstall, create missing shortcut parents, verify target/working directory/custom icon, and refresh only the installer process PATH. Honor PATH opt-outs explicitly, align custom-prefix backup and cargo-dist destinations, fix rollback attribution when the bin directory existed only in process PATH, retain the prior Installed Apps icon during rollback, and identify the failed install stage. Add safe real-COM and child-PowerShell regression fixtures on PowerShell 5.1 and 7 and installer-matrix shortcut assertions.

- Retain corrective-release qualification: the complete Windows installer matrix and five native interaction targets pass. Preserve both GNU x86-64 input-timing failures and all eight first/repeat reports, without claiming publication or a waived gate. Document the public-install verification still required.

- Fix Windows automatic release checks end to end: emit UTF-8, fail explicitly on HTTP errors, reject empty/malformed/unusable responses before transport selection, and retain bounded fallback diagnostics. Add a real public-API Windows CI check that cannot use the candidate-release override.
- Replace vague lifecycle failure headings with the failed operation and recovery downloads while preserving lifecycle JSON schema and exit contracts. Release-check errors state that installation files are unchanged and suggest concrete next actions.
- Validate a newly installed companion through its stable self-test envelope and its own engine contract, avoiding a repeat of the older-updater ABI rejection on future major upgrades. Retain exact same-version ABI checks and reject failed/wrong-product/wrong-version/unknown-envelope results.
- Record the operator-authorized corrective release and extend the existing v4 performance ceilings to 4.0.1 only (ADR 0018). Original targets resume for 4.0.2 and later; preserve all prior measurements and their artifact identities.

- Record the immutable 4.0.0 public release, six native lifecycle workflows, exact-asset verification, deployed desktop/TUI screenshots and the actual managed Windows installation. Document the real 3.1.3 ABI-1 rejection and current Windows update-check workaround without claiming those automatic paths passed.

- Launch Windows PowerShell and PowerShell 7 helpers with a hidden console while retaining suspended job assignment, deadlines, output limits and owned-descendant cleanup. Public 4.0.0 verification reproduced successful empty exits under DETACHED_PROCESS, including failed automatic update checks. Native monitoring workers remain detached. Add real-host file/pipe capture and exit-status regression coverage; correction is not in immutable 4.0.0 assets.

## [4.0.0] - 2026-09-23

- Record the operator-approved 4.0.0-only resource ceilings (4% foreground CPU, 3% hidden CPU, 200 MiB RSS); retain the 300 MiB private-memory limit, original failed verdicts, and all functional gates. Add version-expiring resource assessment and boundary/failure tests. Preserve every original goal for the next version in docs/next-version-targets.md and owned tasks #r16/#r17 (ADR 0017).

- Retain both complete native interaction matrices and all original verdicts. The unchanged-product ce77ebf repeat passes the approved timing policy on all six targets; both Mac native accessibility fixtures and exact keyboard-completion counts pass. Retain the final native resource windows and passing Windows two-hour soak, foreground and hidden windows with exact identities; remaining CPU/RSS overruns remain explicit.

- Record the operator's 2026-09-23 all-platform 4.0.0 responsiveness decision: frame/input p95 and ordinary-refresh maximum may reach 100 ms. Retain original-target verdicts, expire the exception for later versions, and keep CPU/memory, native accessibility, functional, shutdown and lifecycle checks mandatory (ADR 0016; follow-up #r16).

- Reconcile product and contributor typography documentation with the verified Makira heading, Gail Rock body/control, and Plex Mono technical font mapping.

- Guard legacy AppKit accessibility superclass dispatch when NSAccessibilityElement does not implement the requested selector. Native stack attribution found exception unwinding during ordinary semantic publication. Preserve supported text/selection actions and extend the real-host fixture to reject unsupported queries without exceptions.

- Retain the complete native before/after resource matrix with exact artifact identities, measured deltas and all failed limits. Run further GTK graphics-allocation and Mac hidden-frame diagnostics on a separate branch using unchanged application artifacts, without treating observer-attached diagnostics as acceptance.

- Separate macOS accessibility snapshot publication from assistive-client focus actions. Previously each focused publication called the action setter, clearing keyboard-visible focus and producing duplicate input/redraw events. Add an actual AppKit-host regression fixture on both native Mac runners; preserve external accessibility focus requests.

- Retry transient partial automation snapshot reads within the existing bounded deadline. Preserve fatal identity, dispatch-error and output-limit checks after Intel Mac qualification exposed the SDK's non-atomic snapshot replacement; add regression fixtures without changing product binaries or performance thresholds.

- Retain bounded per-input frame/timestamp counters and focused control IDs in native interaction qualification reports. Exclude widget labels and arbitrary role strings, with privacy fixtures, so native input delays and duplicate completion counts can be diagnosed without exporting process or device names.

- Quote staged and installed binary self-test paths in the managed shell installer. Exercise Linux composite lifecycle with a spaced application-data directory and retain a direct installed-version verification fixture.

- Handle GIO's executable-existence check before desktop field-code expansion for literal percent paths by invoking the system env executable without a shell. Retain per-character native launcher cases and diagnostic fixture output.

- Resolve the Linux application-menu icon through an absolute path inside the owned bundle, and quote desktop launch paths using the freedesktop escaping rules. Validate the real GIO parser and launch with literal special-character paths, alongside installed-entry assertions. Windows Start-menu shortcuts and macOS application bundles retain their existing custom icon delivery.

- Record the bounded GUI preference writer and complete interaction timing contract in ADR 0015, including measured evidence and the remaining native qualification boundary.

- Coordinate the unpublished 4.0.0 candidate across CLI, engine, native manifests, packaging and all staged build templates. Extend version reconciliation to the test-stage template. Add native interaction qualification for all six targets using independent automation builds, bounded snapshots, publisher identity, complete percentile coverage and owned cleanup. Correct timing cycles that finish without repainting so idle updates cannot accumulate into an invented long frame.

- Move GUI preference commits off the input/render thread into one joined writer with one replaceable pending document. Preserve atomic namespace-aware writes, report pending/success/failure honestly, and flush the newest request before engine unload, including AppKit termination. Tests cover coalescing, failure, requests during slow I/O and recovery. Local timing attributed 5.5 ms median and 11.3 ms p95 navigation cost to synchronous preference commits.

- Stop macOS native-only event delivery from arming the JavaScript bridge's ten-second, 60 Hz frame keepalive when no WebView exists. Hosted thread counters and stacks attribute the hidden-window cost to this main-thread timer churn; native functional and long-window resource verification remain required.

- Add bounded full-event frame-work and input-to-present qualification counters, including p95, lifetime stall maxima and sample-window coverage. Exclude automation snapshot I/O and queue wait from synchronous work. Keep automation builds in a separate staging directory and verify every patched SDK file in both build preparers; deterministic fixtures cover nesting, idle gaps and overwritten percentile windows.

- Correct Mac diagnostic thread attribution after hosted task-port access denial: use public libproc thread-ID reads with documented nanosecond units, bounded identities extracted from the native stack sample, and an independent native current-thread clock fixture. Report partial coverage and separate the following CPU window from the stack sample. Give the oversized-output regression fixture its own deadline so pipe throughput cannot conflate byte-limit and timeout checks; production limits are unchanged.

- Attribute native macOS diagnostic CPU to bounded live-thread deltas alongside stack samples. Exclude new, ended and reset threads, preserve denied counters, and keep profiler results outside resource acceptance; waiting-stack frequency is not CPU time.

- Preserve completed native GUI resource observations when shutdown fails, and continue candidate measurement after an explicitly reported failure in the immutable public baseline. Keep baseline failures intact; timeouts, malformed or missing reports, and candidate failures still fail qualification. Add deterministic reporting and continuation fixtures after the public musl baseline crashes on close.

- Pair embedded Makira headings/readings with Gail Rock body/navigation/controls. Add reviewed private font provisioning to every native build lane and a heading-font token that keeps intrinsic measurement, paragraph wrapping, selection and rendering on the same face. Retain IBM Plex Mono for compact technical values; verify font coverage and layout/paint agreement. Bound translated glyph-edge raster comparison to one RGB code value with exact alpha; retain exact panel/shadow comparisons.

- Stop and join the GUI engine from AppKit's synchronous termination notification, because `terminate:` does not unwind `main`. Preserve ordinary library-unload ownership and idempotent shutdown; add a native notification fixture and retain the complete-bundle remaining-worker gate on both Mac architectures.

- Derive the GUI connectivity panel's freshness from its own diagnostic topic instead of an unconditional Current badge. Use readable reachability states, label DNS resolution duration separately from RTT, wrap long graphics provenance, and improve spacing in compact Settings.

- Retain bounded process-role, state and parent evidence when native GUI shutdown leaves a helper, without logging executable paths or arguments. Run macOS CPU attribution independently after failed shutdown qualification so both failures remain diagnosable; profiling never substitutes for resource acceptance.

- Review the native GUI across all nine sections and Settings. Prioritize live readings/history, add direct audience switching, wrap explanations and consent, separate optional network scans from bandwidth tests, and place SMART setup with storage. Preserve current process rank on every captured page, avoid startup-zero interpretations, remove synthetic pending detail from available observations, and expose unified/shared GPU memory categories. Record source and live visual evidence separately from accessibility and cross-platform acceptance.

- Reconcile project and contributor documentation with the implemented v4 monitor lanes, isolated workers, adaptive dashboard and independent settings namespaces. Remove obsolete v3 scheduler/layout freezes and distinguish implemented behavior from still-pending native performance/lifecycle qualification.

- Select Cairo before GTK initialization for the Linux CPU-rendered GUI, preserving explicit renderer overrides. Route external quit through close-request so SDK widget cleanup precedes destruction; add native backend, override and lifecycle fixtures. Keep GNU GL smoke coverage and musl repeated shutdown diagnostics. Add separately bounded Apple `sample` runs for native TUI/hidden-GUI CPU attribution; debugger/profile reports cannot qualify performance gates.

- Add an opt-in bounded native backtrace after Alpine GUI shutdown fails with SIGSEGV. Repeat unchanged artifacts only for diagnosis, attach after the sampling window, retain the original failure, and prohibit treating debugger runs as resource qualification. After an unchanged rerun passes, exercise three foreground/hidden shutdown pairs with traces to investigate the intermittent fault.

- Draw opaque solid panels and borders directly through the reviewed SDK patch instead of retaining duplicate panel pixels. Correct fractional shadow-occlusion boundaries that blended pixels twice. Preserve rounded-edge coverage and translucent/gradient memoization; add byte-exact scale, clipping, translation, opacity and border fixtures. Match the warmed benchmark to the actual static-prefix runtime path, retain the generic diagnostic and exercise bounded bidirectional scrolling. Coordinate both preparers and patch/file hashes while retaining official upstream archive pins.

- Preserve in-progress manually dispatched qualification windows, record first/last Windows GUI memory by role, and report retained raster-cache bytes outside render timings. Correct the render fixture's process field availability so process ticks exercise visible numeric changes.

- Restrict GUI collector-presence inspection to collector-shaped processes and retain required-worker verification when protected helpers deny executable lookup. Native GNU long runs exposed a harness AccessDenied on a ping helper after the TUI resource gate passed.

- Attribute Linux GUI RSS to fixed mapping categories after the measurement window and compare the unchanged binary with GTK's diagnostic Cairo renderer. Keep default-renderer acceptance independent and redact mapping paths/addresses from evidence.

- Replace macOS process CPU percentages with checked libproc counters, Mach timebase conversion and per-instance monotonic deltas. Preserve measured idle zero, microsecond creation identity, inaccessible fields and full inventory before ranking. Expose inventory failure/recovery in both frontends and capabilities; use the reserved ABI-2 process-summary word without changing layout. Add native getrusage comparison, identity/reset/unit fixtures and GUI status tests.

- Install Alpine's separate `xvfb-run` package and check native GUI qualification tools before compilation. The musl lane previously built successfully but could not launch its virtual display fixture.

- Retain slow-worker disk/sensor discovery containers and omit unused sysinfo disk I/O collection, following native Mac stage profiling. Refresh device lists and capacities at the existing cadence; retry/resume still resets discovery state. Physical disk activity remains an independent one-second provider.

- Treat future or absent capture timestamps as unavailable age instead of measured zero, restart histories across clock rollback and accept the next genuine sample. Add explicit freshness to schema-2 reports and nullable age to engine schema 2; coordinate GUI parsing and installer checks, and fix the CLI companion verifier's obsolete ABI expectation.

- Apply explicit hidden startup intent before macOS's asynchronous window hide, and connect Linux surface notifications to collection profiles with prompt restore sampling. Extend native hide/restore fixtures, expose X11 setup failures and collect Unix per-stage CPU timing and bounded role attribution to investigate native resource failures.

- Serialize short-lived static, driver and health worker processes within each monitor session after peak-role measurements confirm overlap. Retain independent live sampling, actual capture timestamps and existing cadences; test exclusive admission, cancellation and panic recovery.

- Attribute Windows GUI peak working set to bounded process roles after the first full foreground run passes CPU/private memory but exceeds the RSS gate. Retain the failed result and repeat unchanged artifacts before choosing a product optimization.

- Extend opt-in native before/after resource qualification to both GUI visibility profiles. Verify immutable public GUI baseline hashes, archive bounds and source equivalence; distinguish the legacy in-process collector topology and retain complete-bundle candidate gates.

- Scope Git checkout trust to the mounted source for Alpine qualification commands, preserving global configuration while allowing exact candidate/baseline identity checks across container UID boundaries.

- Preserve explicit unavailable descriptor coverage in Unix performance reports when a protected helper denies fd enumeration. Continue independent CPU/RSS accounting, retain nullable maxima and add denied-versus-zero fixtures.

- Detect Linux GUI mapping/minimized state through GTK/GDK so background windows use the existing reduced collection profile. Add native mapping/recovery fixtures and complete-bundle Mac/Linux foreground/hidden resource smoke checks with bounded process accounting and normal shutdown.

- Add opt-in native before/after release TUI resource measurements to all six CI targets, using immutable baseline bytes, sequential measurements and retained failure reports. Validate Unix completed-child CPU accounting separately and keep visual interaction observers outside the resource window.

- Reconcile the GUI engine's separately resolved Rust dependencies with the qualified CLI lockfile, including sysinfo and serialization. Check shared versions, registry sources and checksums alongside product-version validation so frontend builds cannot silently drift.

- Read Linux and macOS interface counters with explicit per-interface failures, native identities, scoped addresses and 64-bit byte semantics. Preserve other readable rows, reset failed baselines and exclude loopback from the documented aggregate. Keep unavailable cumulative bytes nullable in schema 2 and inspectors, add native local-payload fixtures, and expose successful GPU utilization provenance in GUI Technician mode.

- Prefer a persistent native PDH query for Windows GPU engine utilization, retaining fractional values, adapter identity, measured intervals and explicit warmup after reset/resume. Bound native array parsing and retry, preserve incomplete fields and the existing WMI/NVIDIA fallbacks, and test real worker warmup/reset against available adapters.

- Add whole-process Windows GUI measurement with suspended-before-job ownership, isolated settings, bundle hashes, required worker checks, visibility validation and clean-shutdown assertions. Extend stage profiling with slow providers and quantized process CPU, retaining the still-failing long TUI resource result.

- Qualify noisy worker termination by either the output limit or its earlier absolute deadline, with a fixed cleanup allowance and owner-reaping assertion. Native Apple Silicon evidence identified deadline-first behavior; production limits and collection behavior are unchanged.

- Use native Windows ICMP reply status and millisecond RTT inside the bounded diagnostic worker, with below-resolution RTT left unavailable. Resolve the OS-selected IPv4 route instead of the first printed default route. Remove periodic ping/route subprocesses and show provider provenance in both frontends; retain ICMP-independent TCP reachability fallback.

- Replace invented macOS input devices with structured IOHIDDevice inventory; stop treating device presence or Linux link-down as driver-health evidence. Preserve per-provider discovery failures, service-manager scope and actual runtime states across Windows, macOS and Linux. Project bounded observations and service details into both frontends, shared findings and nullable schema-2 exports while retaining schema-1 keys.

- Replace remaining optional-result command adapters with explicit execution errors, preserving nonzero exit status separately from timeout, missing executable, denied access and invalid text. Propagate failure reasons through gateway/ping and install verification paths; retain bounded ownership and output handling.

- Extend the verified macOS terminal polling path to Linux after the same resize/input test exposes lost readiness on both GNU architectures. Retain the unchanged Windows backend and record the long-window native-network resource result, which still exceeds the CPU gate.

- Use crossterm's file-descriptor polling backend on macOS to retain pending input when resize and keyboard readiness arrive together. Extend real PTY qualification with immediate resize/key pairs without delays or retries.

- Read Windows interface octets and addresses through bounded native tables using full GUID/LUID identities. Remove repeated address enumeration and partial-GUID grouping; preserve native failures and recovery warmup. Define hardware-only aggregate scope while retaining virtual/tunnel rows, and expose per-interface rate/address availability in both frontends and schema-2 exports.

- Report the native pipe fixture's actual failure category and elapsed time before asserting its expected outcome, so hosted failures distinguish output limits from deadlines and I/O errors.

- Attach the successful capture time, age and latest provider state to retained storage-fault and incomplete-observation findings. Keep hardware fault evidence after refresh failure, explicitly mark stale or clock-discontinuous ages, and replace obsolete GUI ABI copy with useful findings guidance.

- Retain bounded structural diagnostics on native terminal qualification failures: completed steps, fixed-label positions, cursor and process state, without arbitrary screen contents. Distinguish platform interaction failures from emulator assumptions before selecting a product fix.

- Deliver isolated collector responses through bounded in-memory frames instead of per-sample temporary files. Validate lengths before allocation, bound pipe draining and stderr, and reap owned processes immediately after malformed, partial, cancelled or timed-out responses. Add native fragmented/large/inherited-pipe fixtures.

- Add native PTY qualification across all six targets for both modes and terminal sizes, filtering, Unicode/ASCII, mouse and keyboard navigation, frozen views, consent dismissal and terminal restoration. Capture input latency separately from terminal setup, add release-stage profiling, and retain failed long-window resource measurements.

- Calculate Windows per-process CPU from process-time deltas over monotonic capture intervals, independent of monitor affinity and processor-group size. Reset unreadable/reused/rolled-back baselines and use the same aggregate CPU provider on GUI Processes as other pages. Add a native restricted-affinity comparison against GetProcessTimes.

- Start redirected Windows collector helpers detached from consoles, retaining suspended-before-job ownership and bounded cancellation. Verify both file and memory output capture without console allocation; reduce transient processes and working sets without changing cadence.

- Prefer a reusable read-only NVML session for NVIDIA telemetry on Windows and Linux, with bounded discovery backoff and the existing nvidia-smi fallback. Require versioned allocated-memory semantics, preserve individual permission/unsupported errors and PCI identity, and expose memory/temperature provenance in both frontends.

- Replace periodic Windows netstat processes with bounded native IPv4/IPv6 TCP/UDP owner-PID tables. Preserve per-provider failures and partial endpoint inventories through reports, findings and both frontends. Correct Linux ss abbreviated states, add a PID-unavailable procfs fallback on minimal hosts, and include macOS UDP endpoints. Native fixtures compare owned loopback sockets with the actual worker output.

- Compute disk activity deltas inside the isolated worker using its monotonic capture clock. Carry capture intervals across IPC, reset baselines on worker replacement/retry/resume, and record fast samples from completed captures rather than scheduled starts.

- Retire isolated static, driver and health workers after each infrequent probe while retaining results and existing refresh/retry cadence in the parent session.

- Resume suspended Windows helpers through a process-specific PSS thread snapshot, retaining owned-job cancellation and a Toolhelp fallback. Add a real ConPTY process-tree benchmark that includes terminated-child CPU costs.

- Search and page full connection/device inventories in the GUI engine before bounded projection. Preserve capture metadata, whole-inventory totals and attention filtering; show missing connection PIDs explicitly. Add guarded query ABI loading and fixtures beyond the former row limits.

- Add explicit TUI session exports through the shared schema-2 redaction path, with one background writer per frontend, private report files and atomic no-clobber persistence. Preserve frozen samples and completed companion results; move GUI file writes off its collection loop.
- Track storage read/write error availability and whole-inventory coverage independently in the GUI. Mark early or unqualified SpeedQX measurements partial and ignore inherited TAR_OPTIONS during verified companion extraction.

- Normalize accepted storage callback sockets to blocking mode before applying finite read/write timeouts on Windows and macOS. Native qualification caught inherited nonblocking mode that Linux does not preserve.

- Add separately prepared and confirmed, single-device SMART reads in Storage in both frontends. Isolate OS authorization, verify helper bytes and product version, bound read/callback/cancellation work, preserve prior results, and include redacted schema-2 findings. Add native synthetic privileged-worker qualification across all six targets without touching physical devices.

- Correlate Windows disk health by unique serial and PnP identity, and retrieve reliability counters through the documented physical-disk association. Reject ambiguous joins and SMART device replacements; preserve conflicting fault evidence instead of letting later healthy readings erase it.

- Fetch signed Alpine package indexes without requiring or modifying a system cache during confirmed SMART setup.

- Capture bounded native Alpine package-manager diagnostics when optional SMART qualification fails, without printing downloaded archive bytes.

- Add separately confirmed SMART helper setup through checksum-pinned Windows component extraction, existing macOS Homebrew, and authenticated Debian/Ubuntu or Alpine package extraction. Preserve independent ownership, verify the JSON interface, avoid package service scripts, and retry ordinary health collection after successful setup. Native qualification exercises installation without device probes.

- Add separately confirmed ND-300 archive setup in both frontends and the `tools nd300` command. Pin official hashes for all six targets, preserve existing owners, verify installed executables, reserve destinations without replacement, and retain ND-300 after SD-300 removal.
- Discover optional tools in standard platform locations and deliberately shared provider paths; preserve those choices across GUI settings writes with bounded cross-process locking.

- Recover from companion worker panics or thread creation failure, verify one-active-request cancellation, and project unavailable speed readings as missing data in both interfaces. Extend snapshot tests through the companion privacy boundary.

- Add explicit ND-300 4.0.1 and separately confirmed SpeedQX actions to both frontends. Bound in-memory process output and cancellation; retain diagnostic exit outcomes, partial checks, nullable throughput and provenance. Export only validated fields in redacted companion reports.
- Replace shell-dependent resource-test producers with native fixtures, serialize timing-sensitive subprocess tests, and use populated deterministic redaction fixtures instead of concurrent live inventories.

- Search and page the complete GUI process inventory before selecting bounded rows. Query changes reuse the actual capture timestamp; ABI-2 page counts and offsets match Rust/Zig layout assertions.

### Fixed

- Read macOS storage counters through bounded native IOKit property snapshots, release owned references, require registry identity, and filter virtual/backing layers using cached physical-disk inventory.

- Reuse one owned subprocess per isolated collector lane with bounded atomic responses and cancellation. Cache Windows graphics topology by device identity and back off negative thermal/NVIDIA discovery without reusing stale numeric readings.
- Invalidate discovery after interface/disk topology changes and resume; reset rate baselines after long fast-sample gaps.

- Replace section-specific render-time sorting with an adaptive prepared dashboard, complete-inventory filters, identity-preserving selection, contextual inspection, paging, pause-view and opt-in mouse input. Add independent TUI preferences and terminal fallbacks.
- Render captured time buckets with explicit gaps in both frontends, keep CPU/GPU thermal histories separate, correct IEC byte labels, and subscribe the GUI storage view to live disk activity.

- Preserve stable thermal/fan channel identities in both frontends, keep identical labels separate, and collect every GPU temperature. Read Linux hwmon units and fault/enable flags explicitly; do not classify generic package temperatures as CPU readings.

- Identify GPUs through DXGI LUID/PCI locations, Linux DRM/PCI devices and Metal registry IDs. Join NVIDIA telemetry by PCI identity, preserve per-field availability, and keep shared/unified memory and allocation budgets distinct from dedicated VRAM.
- Aggregate Windows GPU counters per physical engine across processes, then select the busiest engine. Preserve existing driver/display telemetry through PnP location matching; a temperature-only adapter no longer displays fabricated zero utilization.

- Add Linux power-supply battery, DRM display, hardware-backed network link and DMI/device-tree identity providers with documented units and explicit missing fields. Add native macOS CoreGraphics displays, IOKit power-source snapshots and hardware identity.

- Add opt-in schema-2 JSON exports with nullable measurements, sample metadata and shared findings while freezing schema-1 keys. GUI exports use the richer report.
- Preserve process creation identity and per-field availability in both frontends, label CPU normalization, and use the Windows batch sampler for the full TUI inventory. Revise the internal process ABI atomically with native layout assertions.
- Distinguish resource pressure, reported storage faults and incomplete observations in shared findings; expose evidence and next steps in both interfaces (TUI: F).

- Sample physical disk activity independently each second using identity-keyed counter deltas, documented Windows/Linux/macOS units, warmup/reset handling, and nullable latency. Keep SMART refreshes from overwriting activity charts.
- Match Windows health rows by physical device number, remove partition-order health guesses, enumerate every structured macOS physical disk, and parse optional smartctl JSON/exit bitmasks while retaining partial telemetry.
- Correct swapped medium/slow GUI topic metadata and keep one-shot exports within the cancellable worker boundary.
- Scope the initial measurement placeholder to a running monitor session so populated offline/fixture views remain visible.
- Move TUI and engine collection into independent bounded latest-result lanes; isolate native/helper probes in cancellable version-checked subprocesses, skip overdue work, and back off failed providers.
- Render startup progressively, attach actual capture metadata to engine topics, and append timestamped TUI histories only on fresh samples without combining CPU and GPU temperature series.
- Normalize network counters by monotonic elapsed time and invalidate first/reset/resume samples; retain the full process inventory before frontend ranking.
- Parse ICMP reply RTT instead of subprocess duration, correct macOS timeout units, and use a TCP reachability fallback without mislabelling it as ICMP latency.
- Bound collector output and cancellation with owned Windows jobs/Unix process groups and file-backed capture, eliminating inherited-pipe EOF waits and unjoined reader threads.

### Development

- Reconcile candidate installer/self-test ABI assertions with the revised process ABI and print Linux package validation payloads before failed assertions.

- Execute root collector and isolated-engine tests on every native GUI architecture, including the Alpine musl lane; cross-compilation alone is not the provider qualification bar.

- Copy the staged native test model contract back to the source checkout and run strict binding checks, preventing stale contracts from silently reducing validation coverage.
- Track the accepted v4 monitoring and qualification plan; refresh the task board bundle while preserving project identity and existing acceptance items.

## [3.1.3] - 2026-07-25

### Changed

- Removed the application mark's black square plate on Windows and Linux so
  the operator-selected orange/off-white SD/300 block now floats on transparent
  system surfaces. macOS uses a separate art-directed master with transparent
  corners and a softened isometric graphite halo, preserving the same mark
  while giving the Dock a deliberate native silhouette. The monochrome tray
  identity is unchanged.
- Split deterministic application-icon export into a transparent Windows/Linux
  master and a macOS master while retaining prebuilt ICO and ICNS precedence
  for their native package formats. The original operator-selected raster is
  preserved as source evidence rather than used as a plated production asset.

### Fixed

- Extended icon export checks to reject missing transparency, missing
  orange/off-white structures, or a reintroduced dark plate across every Linux
  hicolor size and the shared Windows/runtime image. The Windows release EXE
  still exposes the multi-resolution ICO through associated-icon extraction,
  and the managed archive manifest hash-verifies the replacement PNG/ICO under
  the existing update and uninstall ownership paths.
- Aligned final macOS PKG validation with Native SDK's prebuilt-container
  contract: the explicitly supplied ICNS keeps its `app-icon.icns` basename in
  the app bundle and `Info.plist`, rather than the `AppIcon.icns` name reserved
  for SDK-generated or fallback artwork.

## [3.1.2] - 2026-07-23

### Added

- Replaced the rejected ECG-style artwork with the operator-selected flat
  isometric SD/300 block and a simplified monochrome companion tray glyph.
  Added a deterministic, pinned-SDK export/check step that derives seven-size
  Windows ICOs, macOS ICNS/template assets, a 512 px runtime image, and the
  complete Linux hicolor size set from the committed masters.
- Added a GUI-only `close_to_tray` preference. The Windows/macOS GUI tray and
  close-to-tray behavior now default on, while launch-at-login remains off and
  every CLI/TUI launch remains tray-free. The tray publishes a bounded live
  tooltip covering CPU, memory, GPU, storage, and disk health.

### Fixed

- Embedded the multi-resolution application ICO in `sd300-gui.exe`, assigned
  both Win32 window icon sizes, loaded a dedicated tray ICO instead of passing
  a PNG to `IMAGE_ICON`, and selected black/white tray variants for the current
  Windows system theme. macOS now loads the monochrome template at runtime and
  its app bundle must contain the generated application icon.
- Carried every new identity asset through Windows native archives, WiX and
  Inno installers, managed-install manifests, Installed Apps identity,
  updater hash verification, ownership checks, and conservative uninstall
  verification. Linux packages now install every supported hicolor size.
- Kept executable-only Win32 resource inputs out of Native SDK's test-only COFF
  analysis object, preserving full semantic analysis while allowing the
  resource-bearing Windows release build and Native test graph to coexist.
  Declared libc explicitly for that analysis object on Linux so its
  bundle-relative engine `dlopen` path remains fully checked on GNU and musl.
- Made GUI companion discovery inject its per-user application root so an
  installed public copy cannot contaminate the missing-companion unit test.

### Changed

- Recorded the v3.1.1 release-operations incident and its prevention: a
  mid-release `main` push desynchronized the `workflow_run`-triggered producer
  and qualify chain from the release commit, and the real release's producers
  (branch-head-associated) were briefly misread as rogue. Documented the
  deterministic recovery (delete draft + re-run Release from one commit) and
  the standing rules — never push during a release; producers under a newer SHA
  are not rogue; a single macOS timestamp-signing failure is a transient flake
  — in `AGENTS.md` and the Codex post-mortem addendum. Filed backlog task
  `#rlx` to pin `workflow_run` consumers to the triggering commit and add a
  `Release` concurrency group as a reviewed, standalone hardening change.

## [3.1.1] - 2026-07-23

### Fixed

- Fixed the Release workflow's `source-check` immutability guard failing a
  `main` push when the already-published version's source commit differs from
  its immutable release tag. Post-release commits to `main` (docs, follow-up
  work) legitimately advance past the tagged release commit, so that case now
  skips deployment cleanly (green) instead of erroring. The guard still fires
  on genuine deploy-path conflicts — a fresh version whose tag already exists
  at a different commit, or a crate-published/release-missing hosting repair
  from the wrong commit — so an immutable release can never be re-pointed.
  (v3.1.0 post-release docs commits red under the pre-fix guard; ADR 0005.)

### Changed

- Documented the immutable-tag guard in `AGENTS.md` as a warn-and-investigate
  note so a future agent treats a red "Immutable tag … resolves to …" Release
  check as a real deploy-path conflict to diagnose (bump the version or repair
  from the tagged commit), never something to force or silently bump away.
- Recorded the architecture decisions behind the v3 releases as ADRs: 0004
  (the v3.0.0 release-scope two-bar model — functional bar shipped, evidence
  bar deferred to owned backlog tasks) and 0005 (the in-app update
  coordinator: GUI intent → engine ABI → detached CLI spawn from proven
  absolute paths → owner-preserving transaction → success-gated relaunch).
  `CLAUDE.md` and `AGENTS.md` now point agents at the ADR series before any
  rework of those areas.
- Refreshed `README.md` for post-release accuracy: replaced the stale v3.0.0
  qualification banner with the released status, documented the in-app
  "Update now" surfaces, and completed the command list with `install`,
  `uninstall`, `snapshot --json`, and `capabilities --json`.
- Corrected the contributor testing docs in `CLAUDE.md`/`AGENTS.md`: gui-engine
  is workspace-excluded, so root `cargo test --locked` never runs its tests —
  the recipe and quick reference now name `cd gui-engine && cargo test
  --locked` explicitly and note the crate's accepted non-gated C-ABI clippy
  findings.

## [3.1.0] - 2026-07-22

Released 2026-07-23 00:25 UTC (merge `2b4c9a5`). Public verification on the
released artifacts: the crate installs and reports the new version (`cargo
install tr300-tui --version 3.1.0` → `sd300 3.1.0`), the GitHub release is
published and promoted to `latest` with 59 assets and their SHA-256 sidecars,
`gh attestation verify` returns two SLSA provenance attestations bound to build
commit `2b4c9a5`, and the stable versionless installer URLs resolve.

### Added

- Added safe in-app and tray-driven updates. The Settings page's "Update now"
  action and the tray's "Update SD-300" item ask the GUI engine to start the
  installed CLI as a detached, windowless update coordinator running the
  existing owner-preserving transaction (`update --json --relaunch-gui`). The
  CLI resolves only from proven absolute product locations — the composite
  root's `bin/` sibling of the app directory, the shared flat `bin/` layout,
  the managed receipt's recorded binary, or the fixed macOS PKG path — never
  from a PATH lookup. Coordinator output streams to `update-launch.log`
  beside the GUI settings so failures stay inspectable after the app closes
  mid-update. The GUI process itself never mutates the installation.
- Added the hidden `--relaunch-gui` flag on `sd300 update`, reserved for the
  GUI coordinator: after a successful transaction the CLI reopens the
  installed monitor through the idempotent singleton Open route, so an
  already-current product keeps the running app open and simply focuses it.
  Failures never launch the app, ordinary terminal updates are unchanged, and
  the update's JSON stdout contract, help output, and exit codes are
  byte-identical to v3.0.0.
- Added the `sd300_engine_request_update` engine ABI entry backing the GUI
  action. The returned status is authoritative; the optional caller buffer
  carries the resolved CLI path or the failure reason for the status line.

### Changed

- The Settings About badge now derives its version from the engine's expected
  product version exactly like the sidebar label, and the product-version
  consistency check now treats literal markup versions as optional (any
  literal that remains must still match the release) since visible labels
  bind to the checked engine constant.
- The About panel's lifecycle copy now describes the in-app update handoff;
  installer ownership, elevation, rollback, and JSON contracts remain
  authoritative in the CLI, and uninstall remains CLI-only.

### Fixed

- Fixed the desktop sidebar showing the hardcoded build-state label
  `v3.0.0 QUALIFICATION`. The label now derives from the engine's expected
  product version, so it cannot drift from the shipped version or carry a
  build-state word into a public release.
- Fixed the post-publication crate convergence check, which polled crates.io's
  web API; that API now rejects unidentified clients under its data-access
  policy, so the check failed after a correct publication. It now polls the
  sparse index Cargo itself resolves against, with an identifying user agent.

## [3.0.0] - 2026-07-22

Released 2026-07-22. Public verification on the released artifacts: the crate
installs and runs (`cargo install tr300-tui --version 3.0.0`), public Windows
installer and GUI archive checksums match their sidecars, `gh attestation
verify` passes against the repository, and a physical Windows install
launched the GUI on live hardware and uninstalled with no residue.

### Added

- Added a native-rendered Vercel Native SDK desktop monitor alongside the
  existing Rust CLI/Ratatui TUI, with Overview, CPU, Memory, Disk, GPU, Network,
  Processes, Thermals, and Drivers surfaces backed by the same collector model.
- Added `sd300 gui` to launch or focus the installed app without changing bare
  `sd300`, which continues to open the existing User/Technician chooser.
- Added a dynamically loaded Rust GUI engine with versioned, bounded,
  latest-only projections, explicit ABI/schema/product/target checks, and
  bundle-relative loading. The app owns a separate collector runtime; it does
  not move or replace the TUI event loop.
- Added GUI-only versioned preferences for mode, temperature unit, window
  geometry, chart density, navigation, tray, close behavior, launch-at-login,
  and reduced motion. These settings do not alter TUI startup or session
  defaults.
- Added Windows tray and macOS status-item lifecycles with Open and Quit, both
  default off. Linux intentionally exits on close because Native SDK 0.5.4 does
  not supply the required tray implementation.
- Added the Warm Carbon visual system with a restrained black/charcoal/orange
  field and subtle grid, bundled Makira for primary copy and major numerals, and
  IBM Plex Mono for technical labels and compact measurements.
- Added GUI self-test, product-version consistency, dependency lock,
  developer-path leakage, performance, lifecycle, and compatibility
  qualification surfaces.
- Added complete native User and Technician presentations for all nine
  diagnostic destinations, including source/provenance and shown/total state,
  bounded histories, process sorting/search, network/driver filtering,
  connection paging, disk reliability/activity detail, driver services, and
  explicit unavailable/unsupported/permission-denied observations.
- Added the SD-300 application icon: a Warm Carbon rounded-square badge with
  an amber diagnostic pulse waveform over a subtle grid (generated with Quiver
  arrow-1.1-max, operator-authorized). The icon ships as the GUI window,
  taskbar, and tray identity via the existing `assets/icon.png` path and as a
  multi-size `wix/Product.ico` wired into both MSI editions' Add/Remove
  Programs entries.
- Added asynchronous GUI driver rescans, redacted snapshot and capability
  exports, in-app interaction help, cause-to-detail navigation, persisted last
  destination, and per-destination collection subscriptions.
- Added a fixed-layout process-summary ABI and a reusable Windows process
  sampler that perform one bounded inventory query for GUI process ranking and
  live CPU/memory updates without sharing mutable state with the TUI.
- Added authenticated cross-process GUI lifecycle endpoints for graceful
  close, uninstall/update handoff, singleton focus, and Windows UI-thread Open
  routing, plus launch-at-login ownership and hidden-start support.
- Added target-pinned Windows, macOS, Linux GNU, and Linux musl build wrappers,
  clean-cache and warmed/offline dependency restore checks, package inventory
  and self-test manifests, and target-specific private-runtime packaging.
- Added private GTK runtime closure discovery for Linux, with architecture,
  ELF interpreter/RUNPATH/dependency, distro-owner, license, checksum, and SPDX
  `CONTAINS` validation on blank pinned Ubuntu and Alpine hosts.
- Added immutable v2.0.6 CLI/TUI help, version, parse-error, capability,
  report/redaction, lifecycle, and noninteractive TUI fixtures so the additive
  GUI work cannot silently rewrite the existing terminal contracts.
- Added a Windows performance harness that isolates GUI settings, selects a
  real section, samples window and engine processes independently, records
  first/last memory windows, and fails if the app exits before the requested
  duration.

### Changed

- Extended managed wrappers, Windows MSI/EXE packages, the macOS universal PKG,
  and Linux managed packages to treat the CLI/TUI, GUI, Rust engine, assets,
  integrations, and Linux private runtime as one composite product. Install and
  update remain dormant: they never open the app automatically.
- Extended proven-owner update, repair, rollback, and uninstall semantics to the
  GUI companion. A complete same-version install is still a no-op; a missing or
  corrupt GUI at the current version is a repair; uninstall removes owned CLI
  and GUI state while preserving ambiguous paths and user-exported reports.
- Defined the intentional Cargo v2 migration exception: the first update uses
  Cargo to install the v3 CLI, and the second same-version update performs a
  transactional managed CLI+GUI takeover. Later operations use the managed
  owner.
- Made GUI/TUI feature parity a release invariant. Collector, capability,
  warning, provenance, redaction, and hardware-data improvements must be shared
  rather than independently reimplemented in either frontend.
- Preserved the 1/3/5/15/60-second engine cadences and required visible-window
  fast-topic presentation at least once per second after renderer optimization;
  only hidden/tray mode may coalesce to its required summaries.
- Kept the six established release targets as hard gates:
  `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`,
  `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`, and `x86_64-unknown-linux-musl`. x86_64 covers
  both Intel and AMD; this does not add a Windows ARM64 release.
- Pinned the GUI distribution graph to `@native-sdk/cli` 0.5.4 and Zig 0.16.0,
  including immutable package URLs, integrity/content hashes, per-host Zig
  checksums, and the reviewed Native SDK renderer patch. Global installations
  and developer-local dependency paths are rejected by distribution checks.
- Reworked the Native SDK 0.5.4 software renderer with hash-verified downstream
  fixes for bounded gradient stops, multi-region dirty rendering, retained
  base fragments and glyph coverage, a persistent Windows top-down DIB/memory
  DC, GTK 4.0/4.10 dialog compatibility, configured-entry model hashing, and
  deterministic ReleaseFast tests against pristine npm restores.
- Kept one-second foreground collection/presentation while replacing full
  per-tick JSON process envelopes, repeated timer construction, and redundant
  process inventories with bounded binary projections, reusable buffers,
  stable process ranking, in-place counter refresh, and a repeating timer.
- Disabled Native SDK per-event trace serialization in distributable builds
  while retaining panic capture, explicit self-test output, renderer
  instrumentation, and opt-in qualification traces.
- Expanded GUI settings and observation ownership so corrupt settings recover
  through an atomic rewrite, user exports survive uninstall, startup/tray
  choices remain independent, and GUI preferences cannot change TUI defaults.
- Reworked Windows MSI/EXE and managed takeover transactions so Cargo ownership
  transfer is journaled and byte-restorable across `.crates.toml`,
  `.crates2.json`, the Cargo binary, and the managed receipt; only the exact
  proven `tr300-tui` owner is retired after companion qualification succeeds.
- Hardened managed archive extraction against links, special files, duplicate
  canonical paths, manifest/inventory/hash disagreement, traversal, and
  concurrent PATH/profile mutation; rollback restores the exact previous
  PATH/profile/GitHub Actions state where owned.
- Expanded exact-source release qualification with immutable v2.0.6 public
  wrapper/archive/MSI/EXE hashes, synthetic-prior and real-prior update/repair/
  uninstall lanes, same-version companion repair, deliberate failure
  injection, and branch-only qualification that cannot publish or replace
  release assets.

### Fixed

- Prevented Windows collector subprocesses from flashing console windows by
  applying the no-window contract to every GUI-owned command path.
- Fixed blank or partially rendered Native SDK frames caused by gradient-stop
  overflow, repeated platform-source compilation, incorrect configured-entry
  hashing, and stale damage/glyph coverage.
- Fixed the high foreground CPU path caused by repeated software command-list
  replay, per-present DIB reconstruction, development event tracing, full
  process-envelope serialization, duplicate process queries, and timer churn.
- Fixed a false one-frame process CPU spike by committing the Windows process
  sampling baseline only after the complete fallible inventory query succeeds.
- Fixed tray, startup, singleton, and close-policy disagreements: hidden startup
  is explicit, tray-off close quits, repeated Open focuses the existing process,
  and the Windows private Open message is handled on the Native SDK UI thread.
- Fixed composite installer defects found by real and hosted trials, including
  overlong WiX deferred CustomActionData, deferred `CARGO_HOME` resolution,
  repair-time `FileKey` validation, same-version reinstall properties leaking
  into major upgrades, pre-qualification Cargo cleanup, missing notice
  components, and GUI self-tests that incorrectly required a console stdout
  pipe under Windows Installer.
- Fixed Global Inno uninstall hooks that attempted `ExecAsOriginalUser` in an
  unsupported uninstall context; update/install still preserve the original
  user where required, while uninstall uses the proven owner token.
- Fixed Windows Installer committed-result handling so exit 1641/3010 is
  treated as committed reboot success and later verification failures do not
  perform an unsafe CLI-only rollback of an already-committed MSI transaction.
- Fixed managed Windows retry cleanup so Start/Search, Installed Apps, PATH,
  receipt, and shortcut ownership are retried even if an interrupted earlier
  attempt already removed the GUI payload root.
- Fixed Linux native package construction across merged-`/usr` ownership,
  private GTK search paths, musl dynamic `cdylib` linking, container Git trust,
  runtime dependency traversal, Alpine public-domain license evidence, Debian
  common-license symlinks, and architecture-specific blank-host launch.
- Fixed macOS hosted builds for Xcode's Security/libDER search path, duplicate
  platform sources, stable engine install identity, and release debug/developer
  path leakage before signing.
- Fixed the large-output command-drain regression test to use its existing slow
  command deadline on hosted Windows while leaving production probe deadlines
  unchanged.
- Replaced the unbounded actionlint/ShellCheck integration that left multiple
  hung `actionlint.exe` processes with bounded workflow parsing and separate
  ShellCheck validation over extracted Bash blocks and repository scripts.
- Fixed managed PowerShell uninstall failing in noninteractive Windows
  PowerShell when the receipt parent directory contained unrelated files: the
  cleanup now removes the parent only when empty via nonrecursive
  `[IO.Directory]::Delete`, treats Win32 `ERROR_DIR_NOT_EMPTY` (145) as the
  expected preservation outcome, and rethrows every other failure into the
  existing rollback path. Hosted qualification now plants an unrelated sibling
  beside the receipt and requires byte-exact preservation plus complete
  owned-state removal (ADR 0003).
- Fixed the same cleanup reporting a false failure under Windows PowerShell
  5.1, whose `-Command` exit code mirrors the last statement's `$?`: a
  tolerated outcome (caught nonempty-parent exception or suppressed removal)
  in final position exited 1 despite correct behavior. The command string now
  ends with a terminal `exit 0`; uncaught errors still abort with a nonzero
  exit before reaching it (ADR 0003 addendum, proven on real 5.1).
- Fixed updater PowerShell discovery on stock Windows: the `--version` spawn
  probe is a Windows PowerShell 5.1 parser error, so machines without
  PowerShell 7 reported the in-box shell as missing and failed managed-channel
  updates and asset downloads. The updater now resolves the trusted System32
  `powershell.exe` image directly (also hardening against PATH interception)
  and uses a PATH-resolved PowerShell 7 only as the fallback.
- Fixed managed updates failing with "Get-FileHash is not recognized" when the
  updater is launched from a PowerShell 7 session: the child in-box shell
  inherited pwsh's `PSModulePath`, which shadows Windows PowerShell 5.1's
  built-in modules with Core-only editions and breaks cmdlet auto-loading.
  Every PowerShell child the updater spawns now starts with a cleared
  `PSModulePath` so the shell rebuilds its own defaults.
- Fixed retired MSI Cargo-transaction journals stranding their empty
  product-owned `Transactions` directory inside the receipt root after commit
  or rollback cleanup, which kept the receipt root from emptying at uninstall.
  The directory is now removed only when empty; unrelated content preserves it.
- Fixed the severe warmed-state scroll lag on scrollable GUI sections: every
  scroll frame is architecturally a full-viewport software repaint
  (~16-22 ms), and queued wheel messages could arrive faster than that service
  rate on the single-threaded update queue — after the 60-sample histories
  fill, mandatory per-second chart repaints join the same queue and the
  backlog becomes user-visible. The Windows host now coalesces same-axis,
  same-modifier wheel bursts into one scroll input at the summed delta, so a
  burst costs one reconcile and one repaint at the final offset (ADR 0002;
  measured by the new warmed-state benchmark).
- Fixed a follow-on wheel regression where one physical click could glide the
  entire scroll range: the runtime derives kinetic velocity from the same
  wheel delta the Windows host now coalesces, so a summed burst became
  runaway momentum. Wheel momentum is disabled at the app design-token level
  (`wheel_velocity_scale = 0`), keeping the coalesced one-repaint burst while
  making every notch a bounded step in the wheel's direction.
- Fixed Global EXE uninstall failing verification on fast machines: the Inno
  uninstaller finishes through a relaunched copy whose final machine-PATH
  removal lands after the original process returns, so owned-state
  verification raced an eventually-consistent uninstall. Verification now
  polls briefly for convergence, and elevated worker failures relay their
  detail (including the uninstaller log tail) across the UAC boundary
  through a report file instead of dying with an opaque exit code.
- Fixed retiring a Global or Corporate EXE owner (channel takeover) stranding
  an orphaned, unrunnable `unins000.exe` in the retired root: Inno
  uninstallers self-delete through a relaunched temp copy after the original
  process returns and can lose that race. The retirement now waits briefly
  for self-deletion and reaps a data-less leftover uninstaller, reclaiming
  the directory only when empty.
- Fixed Cargo ownership evidence (`.crates.toml`/`.crates2.json`) being
  rejected when a Windows PowerShell 5.1 writer prefixed a UTF-8 byte-order
  mark: detection reads now strip a leading BOM, while transactional manifest
  editing stays byte-exact and fails safely.
- Fixed managed receipts written with a UTF-8 byte-order mark (any Windows
  PowerShell 5.1 `Set-Content -Encoding utf8` writer) failing ownership
  verification with "does not prove an exact cargo-dist binary": receipt
  parsing now strips a leading BOM in both the updater and the migration
  engine, and the qualification fixture writes BOM-less receipts like the
  real installer.
- Fixed a mid-session tray toggle stranding a hidden, icon-less process (or
  quitting past a still-live tray icon): the close-to-tray quit decision now
  consults the startup-effective tray presence for this session rather than
  the persisted preference, matching the RESTART REQUIRED semantics.
- Fixed up to 30 seconds of stale data after restoring a minimized window:
  collection cadence now follows the close-to-tray policy-hidden state rather
  than raw visibility, so a minimized window keeps one-second sampling and the
  foreground collection profile and restore is instantly fresh; tray-hidden
  windows keep the 30-second cadence.

### Removed

- Removed the automatic Claude Code Review pull-request workflow. It failed
  externally before producing any review turns or findings on every recent run
  (for example exact-head run `29892152564`) and therefore added a permanently
  red check without review value. Independent review is performed in-session;
  the mention-triggered Claude workflow is unaffected.

### Security and release integrity

- Moved the commercial Makira source face out of the public Git graph and into
  trusted-runner reconstruction guarded by exact SHA-256. This protects the
  bytes but does not establish an app-embedding license; license evidence is
  still required before publication.
- Required source commit, triggering SHA, coordinated product version, draft
  target, immutable tag, package manifest, checksums, SBOM, and attestations to
  agree before an artifact can be attached or promoted.
- Required exact owned paths, receipt/registration identity, publisher,
  install scope, artifact kind, and trusted system executables before Windows
  cleanup, takeover, or uninstall can authorize mutation.
- Preserved unrelated Cargo packages, user-exported reports, ambiguous paths,
  and nonempty shared state roots across takeover, rollback, repair, and
  uninstall qualification.

### Qualification completed so far

- Exact-head ordinary CI passes Rust format/Clippy/tests/release/target checks,
  security audit, and Native GUI build/test/package lanes for Windows x86-64,
  macOS Intel/Apple Silicon, Linux GNU x86-64/ARM64, and Linux musl x86-64.
- Physical Alienware qualification passes all nine destinations, both audience
  modes, keyboard navigation, maximized scaling, redacted export, singleton
  focus, hidden startup, repeated close-to-tray, launch-at-login add/remove,
  default close/exit, and exact adjacent-engine self-test.
- Physical Corporate MSI qualification passes injected post-Cargo failure and
  exact rollback, successful v2.0.6 takeover, current-version missing-engine
  repair, CLI/snapshot/capability checks, GUI launch/focus/export, supported
  uninstall, user-export preservation, and exact restoration of the original
  Cargo-owned v2.0.6 fixture.
- The release-shaped Processes workload passed a 15-minute foreground sample
  at 1.58% of one logical core with 84.78 MiB average working set and 227.2 MiB
  average private memory, and a 30-minute hidden sample at 0.18% with 64.98 MiB
  working set and 206.65 MiB private memory.
- Local gates pass 107 Rust unit tests, seven immutable-v2 CLI compatibility
  tests, release build, crates.io dry run, strict product-version checks,
  dependency/path-leak verification, and 31 optimized Native SDK tests with
  one expected platform skip.

### Known open qualification issues

- The operator reports severe scrolling/input lag on scrollable GUI sections
  after the end-user app has been open for roughly one minute. Average CPU and
  memory samples do not clear this release-blocking interaction regression.
- The first exact two-hour soak attempt was invalidated by an operator window
  close (attributed 2026-07-22 with harness, code-path, and event-log evidence;
  ADR 0001 — not a product defect). The pre-release soak gate was explicitly
  waived by the operator; the two-hour soak and formal frame/input percentile
  evidence move to a tracked post-release task against released bytes.
- Exact-head Windows installer qualification previously failed managed
  PowerShell uninstall on a nonempty-receipt-parent prompt. The empty-only
  cleanup and unrelated-sibling preservation proof are now committed and
  locally validated (ADR 0003); a fresh hosted Windows qualification run
  remains the authoritative Windows PowerShell 5.1 proof.
- Native SDK exposes the Windows/Linux GUI as a named canvas but not as an
  internal screen-reader control tree. The existing TUI remains the documented
  accessible fallback until the SDK provides that platform capability.
- Makira app-embedding license evidence or an authorized open-font replacement,
  signed/notarized final packages, provenance attestations, immutable tag and
  release publication, fresh public-byte verification, website verification,
  and final physical acceptance remain incomplete.

### Release qualification

- Requires the existing CLI/TUI contracts and lifecycle behavior to remain
  compatible while qualifying the composite product on every release target.
- Requires foreground, hidden/tray, and soak performance gates: at most 2% of
  one logical core foreground, 1% hidden/tray, 150 MiB working set/RSS,
  300 MiB private memory/commit, 16.7 ms frame-time p95, 50 ms input-response
  p95 outside explicit scans, and no unbounded growth.
- Requires SHA-256 sidecars, an SPDX SBOM, GitHub build-provenance and SBOM
  attestations, exact-tag asset verification, and public-byte verification
  before v3.0.0 is treated as released.

## [2.0.6] - 2026-07-19

### Fixed

- Corrected the Windows native-to-managed qualification probe so an intentionally removed `InstallSource` registry property is treated as successful cleanup instead of a terminating PowerShell error. The failed v2.0.5 candidate remained an unpublished draft and was not retagged.

## [2.0.5] - 2026-07-19

### Fixed

- Added a rollback-capable Windows live-image handoff to `sd300 install`, allowing an intentional managed PowerShell reinstall to replace the currently running managed, Cargo, or Corporate binary without colliding with its open executable.
- Added a tightly validated elevated worker for deliberate Global MSI/EXE to managed-PowerShell takeovers, with exact release pinning, ownership revalidation, rollback, and bounded trusted cleanup.
- Expanded Windows release qualification so all four native channels must successfully transfer ownership through `sd300 install --json`, remove their native registration, marker, PATH entry, and payload root, verify the managed receipt/binary, and then uninstall cleanly.

## [2.0.4] - 2026-07-19

### Fixed

- Made Windows Config Manager problem codes authoritative over generic `Win32_PnPEntity` status text, eliminating false driver warnings when SetupAPI and `ConfigManagerErrorCode=0` agree that a device has no problem.
- Centralized driver attention semantics across the overview, snapshot warnings, and Drivers page; User Mode now surfaces genuine issues from every counted category instead of hiding System or Other devices.
- Kept real GPU temperature telemetry available when CPU sensors are absent, and split CPU, GPU, aggregate temperature, and fan capability states so one missing provider no longer marks all thermals unsupported.
- Added read-only Windows thermal providers for Libre Hardware Monitor/Open Hardware Monitor WMI bridges and guarded Dell AWCC temperature/fan enumeration. Dell firmware access is reported as permission-gated when the process is not elevated; no thermal-control method is invoked.
- Added sensor kind/source provenance to diagnostic snapshots and the Technician thermal table.

## [2.0.3] - 2026-07-19

### Fixed

- Added a rollback-capable Windows native-uninstall live-image handoff so `sd300 uninstall` can return its final result instead of being terminated by MSI or EXE Restart Manager while removing the running binary.
- Removed empty SD-300 receipt directories after managed shell, managed PowerShell, and macOS PKG uninstall while preserving shared Cargo/Rust tooling and non-empty shared directories.
- Changed Windows native uninstall to resolve `msiexec.exe` from the trusted Windows system directory rather than executable-search `PATH`.
- Expanded release qualification so the CLI itself must completely remove all four Windows MSI/EXE channels, macOS managed shell and PKG channels, and Linux managed shell/Cargo channels, including registrations, markers, owned PATH entries, receipts, and payload roots.

## [2.0.2] - 2026-07-19

### Fixed

- Normalized cargo-dist's generated managed-install receipt identity and path from the historical package name `tr300-tui` to the product identity `sd300`, so production shell and PowerShell installs can be proven and updated through the managed channel.
- Added release-assembly assertions for the exact managed receipt identity, directory, and filename in both generated installer formats.

## [2.0.1] - 2026-07-19

### Fixed

- Drained child-process stdout and stderr concurrently so updater release checks cannot deadlock when GitHub's latest-release response exceeds an operating-system pipe buffer.
- Added repeatable post-public artifact and lifecycle qualification without republishing an already-public crate or release.

## [2.0.0] - 2026-07-18

### Added

- Added `sd300 install`, `sd300 uninstall`, `sd300 snapshot`, and `sd300 capabilities` with exactly-one-object JSON lifecycle output and privacy-redacted noninteractive diagnostics.
- Added stable `recovery_url` and `requires_user_action` fields to every JSON install, update, and uninstall result for cross-product automation parity.
- Added proven install-channel detection and same-channel updates for managed PowerShell/shell, Cargo, Global/Corporate MSI, Global/Corporate EXE, and signed universal macOS PKG ownership.
- Added stable, versionless public wrappers and native artifact names with SHA-256 sidecars, plus immutable 1.4.x compatibility routers that preserve provable native ownership across the v2 transition.
- Added authoritative fresh-install takeover with strict, bounded migration cleanup, rollback, downgrade-capable MSI packages, and opposite-scope Windows refusal before mutation.
- Added separate Global and Corporate Windows MSI/EXE packages, a direct Developer ID signed/notarized universal macOS PKG, and draft release qualification across Windows, Apple Silicon, Intel Mac, and Linux before publication as `latest`.
- Gated crates.io publication behind the same complete native asset and test matrix so v2 is not publicly installable through Cargo before Windows and macOS qualification succeeds.
- Changed raw Cargo ownership detection to require Cargo's structured install manifest, exact package/binary ownership, and matching version instead of trusting the `.cargo/bin` path alone.
- Required exact cargo-dist receipt fields in both updater detection and takeover cleanup; recursively nested lookalike keys no longer count as ownership proof.
- Added a Windows live-image handoff with an elevated, same-channel Global worker, verified rollback, and bounded detached cleanup so MSI/EXE Restart Manager behavior cannot kill the updater's final result or strand the prior executable.
- Resolve overlapping exact Cargo/managed ownership by the newer metadata record and fail closed on equal timestamps instead of guessing.
- Added candidate-to-candidate hosted version-transition gates for managed PowerShell, four Windows native channels, direct PKG on Intel/Apple Silicon, and Linux managed shell/Cargo ownership.
- Managed CLI wrappers now SHA-256 verify their exact-tag cargo-dist installer payload before execution.
- Added Windows memory-module, multi-GPU, display, physical-disk health/reliability, battery, hardware identity, native adapter/link-speed, and SetupAPI/WMI driver-status diagnostics.
- Added typed observation provenance so unavailable, unsupported, permission-denied, contradictory, and error states are never presented as fabricated zero telemetry.

- Added an exhaustive, privacy-sanitized macOS hardware-monitor capability and implementation report based primarily on live testing of a `Mac14,7` M2 MacBook Pro running macOS 26.3.1.
- Documented locally proven unprivileged access to 38 IOHID temperature services, read-only AppleSMC fan telemetry, IOReport energy/frequency residency, Metal/AGX data, battery/adapter internals, APFS/NVMe status, block-I/O counters, Wi-Fi radio state, displays, and device inventories.
- Added implementation-ready Rust guidance covering module boundaries, target dependencies, FFI ownership, exact private-interface call sequences and SMC ABI layout, collector cadence, availability/provenance types, redaction, fixtures, CI, and later Mac qualification.
- Documented the macOS 26 unprivileged `diskutil` NVMe SMART-detail dictionary, including spare/life-used/temperature/lifecycle/error fields, checked conversion rules, and its explicit non-guaranteed cross-version status.
- Added sanitized real request/response examples for `system_profiler`, `diskutil`, I/O Registry, IOHID, AppleSMC, and IOReport, with exact command envelopes and typed parser schemas but no stable machine, account, network, or device identifiers.
- Added a critical-thinking inquiry canvas that separates exact-host observations, public contracts, private/model-specific behavior, and unverified fleet assumptions.

### Changed

- Made `irm .../sd300-cli-installer.ps1 | iex` the recommended Windows install and `curl .../sd300-cli-installer.sh | sh` the recommended macOS/Linux install; native installers remain first-class options.
- Made a deliberate fresh official install the authoritative latest user intent regardless of installed version, while raw `cargo install tr300-tui` remains an advanced unmanaged option because Cargo provides no post-install ownership hook.

- Corrected project documentation so macOS is described as a supported baseline with known telemetry gaps instead of claiming comprehensive IOKit-based/full monitoring.
- Recorded Vercel Labs Native as an optional pre-1.0 GUI experiment while preserving the Rust CLI and Ratatui TUI as the canonical interfaces.

## [1.4.3] - 2026-05-11

### Changed
- Changed the crates.io package name to `tr300-tui` while keeping the product name SD-300 and the installed command `sd300`.
- Updated `sd300 update` so its Cargo strategy runs `cargo install tr300-tui --force`; installer fallback URLs now use the package-derived `tr300-tui-installer.*` cargo-dist assets while still installing the `sd300` binary.
- Clarified that `sd300`, `sd300 --user`, `sd300 --tech`, `sd300 update`, and legacy `sd300 --update` remain the standard user commands after installing from `tr300-tui`.
- Updated README, project context, project plan, local agent docs, and global Codex agent guidance to document `cargo install tr300-tui` as the supported Cargo install path.
- Kept the hand-edited WiX/MSI product name as `sd300` and allowed the MSI customization in cargo-dist config so the package rename does not rebrand the app installer.
- Kept only non-conflicting legacy `SD300-installer.sh` and `SD300-installer.ps1` aliases for older 1.4.0/1.4.1 installer fallback compatibility; lowercase `sd300-*` release-asset aliases were removed because GitHub release assets are case-sensitive in display but can conflict during upload.
- Bumped the release version to `1.4.3` for a clean crates.io and GitHub Release publish under the new package name.

## [1.4.2] - 2026-05-11

### Changed
- Switched the canonical crates.io package metadata to lowercase `sd300` so the supported Cargo install path is `cargo install sd300` while the installed command remains `sd300`.
- Removed the accidental uppercase `SD300` package from the release path and prepared the package for lowercase `sd300` publication.
- Updated `sd300 update` to use `cargo install sd300 --force` and lowercase cargo-dist installer asset URLs.
- Updated WiX/MSI product naming to lowercase `sd300`.
- Updated README, changelog, project context, project plan, local agent docs, and global Codex agent guidance for lowercase package, install, update, and release behavior.
- Release automation now publishes the crate after all cargo-dist artifacts build but before hosting the GitHub Release, reducing partial-release risk if crates.io rejects a publish.
- Release source-check can now repair a crates.io-published/GitHub-release-missing partial state by rebuilding artifacts and finishing release hosting.

### Added
- GitHub release uploads legacy `SD300-installer.sh` and `SD300-installer.ps1` aliases alongside lowercase installer assets so already-installed `1.4.0`/`1.4.1` updaters can still fall back to the installer path.

The `1.4.0` and `1.4.1` entries below are retained as historical notes for the
short-lived uppercase crates.io package path. `1.4.2` supersedes that path with
the lowercase `sd300` package metadata, and `1.4.3` supersedes it with the
publishable `tr300-tui` crates.io package while preserving the `sd300` command.

## [1.4.1] - 2026-05-11

### Changed
- Corrected Cargo installation documentation after release verification: the originally published crates.io package was `SD300`, `cargo install` required that package casing, and the installed command remained lowercase `sd300`.

## [1.4.0] - 2026-05-11

### Added
- `sd300 update` command form while preserving the legacy `sd300 --update` flag.
- Updater dispatch before Ratatui terminal initialization so update failures cannot leave the terminal in an altered TUI state.
- GitHub release updater that checks `QubeTX/qube-system-diagnostics` latest-release JSON, compares semantic version segments, exits `0` when current/successful, and exits `2` on update-check or update-attempt failure.
- Ordered updater strategies with per-attempt diagnostics:
  - Cargo first only when `cargo --version` succeeds, using `cargo install SD300 --force`.
  - macOS/Linux fallback through the cargo-dist shell installer with hardened `curl`, then `wget`.
  - Windows fallback through the cargo-dist PowerShell installer with `powershell.exe`, then `pwsh.exe`.
- New install and update documentation covering all supported installation paths: macOS/Linux shell installer (`SD300-installer.sh`), Windows PowerShell installer (`SD300-installer.ps1`), Windows MSI (`SD300-x86_64-pc-windows-msvc.msi`), Cargo (`cargo install SD300`), and source builds.
- Shared bounded command runner for collector subprocesses with timeout and kill behavior.
- Background startup and refresh jobs for connectivity, disk health, and driver scans so the TUI can render while slower probes run.
- CI workflow covering Ubuntu, macOS, and Windows with format checks, Clippy, tests, release build, target checks, audit, and `cargo-dist` plan.
- ND-300-style release workflow that can deploy from `main` when the current version is unreleased, while preserving explicit `v*.*.*` tag releases.
- Release source-check job that reads package metadata from `Cargo.toml`, checks crates.io version state, GitHub Release state, and remote tag state, skips fully published versions, and fails partial-release states before artifacts or crates are published.
- Initial crates.io publish job for the short-lived uppercase `SD300` release path; this was later superseded in `1.4.2` by the lowercase `sd300` package and publish-before-hosting release order.
- Tests for CLI update parsing/help/conflicts, updater strategy ordering and version comparison, bounded command timeout behavior, gateway/socket parser fixtures, and macOS disk/system-profiler parsers.
- `CODEX_PROJECT.md` project context file with current status and file tree.
- Local `AGENTS.md`, `CLAUDE.md`, and global Codex agent guidance documenting the SD300 release, publish, installation, and update workflows.

### Changed
- Bumped package version to `1.4.0` and crate package name to `SD300`; the installed binary remains `sd300`, and the Rust library target is `sd_300`.
- Added a crates.io package include list so published packages contain source, WiX manifest, Cargo metadata, changelog, license, README, and toolchain files without unrelated workspace files.
- Set Rust `1.95` as the explicit MSRV via `rust-version` and `rust-toolchain.toml`.
- Updated `sysinfo` to `0.39.x` and migrated to persistent `Networks`, `Disks`, and `Components` refresh handles.
- Updated direct `crossterm` to `0.29` to align with the Ratatui dependency tree.
- Removed the Rust HTTP client dependency from updater code; release metadata is fetched through bounded platform-native command helpers to avoid extra TLS/native build surface.
- Updated cargo-dist metadata for `SD300-*` release artifacts, shell/PowerShell/MSI installers, `CARGO_HOME` install path, and `allow-dirty = ["ci"]` because `.github/workflows/release.yml` is intentionally customized.
- Updated WiX packaging names, prompts, and install folder names from the old `sd-300` package identity to `SD300`.
- Network throughput now uses persistent refresh deltas instead of reconstructing network state each tick.
- Network interface display now includes operational state and uses Ratatui `Table`; bottom navigation now uses Ratatui `Tabs`.
- TUI sections now use more Ratatui-native composition and shared responsive helpers for bordered panels, gauges, tables, scroll indicators, and compact status rows.
- macOS audio driver collection now parses `system_profiler -json` instead of scanning JSON as plain text.
- macOS disk health parsing now has a dedicated `diskutil info` parser for model/media type detection.

### Fixed
- Update handling now happens before terminal initialization, avoiding dirty terminal state after update failures.
- Linux network operational states are normalized distinctly instead of treating every non-empty state as equivalent.
- External collector commands are bounded and degrade to unavailable/unknown data instead of freezing the app.
- Connectivity and disk-health refreshes no longer block the draw loop; they run as bounded background jobs and update warnings when complete.
- Windows Setup API driver scanning releases device-info handles through RAII cleanup even on early returns.
- macOS and Linux driver, disk, GPU, thermal, route, DNS, and socket probes use read-only commands with timeouts instead of unbounded `Command::output()` calls.
- Release metadata parsing now works on Ubuntu 22.04 runners without requiring Python `tomllib`.
- Clippy warnings blocking `-D warnings` were resolved across sorting, clamps, and key-event matching.

## [1.3.0] - 2026-03-12

### Added
- Man page generation via `clap_mangen` — `sd300.1` built automatically at compile time
- Enriched `--help` output with full keybindings table and all 9 diagnostic sections
- Scroll support for Drivers section (Tech Mode) — `j`/`k` keys with position indicator
- Scroll support for Disk section (Tech Mode) — `j`/`k` keys with position indicator
- `SPARK_SWAP` named color constant for swap sparkline consistency
- `Shift+M` keybinding for sorting processes by memory usage

### Changed
- Disk Tech Mode now uses bordered `sub_block()` panels for Partitions and Physical Drives (matches CPU/Memory layout)
- Network Tech Mode caps interface list at 8 entries with "+ N more" indicator (prevents layout overflow on Docker/WSL hosts)
- Label padding standardized to 18 chars in Memory section (was 20, now matches all other sections)
- Swap sparkline uses dedicated `SPARK_SWAP` constant instead of reusing `COLOR_WARN`
- Process sort header updated: `[m]emory` → `[M]emory` to reflect actual keybinding
- CPU per-core gauge width documented with clarifying comment (16-char fits 50% split at 80-col minimum)

### Fixed
- **Memory sort keybinding unreachable (BUG)**: `m` was bound to "return to mode selection" globally, making `ProcessSortKey::Memory` impossible to activate — now uses `Shift+M` which doesn't conflict

## [1.2.2] - 2026-03-12

### Changed
- Upgraded cargo-dist from v0.30.3 to v0.31.0 (includes bugfixes from v0.30.4 and installation robustness improvements)
- Updated GitHub Actions: checkout v4→v6, upload-artifact v4→v6, download-artifact v4→v7

## [1.2.1] - 2026-02-09

### Changed
- **Driver scanning replaced WMI with Windows Setup API**: Device enumeration now uses `SetupDi*` functions and Configuration Manager (`CM_Get_DevNode_Status`) instead of `Win32_PnPSignedDriver` WMI queries — immune to WMI repository corruption
- **Service status via Service Control Manager**: Replaced `Win32_Service` WMI query with direct SCM API (`OpenSCManager`/`QueryServiceStatus`) for the 13 monitored services
- Driver version and date now read directly from registry (`HKLM\SYSTEM\CurrentControlSet\Control\Class`) instead of WMI
- Renamed `DriverScanStatus::WmiUnavailable` to `ScanFailed` to reflect the API-agnostic implementation
- Updated error messages to remove WMI-specific language

### Added
- `windows` crate v0.62 dependency with Setup API, Registry, and Services features

## [1.2.0] - 2026-02-09

### Added
- Header bar with mode badge (User/Tech) and UTC clock across all screens
- `content_block()` and `sub_block()` helpers for consistent bordered panels (rounded borders)
- Temperature sparkline now visible in User Mode thermals (was Tech-only)
- Fan RPM display in User Mode thermals (was Tech-only)
- "Scanning..." animated state for driver tab during WMI scans
- Pipe separators (`|`) and right-aligned "? Help" hint in bottom navigation bar

### Changed
- **Complete UI overhaul**: Warm earth color palette replacing neon terminal colors
  - Sage green (good), warm amber (warnings), terracotta red (critical), warm gold (accent)
  - Slate blue (info), warm gray (dim/muted), warm white (text)
- All 9 tabs x 2 modes now use bordered content panels with rounded corners
- Gauge bars standardized to 20-character width across all sections, cleaner Unicode blocks
- Bottom bar: active tab uses warm gold on dark background, inactive tabs in muted gray
- Mode select screen: rounded borders, warm palette (sage for User, amber for Tech)
- Help overlay: warm palette with gold accent keys
- Overview User Mode: shows 5 top processes (was 3), wrapped in sub-panels
- Sparkline colors updated: warm gold (CPU), muted purple (memory), slate blue (network), sage (GPU), amber (temp)

### Fixed
- **Driver tab UI freeze (CRITICAL)**: WMI device scanning now runs asynchronously via `tokio::spawn_blocking` with `JoinHandle` polling, preventing 2-10s UI freezes
- Manual driver refresh ('r' key) no longer blocks the event loop
- WMI error messages now suggest running as Administrator
- Removed unused `Modifier` imports in cpu.rs and overview.rs

## [1.1.0] - 2026-02-08

### Added
- Ctrl+C to quit from any screen (OS-independent)
- Scroll indicators ("Showing X-Y of Z") on process table and network connections
- Network connections section documented in help overlay (j/k scroll hint)
- Temperature threshold constants (`TEMP_CPU_WARN/CRIT`, `TEMP_GPU_WARN/CRIT`) for consistent behavior

### Changed
- Extracted `truncate_str()` to shared `common.rs` (removed 8 duplicate copies)
- Consolidated `health_gauge_line()` and `health_gauge_line_simple()` into `common.rs`
- Replaced hardcoded refresh intervals with named constants (`REFRESH_FAST/SLOW/MEDIUM/DIAG/HEALTH`)
- Replaced hardcoded history buffer size with `HISTORY_SAMPLES` constant
- Fixed inconsistent temperature thresholds across overview, thermals, and CPU sections
- Tech mode sensor coloring now uses same thresholds as user mode (was 80/95, now 70/85)

### Fixed
- 13 Clippy warnings resolved (collapsible if, map_or to is_none_or, redundant to_string, useless format!, let_unit_value, manual range check)

## [1.0.0] - 2026-02-07

### Added
- Initial release with 9 diagnostic sections
- User Mode (plain language) and Technician Mode (raw data)
- Real-time monitoring: CPU, memory, disk, GPU, network, processes, thermals, drivers
- Cross-platform support: Windows, macOS, Linux (x86_64 + ARM)
- WMI-based driver scanning and SMART disk health (Windows)
- Network connectivity diagnostics (gateway, DNS, internet)
- Active connection monitoring with protocol/state/PID
- Temperature unit toggle (Celsius/Fahrenheit)
- cargo-dist release workflow with shell/powershell/MSI installers
