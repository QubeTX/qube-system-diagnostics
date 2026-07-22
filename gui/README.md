# SD-300 native GUI

This directory contains the additive Vercel Native SDK desktop monitor for
SD-300 v3. It is a native-rendered application: the view is declarative
`src/app.native`, behavior is a Zig `Model`/tagged `Msg`/`update` loop in
`src/main.zig`, and live diagnostics come from the bundle-relative Rust engine.
There is no application WebView or JavaScript runtime.

The v3 app is still in qualification. A successful local build or strict test
does not prove another target, native installer, physical interaction,
performance soak, or public release.

## Product contract

- This GUI is additive. Bare `sd300` keeps the existing User/Technician chooser
  and unchanged Ratatui flow; `sd300 gui` launches or focuses this app.
- Managed wrappers and native installers package the CLI/TUI, GUI, engine, and
  platform integrations together, but installation and update never open the
  app automatically.
- Proven-owner uninstall stops the app and removes the owned CLI+GUI product,
  integrations, startup state, and private app data while preserving ambiguous
  files and user-exported reports.
- Existing Cargo v2 users run `sd300 update` twice: the first update installs the
  v3 CLI through Cargo, then the second same-version update transfers to the
  complete managed CLI+GUI owner.
- GUI/TUI feature parity is release-blocking. Both frontends consume the same
  typed collectors, observations, warnings, capability states, provenance, and
  redaction. GUI work must not duplicate or subtly reinterpret collector truth.

## Runtime model

The GUI dynamically loads one target engine from the application bundle:

- Windows: `sd300_engine.dll`
- macOS: `libsd300_engine.dylib`
- Linux: `libsd300_engine.so`

The loader uses an absolute bundle-relative path, local symbol scope/restricted
Windows search flags, and rejects ABI, schema, product version, or target
mismatches before starting. The engine owns a dedicated Rust thread, Tokio
runtime, and non-cloneable `SystemSnapshot`; it never shares state with the TUI.
No Rust panic, allocation, reference, or borrowed buffer may cross the C ABI.

The engine preserves the 1/3/5/15/60-second collector cadences and exposes
bounded, latest-only versioned topics. Zig consumes sequence changes, keeps
bounded histories, and does not queue every missed render. A visible GUI must
present fast-topic samples at least once per second after renderer optimization;
hidden/tray mode may coalesce to its required summaries. Reducing collector
frequency or leaving a temporary slow visual cadence in place to conceal
renderer cost is not an acceptable optimization.

## Development commands

From the repository root on a native target host:

```powershell
npm --prefix gui ci --ignore-scripts
npm --prefix gui run check
npm --prefix gui test
scripts/build-native-gui.ps1 -Target windows-x86_64
```

`scripts/build-native-gui.ps1` accepts exactly:

- `windows-x86_64`
- `macos-x86_64`
- `macos-arm64`
- `linux-gnu-x86_64`
- `linux-gnu-arm64`
- `linux-musl-x86_64`

Each release target builds and tests on its native operating-system runner. The
x86_64 artifacts support Intel and AMD; Windows ARM64 is not one of the current
six release targets. Direct `native build` output is developer-only. Release
artifacts must use the target-pinning wrapper so the Rust host, Zig target,
baseline CPU, engine name, staged layout, and self-test contract are enforced.

## Reproducible dependency lock

The distribution graph is local to this repository and intentionally exact:

- `@native-sdk/cli` 0.5.4 from
  `https://registry.npmjs.org/@native-sdk/cli/-/cli-0.5.4.tgz`
- npm integrity
  `sha512-8ixE8TjN2zQ+9rnnpjOnmHDeloyvKBc9CKXVUdYxge63fSKn6AH3rodRcdE6EYQiAIDYzQiJSr8AKT1qdFcABA==`
- npm git head `349618a1385dedea361fbfe3b74ddba40ab6ee66`
- Zig dependency content hash
  `native_sdk-0.1.0-hzDzQo8l5gCK6W8hPyRC4voBqyQU8bhy6ktUDXKIqWlb`
- Zig 0.16.0 with per-host official archive SHA-256 values in
  `toolchain-lock.json`
- the reviewed `patches/native-sdk-0.5.4-software-render.patch`

`package-lock.json`, `build.zig.zon`, and `toolchain-lock.json` must move
together in one reviewed dependency update. Release builds must work from a
clean dependency cache and a warmed/offline cache without a global Native SDK.
Never commit a user-profile `.path` dependency, global npm path, local SDK
checkout, unpinned branch, temporary directory, or customer-side compiler
assumption.

The checked-in manifest deliberately resolves the pristine immutable tarball,
while SD-300 applies its separately hashed renderer patch to the project-local
npm copy. Run app tests through the repository-owned staging wrapper so the
Native SDK CLI sees the same relative patched graph used for releases:

```powershell
npm test -- -Doptimize=ReleaseFast
$env:SD300_RENDER_BENCH = "1"; npm test -- -Doptimize=ReleaseFast
```

