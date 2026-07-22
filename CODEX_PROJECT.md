# SD-300 / SD300 Project Context

## TL;DR

SD-300 is a cross-platform diagnostics product with two first-class frontends:
the established Rust/Ratatui CLI/TUI and an additive Vercel Native SDK desktop
monitor. The binary is `sd300`; the crates.io package is `tr300-tui`; the GUI
loads a bundle-relative Rust `cdylib` that reuses the same collectors without
sharing the TUI process or event loop.

The v3.0.0 worktree is a qualification candidate, not a published release.
Release completion requires preserved v2.0.6 CLI/TUI behavior, the complete
composite installer/update/uninstall lifecycle, performance gates, and native or
hosted evidence for all six release targets. Do not describe local builds as
proof that the public release or another operating system has passed.

## Compatibility contract

- Bare `sd300` always opens the existing User/Technician chooser. Current TUI
  sections, keybindings, cadence, styling, sorting, scrolling, warnings,
  commands, flags, help, JSON, exit codes, and terminal behavior remain
  compatible.
- `sd300 gui` is additive. It launches or focuses an installed GUI and gives an
  install/update repair instruction when the companion is absent. Install and
  update never launch the app.
- Managed/native owners install CLI+GUI as one product and preserve their
  existing scope, path, PATH, receipt, owner, and update route. Proven-owner
  uninstall additionally removes the GUI, engine, integrations, startup entry,
  private data, staging, and rollback state while preserving ambiguous paths
  and user exports.
- Existing Cargo v2 users update twice. The first update uses Cargo to install
  the v3 CLI; the second same-version update performs the intentional
  transactional managed CLI+GUI takeover. Later operations use that managed
  owner. This is the only planned ownership exception.
- GUI/TUI feature parity is a release invariant. New hardware fields,
  collectors, availability states, warnings, capabilities, provenance,
  redaction, and shared configuration must reach both applicable frontends in
  the same product update.

## Current v3 candidate architecture

- Primary command paths:
  - `sd300` opens interactive mode selection.
  - `sd300 --user` and `sd300 --tech` open the existing TUI modes directly.
  - `sd300 gui` launches or focuses the native app.
  - `sd300 update` and legacy `sd300 --update` run before terminal setup.
  - `sd300 install` makes the preferred managed composite channel authoritative.
  - `sd300 uninstall` delegates to the proven owner for complete product cleanup.
  - `sd300 snapshot --json` and `sd300 capabilities --json` expose redacted
    automation contracts.
- The TUI retains its current non-cloneable `SystemSnapshot` and `tokio::select!`
  loop: fast refresh at 1 second, connections at 3 seconds, slow at 5 seconds,
  diagnostics at 15 seconds, disk health at 60 seconds, and asynchronous driver
  scanning.
- The GUI engine owns a separate `SystemSnapshot` and Tokio runtime on a
  dedicated Rust thread. It publishes versioned, latest-only projections for
  static, fast, medium, slow, diagnostics, health, drivers, warnings, and
  capabilities so renderer delay cannot create an unbounded collector backlog.
- The Native SDK app is declarative `app.native` plus Zig `Model`/tagged
  `Msg`/`update` logic. It contains no application WebView or JavaScript runtime.
  It loads `sd300_engine.dll`, `libsd300_engine.dylib`, or
  `libsd300_engine.so` only from an absolute bundle-relative path and rejects
  ABI, schema, version, product, or target mismatches before collection.
- Settings are a versioned document with `shared` and `gui` namespaces. GUI
  mode/unit, geometry, chart density, navigation, tray, close behavior,
  launch-at-login, and reduced motion cannot change TUI startup or session
  defaults.
- Tray and launch-at-login are independent and default off. Windows/macOS use
  one app process with Open/Quit and hide-on-close only when tray is enabled.
  Native SDK 0.5.4 has no suitable Linux tray, so Linux exits on close and never
  autostarts into an unreachable hidden state.
- Keyboard navigation, visible focus, text chart equivalents, reduced motion,
  and the deterministic semantic tree apply on every target. Native SDK 0.5.4
  bridges retained canvas controls into the system accessibility tree only on
  macOS; Windows and Linux expose the named canvas rather than its children, so
  the unchanged TUI is the documented system-screen-reader path there.

## Distribution and release gates

The exact six-target contract is:

| Platform | Rust target | Composite v3 requirement |
|----------|-------------|--------------------------|
| Windows x86_64 | `x86_64-pc-windows-msvc` | CLI/TUI, GUI, engine, managed wrapper, Global/Corporate MSI+EXE |
| macOS x86_64 | `x86_64-apple-darwin` | CLI/TUI and GUI in the signed/notarized universal PKG |
| macOS ARM64 | `aarch64-apple-darwin` | CLI/TUI and GUI in the signed/notarized universal PKG |
| Linux GNU x86_64 | `x86_64-unknown-linux-gnu` | CLI/TUI, GUI, private runtime, managed lifecycle |
| Linux GNU ARM64 | `aarch64-unknown-linux-gnu` | CLI/TUI, GUI, private runtime, managed lifecycle |
| Linux musl x86_64 | `x86_64-unknown-linux-musl` | CLI/TUI, GUI, private runtime, managed lifecycle |

