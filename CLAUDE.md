## v4 implementation contract (2026-09-23)

The accepted v4 plan governs the collection, terminal and compatibility contracts below.
`presentation.rs` prepares inventory sorting, filtering, identity selection and inspection;
`ui/dashboard.rs` renders only visible rows. `Monitor` owns independent bounded lanes in
both frontends. Space freezes terminal presentation while collection continues; `/`,
Enter, page navigation and optional mouse are additive. GUI and TUI settings are separate
namespaces. Histories use capture timestamps and preserve missing time buckets. See
[ADR 0006](docs/adr/0006-v4-sampling-and-terminal-contract.md) and
[ADR 0007](docs/adr/0007-adaptive-presentation-and-time-buckets.md).
Every candidate remains unpublished until its accepted gates pass. ADRs 0016/0017
record the original v4 performance decision; ADR 0018 extends those same ceilings
to the operator-authorized 4.0.1 updater correction. All original goals remain in
[Next-version targets](docs/next-version-targets.md) and automatically apply after 4.0.1; track responsiveness in #r16 and resources in #r17.

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Task management system

This repository uses the git-tracked `.tasks/` board for milestones, active work, verification, and cross-session handoff. At session start, read `.tasks/TASKS.md`, `.tasks/MILESTONES.md`, `.tasks/CLAUDE.md`, and every Active task's detail file. Keep each Active task's `## Status`, `## Verification`, and newest `## Activity` entry current as work progresses. Launch or repair the live board with `node .tasks/board-server.mjs ensure --open` and read its identity-bound port from `.tasks/.board-server.json`.

### Skill routing and current guidance

Use `/tasks-start` to initialize, repair, upgrade, relaunch, or resume the board.
`/tasks-create` is the preferred way to add a well-formed milestone, task, or proper
dashboard-visible subtask; `tasks-management` is the format and completion contract.
Use `/tasks-update` to upgrade and reconcile the existing board, sync/triage current
work, and refresh memory. `tasks-memory` governs that memory, `tasks-boards` governs
live-server identity, and `/tasks-remove` decommissions the system. As work changes,
keep `.tasks/TASKS.md` plus each Active task's `## Status` and `## Activity` current.

If the installed tasks plugin is missing or may be older than the board, first try the
harness-native plugin update. If that is unavailable, fails, or still leaves freshness
uncertain, use the GitHub skill/connector to read the relevant current `main` file under
`RealEmmettS/shaughv-tasks/skills/<skill-name>/SKILL.md` and use it as the latest
operating guidance: https://github.com/RealEmmettS/shaughv-tasks/tree/main/skills

## Changelog rule

This repository maintains two changelogs in parallel:

- `CHANGELOG.md` is the technical record. Preserve the project's existing release,
  file, command, metric, and qualification detail here.
- `HUMAN_CHANGELOG.md` is the plain-English companion. Every technical entry must have
  a corresponding explanation for a non-engineer: no version numbers, code references,
  or jargon, just what changed and why it matters.

Update both files in the same commit. Translate internal-only changes under **Behind the
scenes** rather than omitting them, and use the plain labels **Added**, **Improved**,
**Fixed**, **Removed**, **Security**, and **Behind the scenes** as applicable.