Release-shaped builds also pass `-Dtrace=off`. Native SDK 0.5.4 defaults to
serializing every runtime event for development; SD-300 keeps panic capture,
engine self-testing, and explicit qualification telemetry without paying that
continuous foreground/tray cost. Diagnostic builds can opt back into SDK trace
levels explicitly.

The wrapper still executes `native test`; it never rewrites the checked-in
manifest or Zig's content-addressed package cache.

## Settings, tray, and startup

The settings document has separate `shared` and `gui` namespaces. The GUI owns
its remembered mode and unit, geometry, chart density, navigation, tray,
close behavior, launch-at-login, and reduced-motion settings. None of these may
change the next TUI launch, its chooser, or existing terminal defaults.

Tray and launch-at-login are independent and default off:

- Windows and macOS use one app process. When tray is enabled, close hides;
  Open restores; Quit terminates. Without tray, close exits.
- Linux has no tray under Native SDK 0.5.4. It uses its Linux manifest and exits
  on close. Launch-at-login must open visibly so no unreachable process remains.

## Accessibility support

The retained view declares semantic roles, names, focus order, text equivalents
for charts, reduced-motion behavior, and keyboard activation on every target.
Native SDK's deterministic automation snapshot exercises that semantic tree.

Native SDK 0.5.4 currently bridges retained canvas-widget semantics into the
operating-system accessibility tree on macOS only. Windows UI Automation and
Linux AT-SPI expose the named SD-300 canvas, but not its individual retained
controls. That is an explicit platform limitation, not a passed screen-reader
claim. The unchanged terminal UI remains available on Windows and Linux when
system screen-reader navigation is required. A future SDK upgrade must be
physically qualified before this limitation is removed.

## Visual identity and typography

The GUI uses Warm Carbon: deep black/charcoal surfaces, restrained orange and
amber energy, a subtle non-flat background gradient and fading grid, and
existing green/amber/red health semantics. Avoid generic purple-gradient “AI”
styling, blur-heavy cards, continuous ambient animation, custom cursors, and
other decorative work that costs legibility or idle performance.

Makira is the primary face for body copy, headings, navigation, and large
numbers. IBM Plex Mono is secondary for technical labels and compact numeric
data. The binaries are embedded from `src/fonts`; license notices/evidence live
under `assets/fonts`. IBM Plex Mono's OFL notice must ship. Do not publicly ship
Makira unless repository/release evidence confirms that the purchased license
permits desktop-application embedding and redistribution.

Makira's commercial source file is deliberately excluded from this public
repository. Local builds use the operator-provided ignored file. Trusted CI
reconstructs the exact reviewed bytes from the split encrypted
`SD300_MAKIRA_FONT_BROTLI_BASE64_PART_1` and
`SD300_MAKIRA_FONT_BROTLI_BASE64_PART_2` secrets, then verifies the digest in
`toolchain-lock.json` before compiling. Never print, upload, or attach either
secret or the standalone font file.

## Performance and release qualification

Release binaries must pass Native SDK strict checks, GUI `--self-test --json`,
path/debug-symbol leakage scans, target and engine mismatch tests, lifecycle
tests, and the full platform matrix. Required performance runs are 15 minutes
foreground, 30 minutes hidden/tray, and a two-hour soak, with these ceilings:

- foreground average CPU: 2% of one logical core
- hidden/tray average CPU: 1% of one logical core
- working set/RSS: 150 MiB
- private memory/commit: 300 MiB
- frame-time p95: 16.7 ms
- input-response p95: 50 ms outside explicit scans
- ordinary refresh stall: no event longer than 100 ms
- no unbounded memory, event, log, history, or renderer-work growth

Linux packages include a pinned private GTK runtime and must be exercised where
the host has no GTK packages installed. Windows packages must pass Global and
Corporate MSI/EXE fresh, update, repair, running-app, rollback, and uninstall
flows. The universal macOS package must pass native Intel and Apple Silicon
signing, notarization, stapling, update, repair, and uninstall flows.

Qualified release assets receive SHA-256 sidecars, SPDX SBOM coverage, and
GitHub provenance attestations. This documentation is a gate definition; it is
not a claim those gates have passed until their immutable evidence is recorded.

## Hot reload

`src/app.native` is embedded into the binary and watched during development.
Editing it while the app runs refreshes the view without intentionally losing
model state; a parse failure retains the last good view. Hot reload is a
developer convenience and is not part of packaged release behavior.

See [the distribution decision](../docs/thinking/2026-07-20-native-gui-distribution.md)
for why customer machines install centrally built artifacts rather than compile
the GUI during `sd300 update`.

See [the process model](../docs/thinking/2026-07-20-native-gui-process-model.md)
for CLI/TUI isolation, desktop/tray lifecycle, and no-console collection.

See [the information architecture](../docs/thinking/2026-07-21-native-gui-information-architecture.md)
for hierarchy and professional/new-user presentation decisions.