x86_64 covers Intel and AMD. Windows ARM64 and Linux musl ARM64 are not part of
this established release matrix and must not be implied by shorthand such as
“all ARM platforms.”

Native SDK is locked to `@native-sdk/cli` 0.5.4 and Zig to 0.16.0 through
`gui/toolchain-lock.json`, `gui/package-lock.json`, and `gui/build.zig.zon`.
Those records pin immutable package URLs, npm integrity/content hashes, per-host
Zig SHA-256 values, and the reviewed renderer patch. Distribution builds must
work without a global Native SDK, local checkout, user-profile `.path`
dependency, or customer-side toolchain.

The unpublished draft must pass existing CLI/TUI golden and lifecycle
compatibility, GUI strict/self-tests, target/ABI checks, running-app handoff,
same-version repair, rollback, Cargo two-update migration, complete uninstall,
application discovery, Linux private-runtime isolation, path-leak scans, and
native matrices before either crates.io publication or `latest` promotion.
Implementation remains on a `codex/` feature branch until the composite Windows
MSI has been built and exercised; compiling its inputs alone is not sufficient.

Performance qualification uses release binaries in 15-minute foreground,
30-minute hidden, and two-hour soak runs. Budgets are at most 2% of one logical
core foreground, 1% hidden/tray, 150 MiB working set/RSS, 300 MiB private
memory/commit, 16.7 ms frame p95, and 50 ms input p95 outside explicit scans,
with no ordinary refresh stall over 100 ms or unbounded history/event/log/memory
growth. One-second data collection remains live, and a visible GUI must present
fast-topic samples at least once per second after renderer optimization;
hidden/tray mode may coalesce to its required summaries. Presentation consumes
bounded sequence changes instead of accumulating renderer work.

Every qualified release subject receives a SHA-256 sidecar, SPDX SBOM coverage,
and GitHub build/SBOM provenance. Public assets are verified with
`gh attestation verify <asset> -R QubeTX/qube-system-diagnostics` and against
their exact public digest. Attestation does not create a Windows “Verified
publisher” identity or remove SmartScreen risk.

## GUI product identity

The app uses the Warm Carbon direction: near-black and charcoal depth, restrained
orange/amber energy, a subtle background gradient and fading grid, and existing
green/amber/red status semantics. It deliberately avoids generic purple-gradient,
blur-heavy, cursor, and continuous-background effects.

Makira is the primary typeface for body text, headings, and prominent numerals.
IBM Plex Mono is the technical secondary face for compact labels and small
measurements. Both are bundled only with retained evidence that their licenses
permit application embedding; do not silently replace the typography or assume
that possession of a font file proves redistribution rights.

## Goals

- Preserve the mature CLI/TUI as an unchanged, high-quality terminal product.
- Make the app/tray feel like one predictable desktop utility while keeping it
  dormant unless explicitly launched or enabled at login.
- Prefer read-only, bounded platform probes that degrade to explicit
  unavailable/unsupported/permission-denied states instead of blocking,
  crashing, or manufacturing zero telemetry.
- Share collector and hardware-data improvements automatically across the TUI,
  GUI, snapshots, and capabilities exports.
- Keep dense professional detail available through sensible hierarchy and
  progressive disclosure without reducing the utility for new users.
- Make complete lifecycle, performance, supply-chain provenance, and exact
  public-byte evidence release requirements rather than post-release cleanup.

## Source map

```text
.
├── .github/workflows
│   ├── release.yml               # base cargo-dist draft and stable routers
│   ├── windows-installers.yml    # Windows GUI + MSI/EXE qualification
│   ├── macos-installer.yml       # universal app/PKG signing qualification
│   ├── linux-native-gui.yml      # GNU/musl GUI and private runtime builds
│   └── release-qualify.yml       # aggregate, SBOM, attest, publish gates
├── .tasks/                       # tracked milestone, tasks, and evidence
├── gui/
│   ├── src/app.native            # declarative native view
│   ├── src/main.zig              # model, messages, effects, histories
│   ├── assets/                   # icons, notices, retained license evidence
│   ├── src/fonts/                # embedded application font binaries
│   ├── platform/linux/           # Linux-specific app manifest
│   ├── patches/                  # reviewed pinned-SDK patches
│   └── toolchain-lock.json       # Native SDK/Zig distribution lock
├── gui-engine/                   # Rust cdylib and C ABI
├── scripts/                      # reproducible build/package/verification tools
├── src/
│   ├── app.rs                    # unchanged TUI state/event loop
│   ├── collectors/               # shared typed platform collectors
│   ├── gui.rs                    # CLI-to-GUI launch/focus and local IPC
│   ├── migrate.rs                # bounded ownership migration
│   ├── update.rs                 # proven-owner lifecycle
│   └── ui/                       # existing Ratatui rendering
├── tests/                        # CLI/TUI compatibility contracts
├── wix/ and wix-corporate/       # Windows native packaging
├── inno/                         # Windows EXE packaging
├── Cargo.toml and Cargo.lock
└── README.md, AGENTS.md, CLAUDE.md, CHANGELOG.md
```