## Build & Test Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (with LTO)
cargo run                      # Run TUI (interactive mode selection)
cargo run -- --user            # Launch directly into User Mode
cargo run -- --tech            # Launch directly into Technician Mode
cargo run -- gui               # Launch or focus the installed native GUI
cargo run -- update            # Run self-update action (preferred command form)
cargo run -- --update          # Run self-update action (legacy flag form)
cargo run -- install           # Deliberate preferred managed install
cargo run -- uninstall         # Uninstall through proven owner
cargo run -- snapshot --json   # Redacted noninteractive diagnostic snapshot
cargo run -- capabilities --json # Capability/provenance matrix
cargo run -- --help            # Show help with keybindings and sections
cargo clippy                   # Lint
cargo test                     # Run tests (assert_cmd/predicates available for CLI integration tests)
cd gui-engine && cargo test --locked # Engine-crate tests — NOT run by root cargo test (workspace-excluded)
cd gui && npm ci               # Restore the exact Native SDK dependency graph
cd gui && npm run check        # Native SDK strict model/manifest check
cd gui && npm test             # Native SDK/Zig tests
scripts/build-native-gui.ps1 -Target windows-x86_64 # Release-style GUI build
```

The binary is named `sd300` (not `sd-300`). The crates.io package name is `tr300-tui`; use `cargo install tr300-tui` for Cargo installs. The Rust library target is `sd_300`.

## CLI/TUI/GUI compatibility contract

- Bare `sd300` continues to open the User/Technician chooser. Preserve lifecycle
  commands, mode flags, nine section shortcuts, schema-1 JSON defaults, exit
  contracts and terminal restoration. The accepted v4 plan deliberately changes
  TUI layout, filtering, inspection, pause-view and collection scheduling; see
  ADRs 0006 and 0007. Neither frontend launches or shares a session with the other.
- `sd300 gui` is the only additive public launch command. It launches or focuses
  the installed app and reports a managed install/update repair instruction if
  the companion is absent.
- Managed/native install and update own CLI+GUI as one composite product but
  never launch the app, with one deliberate v3.1.0 exception: the GUI's own
  "Update now" surfaces spawn the CLI coordinator (`update --json
  --relaunch-gui`), and only that hidden flag plus a successful transaction
  relaunches the monitor through the idempotent singleton Open route.
  Installs, ordinary terminal updates, and failed updates still never launch
  the app. Proven-owner uninstall removes both frontends and their
  owned integrations/data without deleting ambiguous paths or user exports.
- Existing Cargo-owned v2 users intentionally update twice: Cargo installs the
  v3 CLI first, then the second same-version update performs the transactional
  managed CLI+GUI takeover. This is the sole ownership exception.
- GUI/TUI feature parity is a release invariant. New collectors, fields,
  warning semantics, observation states, capabilities, provenance, redaction,
  and configurable features must be wired to both applicable frontends in the
  same change, or explicitly documented as platform/frontend unavailable.
- Keep settings namespaces separate. Only deliberately shared persistent
  settings belong under `shared`; window, navigation, GUI mode/unit, chart,
  tray, startup, close, and motion choices belong under `gui` and must never
  change TUI session defaults.
- `docs/adr/` records the decisions behind this contract; ADR 0005 specifies
  the in-app update coordinator (GUI intent → engine ABI → detached CLI
  spawn from proven absolute paths → owner-preserving transaction → gated
  relaunch) and ADR 0004 the release-scope two-bar model. Read the relevant
  ADR before reworking these areas; supersede rather than silently diverge.

## Dual-frontend editing model

SD-300 is one product with two frontends over one collector core. The governing
product model (operator, 2026-07-22): the TUI and the app surface the same
information and serve the same function. They are installed together, updated
together, and uninstalled together, but once installed each is usable
independently. They expose all the same functions with one exception —
uninstall is CLI-only. Treat any divergence in what the two frontends can
observe as a defect unless it is deliberately documented as platform- or
frontend-unavailable.

The single source of truth is the `sd_300` Rust library. The TUI links it
in-process; the GUI loads it as a separate engine dynamic library. Feature work
changes the shared core once and wires the result into both frontends in the
same change.

### What the frontends share

Both consume the same Rust collector truth; neither reinterprets it.

- **Collectors and `SystemSnapshot`.** Every collector under `src/collectors/`
  returns a typed struct; `SystemSnapshot` (`src/collectors/mod.rs`) owns them
  and exposes the `refresh_*` methods. `SystemSnapshot` is not Clone (it owns
  `sysinfo::System`). The GUI engine reuses these exact structs and refresh
  methods rather than maintaining a parallel collector.
- **Warning sources and deduplication.** Warnings carry a `source` and are
  cleared per-source before re-collecting (`warnings.retain(|w| w.source !=
  "Name")`, then extend). The engine's collect loop (`gui-engine/src/lib.rs`)
  follows the identical pattern so warning identity and counts match the TUI.
- **Observation states.** `src/observation.rs` defines `ObservationStatus`
  (`Available`, `Unavailable`, `Unsupported`, `PermissionDenied`, `Error`,
  `Contradictory`). Both frontends render these states rather than inventing
  "missing" or zero placeholders.
- **Capability and provenance model.** `report::capabilities_for(&snapshot)`
  produces the capability records, and per-topic provenance strings live on the
  engine's `Topic::provenance()`. Both frontends present capability and
  provenance, not raw guesses.
- **Redaction.** `report::DiagnosticReport::from_snapshot(snapshot,
  include_sensitive)` and its `redact()` own the redaction rules and
  `redacted_fields`. The CLI `snapshot`/`capabilities` exports and the GUI
  engine's export requests both go through this one path.
- **Product version.** `env!("CARGO_PKG_VERSION")` is stamped into engine
  metadata and every topic envelope; `npm --prefix gui run
  check:product-version` reconciles the crate, engine, npm, and Zig manifests to
  one version.

### What they do not share

Processes, runtimes, mutable state, schedulers, and settings namespaces are
deliberately separate.

- **Processes and runtimes.** `App::run()` draws first and polls input plus
  completed samples; it never performs slow collection on the render loop.
  Each frontend creates its own `Monitor` with one bounded latest-value lane
  per provider. Fast CPU/memory/process sampling owns its in-process state;
  native and helper-backed probes run in owned CLI worker processes. The GUI
  separately loads its bundle-relative engine library and uses a Rust thread
  with a Condvar wake. Cancellation joins workers before engine unload.
- **Mutable state and scheduling.** Neither frontend shares mutable state or a
  scheduler with the other. The engine publishes bounded, latest-only,
  versioned topic projections; the GUI consumes sequence changes and keeps its
  own bounded histories. There is no shared buffer to mutate across the two.
- **Settings namespaces.** `src/settings.rs` separates `shared`, `tui`, and
  `gui`. Verified ND-300 and SMART helper paths belong under `shared`. Terminal
  mouse, ASCII, color and motion preferences belong under `tui`. GUI mode,
  geometry, navigation, unit, chart, tray, close and startup preferences never
  change terminal defaults. Updates preserve the namespaces they do not own.

### Recipe: adding a data field or feature to both frontends

Wire the change end to end, in this order.

1. **Collector struct (`src/collectors/X.rs`).** Add the field to the
   collector's typed struct and populate it in `collect()`. Represent absence
   with an `Observation` or a warning (using the per-source dedup pattern),
   never a silent zero.
2. **TUI presentation and render.** Prepare both audience modes in
   `src/presentation.rs`, including source, units, availability and inspection.
   `src/ui/dashboard.rs` renders visible rows and prepared chart buckets using
   the shared `content_block`/`sub_block`/`COLOR_*` helpers. Keep filtering,
   sorting and chart preparation outside rendering.
3. **Engine projection / ABI (`gui-engine/src/lib.rs`).** If the field already
   sits inside a topic's `Serialize` projection struct (`FastProjection`,
   `SlowProjection`, …), it crosses automatically because those borrow the live
   `&snapshot`. A new fixed-layout summary field means extending the matching
   `#[repr(C)]` struct *and* its `size_of`/`align_of` assertion test. ABI rules:
   caller-owned buffers only (`copy_to_caller`); bounded, latest-only topics;
   and no Rust panic, allocation, reference, or borrowed buffer across the ABI —
   every `extern "C"` entry is `catch_unwind`-guarded and serializes to an owned
   buffer.
4. **GUI projection (`gui/src/projection.zig`).** Add the field to the matching
   `*Json` parse struct and to `Projection`, then set it inside the matching
   `apply*Json`. Fixed-capacity rules apply: text uses `canvas.TextBuffer(N)`;
   lists use a `max_*` cap with a fixed array, a saturating total count, and a
   `@min` clamp. No per-sample heap growth.
5. **Model and sampling (`gui/src/main.zig`).** The projection lives at
   `Model.detail`; add a `Model` accessor for any computed value the view needs.
   Confirm the active section's `sampleDetailedTopics`/`sampleTopic` path
   subscribes to the topic that now carries the field, and push into a bounded
   history if it is charted.
6. **Markup binding (`gui/src/app.native`).** Bind the value with `{accessor}`
   or `{detail.field}`. `native check --strict` comptime-validates every `Model`
   and `Msg` field against the view: a field the view does not bind must be added
   to the relevant `view_unbound` tuple (one on `Msg`, one on `Model`) or the
   strict check fails.
7. **Tests.** Add or extend Rust unit tests run by `cargo test --locked` —
   and remember gui-engine is workspace-excluded, so its tests (topic-envelope
   contract, metadata, worker-join) only run via `cd gui-engine && cargo test
   --locked`; the `#[repr(C)]` layout assertions live on the Zig side as
   comptime checks. Keep the GUI strict check green (`npm --prefix gui run
   check`) and exercise the native tests (`npm --prefix gui test`).
8. **Parity and changelog.** The same change must wire both frontends, or
   explicitly document the field as platform/frontend-unavailable — parity is a
   release invariant. Update `CHANGELOG.md` and `HUMAN_CHANGELOG.md` in lockstep.

### Testing quick reference

```bash
cargo test --locked                                                  # Root crate unit/integration (gui-engine NOT included)
cd gui-engine && cargo test --locked                                 # Engine crate: envelope/metadata/worker tests
npm --prefix gui run check                                           # Native SDK strict model/binding check
SD300_RENDER_BENCH=1 npm --prefix gui test -- -Doptimize=ReleaseFast # native tests + render benchmark
```

gui-engine is not clippy-gated in CI (root clippy is); its C-ABI surface
carries accepted `not_unsafe_ptr_arg_deref` findings — new `extern "C"`
entries follow the established guarded-pointer pattern (see ADR 0005 and
Backlog #hrd) rather than chasing that lint.

The Native SDK is a pinned, patched dependency. Never edit the staged SDK under
the npm or Zig caches directly; changes go through
`gui/patches/native-sdk-0.5.4-software-render.patch`, both build preparers, and
the hash pins in `gui/package-lock.json`, `gui/build.zig.zon`, and
`gui/toolchain-lock.json`, which move together in one reviewed update.

For GUI acceptance, Computer Use is encouraged to drive and verify the running
app, with one exception in this repository: it cannot see the Windows tray, so
tray interactions are verified by programmatic dispatch plus a manual operator
test rather than by Computer Use.

## Release Process (cargo-dist + crates.io)

The standard deploy path is a push to the repository default branch (`main`) with a new, unreleased `Cargo.toml` version. `.github/workflows/release.yml` is intentionally customized from cargo-dist output; do not overwrite it with a generated workflow unless you preserve the main-branch deployment gate, unpublished qualification draft, native matrices, and final crates.io/latest publish gate.

For product work, keep implementation and qualification on a `codex/` feature
branch first. Build and exercise the composite Windows MSI before merging to
`main`; a successful compile is not installer acceptance.

1. Bump version in `Cargo.toml`
2. Update `CHANGELOG.md` with new version entry
3. Update `README.md`, `gui/README.md`, `CODEX_PROJECT.md`, `AGENTS.md`, and `CLAUDE.md` for user-visible release/install/update workflow changes
4. Reconcile the root crate, GUI engine, npm package/lock, Zig manifests, staged templates, and package metadata to one product version; run `npm --prefix gui run check:product-version`
5. Run local verification: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked`, `cargo build --release --locked`, `cargo publish --dry-run --locked --allow-dirty`, cross-target `cargo check`, `dist plan`, `npm --prefix gui ci`, `npm --prefix gui run check`, and `npm --prefix gui test`
6. Build only through the target-pinning GUI wrappers and prove a clean-cache build, warmed/offline dependency build, GUI self-test, distribution-lock check, and developer-path/debug-symbol leakage scan
7. Qualify an unpublished draft, including v2.0.6 compatibility, all owner/update/repair/rollback/uninstall routes, all six CLI+GUI targets, and the complete performance matrix
8. Generate checksums and SPDX SBOMs, attest every release subject, and publish the crate/release only after every candidate gate succeeds
9. Verify exact public bytes, attestations, stable routers, install/update/repair/uninstall, and application discovery before calling the release complete

On `main`, the release workflow reads the package name/version, checks crates.io, GitHub Releases, and tags, then:
- skips deployment if the exact version is already fully published everywhere
- repairs a crates.io-published/GitHub-release-missing state by rebuilding artifacts and finishing release hosting
- fails other partial-release states so a human can repair or bump forward
- runs cargo-dist artifact builds for all configured targets before hosting anything
- creates an unpublished `v{VERSION}` draft after all cargo-dist artifacts build
- renders stable managed wrappers, internal exact-tag cargo-dist installers, and immutable-client compatibility routers with SHA-256 sidecars
- builds and exercises Global/Corporate MSI and EXE installers on Windows, including synthetic-prior real self-updates, complete CLI uninstalls, and deliberate fresh takeovers into the managed PowerShell channel through all four native lanes
- builds a signed/notarized universal PKG and exercises managed-shell uninstall plus synthetic-prior same-PKG update/uninstall on native Intel and Apple Silicon runners
- builds the GUI and bundle-relative Rust engine for Windows x86_64, macOS
  x86_64/ARM64, Linux GNU x86_64/ARM64, and Linux musl x86_64; x86_64 is the
  Intel/AMD target, and Windows ARM64 is not part of this six-target contract
- qualifies the complete candidate lifecycle before publishing `tr300-tui` or
  changing the draft to `latest`; post-public checks verify the same public bytes
- creates SHA-256 sidecars, an SPDX SBOM, and GitHub attestations for the release
  subjects; `gh attestation verify <asset> -R QubeTX/qube-system-diagnostics`
  is the documented provenance check, not a substitute for platform code signing

Version tag pushes (`v*.*.*`) remain supported for explicit/manual releases, but the normal automation path is main-branch push. `CARGO_REGISTRY_TOKEN` must exist as a GitHub Actions secret; never commit registry tokens or publish from a local machine unless the user explicitly asks for an emergency manual publish after CI status has been checked.

The package was moved to `tr300-tui` so the project can publish while keeping the installed command and product identity as `sd300` / SD-300. After release, verify `cargo install tr300-tui --version {VERSION}` installs `sd300` and that the GitHub Release assets are present.

cargo-dist builds for 6 targets (x86_64/aarch64 across Windows/macOS/Linux) and produces `tr300-tui-*` archives containing the `sd300` binary. Fresh installs advertise stable `sd300-cli-installer.ps1` / `sd300-cli-installer.sh`; native options are stable Global/Corporate MSI/EXE names and `sd300-macos-universal.pkg`. `tr300-tui-installer.*` and uppercase `SD300-installer.*` remain compatibility routers for immutable 1.4.x clients. The `-cli-` segment prevents GitHub's case-equivalent asset collision with the uppercase bridge. Updater internals resolve the latest tag once and use exact-tag URLs plus SHA-256 sidecars. `allow-dirty = ["ci", "msi"]` is set because CI and installer naming are deliberately customized.

The GUI dependency graph is distribution-locked by `gui/toolchain-lock.json`,
`gui/package-lock.json`, and `gui/build.zig.zon`: `@native-sdk/cli` is exactly
0.5.4 and Zig is exactly 0.16.0, with immutable URLs, npm integrity/content
hashes, per-host Zig SHA-256 values, and the reviewed renderer patch. Do not
introduce `.path` dependencies, profile/global npm paths, local SDK checkouts,
unpinned branches, or a requirement for customer-side compilation.

## Architecture

### Data Flow

```text
main.rs → App::run(): immediate chooser, input, completed samples, dirty redraw
                       ↕ bounded latest values
                    Monitor (independent for TUI and GUI)
    Fast / Activity: 1s; Connections: 3s; Slow: 5s
    Diagnostics: 15s; Health: 60s; Static / Drivers: 300s
                       ↕ owned, bounded CLI worker protocol
                    native/helper-backed providers
```

The fast lane owns its `sysinfo::System`; presentation snapshots contain only
completed typed readings and actual capture metadata. GUI profiles subscribe to
required lanes; hidden mode retains summaries and slower thermal sampling.
Static, driver and health workers serialize their transient memory peaks while
live lanes remain independent. Retry/resume invalidates counter baselines.
Neither frontend shares mutable state or a scheduler with the other. Histories
advance only for distinct captured samples and retain missing intervals.

### Rendering Pipeline

```text
Presentation::prepare(app)          # sort/filter/inspect and chart preparation
ui::render(frame, app)
  → mode_select                    # until an audience is chosen
  → header_bar + dashboard         # adaptive 80×24 or wider layout
  → bottom_bar + optional overlays # keyboard, findings, consent, help
```

User and Technician modes consume the same central findings and observations.
Technician inspection adds source, units, identity, age and provider failures.
The minimum supported terminal is 80×24; smaller sizes display a resize notice.

### Module Layout

- **`app.rs`** — Input, dirty redraw, latest-sample polling, pause-view and explicit actions.
- **`monitor.rs`** — Independent provider lanes, deadlines, backoff and bounded delivery.
- **`presentation.rs`** — Prepared rows, stable selection, filtering and inspector content.
- **`collectors/`** — Each collector returns a typed data struct. `SystemSnapshot` owns all of them and has refresh methods that delegate to individual collectors.
- **`collectors/drivers/platform/`** — Platform-dispatched driver scanning: Windows uses Setup API (`SetupDi*`), Linux uses sysfs, macOS uses IOKit. Selected at compile time via `#[cfg(target_os)]`.
- **`collectors/thermals.rs`** — Cross-platform component sensors plus Windows Libre/Open Hardware Monitor WMI bridges, guarded read-only Dell AWCC enumeration, ACPI fallback, and independent GPU-temperature merging. Dell control methods must never be called by the collector.
- **`ui/common.rs`** — Color palette, `content_block()`/`sub_block()` panel helpers, `gauge_bar()`, `format_bytes()`, sparkline bar sets. All UI constants (colors, sparkline colors) are defined here.
- **`ui/dashboard.rs`** — Adaptive dashboard for all nine sections and both audience modes.
- **`types.rs`** — Core enums: `DiagnosticMode`, `Section` (1-9), `HealthStatus`, `ProcessSortKey`, `TempUnit`, `DeviceCategory` (9 variants), `DriverScanStatus` (4 variants).
- **`history.rs`** — Bounded timestamped optional samples and truthful gap buckets.
- **`gui-engine/`** — `cdylib` C ABI over the shared Rust collectors; no Rust
  panic, allocation, reference, or borrowed buffer may cross the ABI.
- **`gui/src/main.zig`** — Native GUI `Model`, tagged `Msg`, update effects,
  engine bridge, settings, and bounded view histories.
- **`gui/src/app.native`** — Declarative Native SDK view hierarchy and bindings.
- **`gui/src/fonts/`** — embedded Makira, Gail Rock and IBM Plex Mono font binaries;
  license notices and retained evidence live under `gui/assets/fonts/`.

### Platform Patterns

Windows-only deps (`wmi`, `serde`, `winapi`, `windows` crate) are gated under `[target.'cfg(windows)'.dependencies]`. Unix-only deps (`libc`, `nix`) under `[target.'cfg(unix)'.dependencies]`.

In source code, use `#[cfg(target_os = "windows")]` / `#[cfg(target_os = "linux")]` / `#[cfg(target_os = "macos")]` for platform-specific blocks. The driver scanning module (`collectors/drivers/platform/mod.rs`) is the primary example of this dispatch pattern.

### Key Constraints

- **`SystemSnapshot` is not Clone** — it owns `sysinfo::System` which has no Clone impl. Don't try to derive Clone on types containing it.
- **Bounded driver scanning** — Setup API/sysfs/IOKit discovery runs in the isolated Drivers lane. Never invoke `drivers::collect()` on the input/render loop or place an uninterruptible native call in a thread that prevents engine unload.
- **Warning deduplication** — Warnings are cleared per-source before re-collecting: `warnings.retain(|w| w.source != "SourceName")`. Always follow this pattern when adding new warning sources.
- **UI helpers** — Use `content_block(title)` for outer section panels and `sub_block(title)` for nested subsections. Use the existing `COLOR_*` and `SPARK_*` constants from `common.rs` — don't hardcode RGB values.
- **Sparkline rendering** — Windows uses `THREE_LEVELS` bar set, Unix uses `NINE_LEVELS`. The `sparkline_bar_set()` function handles this automatically.
- **build.rs** — Generates a man page (`sd300.1`) via `clap_mangen` at build time. It includes `src/cli.rs` via `#[path]` attribute, so the Cli struct must remain compatible with both the main binary and the build script.
- **One-second data, bounded presentation** — preserve live collector cadence
  while consuming sequence changes/latest-only projections. A visible GUI must
  present fast-topic samples at least once per second after renderer
  optimization; hidden/tray mode may coalesce to its required summaries. Never
  create an unbounded renderer queue or hide a collector regression by lowering
  data fidelity.
- **Tray/startup lifecycle** — tray and launch-at-login are independent. The
  GUI tray and close-to-tray behavior default on; launch-at-login defaults off.
  The TUI never creates a tray. Windows/macOS keep the GUI process alive after X
  only when a tray exists and close-to-tray is enabled; Linux has no tray under
  Native SDK 0.5.4, closes normally, and must never autostart hidden.
- **Performance is release-blocking** — qualify 15-minute foreground,
  30-minute hidden, and two-hour soak runs. Budgets are <=2% of one logical core
  foreground, <=1% hidden, <=150 MiB working set/RSS, <=300 MiB private
  memory/commit, <=16.7 ms frame p95, and <=50 ms input p95 outside scans, with
  no ordinary refresh stall over 100 ms or unbounded growth.

### Visual systems

The Ratatui TUI retains its warm earth palette and existing helpers unchanged.
The native GUI uses the Warm Carbon identity: near-black/charcoal surfaces,
controlled orange/amber status energy, restrained gradients, and a subtle
opacity-faded grid rather than generic purple “AI” styling. Makira serves
headings and major numerals, Gail Rock serves body copy, navigation and controls,
and IBM Plex Mono serves technical labels and compact values. Do not silently substitute or redistribute
fonts without preserving the applicable embedding-license evidence.


V4 optional setup contract: `sd300 tools nd300` is read-only; installation requires
`--install --accept` or the separate frontend confirmation. Official archives are
pinned for all six targets. Preserve existing ND-300 owners, leave its standalone
directory outside SD-300 uninstall ownership, and never infer diagnostic or M-Lab
consent from setup. Shared provider paths survive older GUI settings writes.
