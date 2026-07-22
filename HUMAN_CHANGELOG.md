# Human Changelog

A plain-English companion to [CHANGELOG.md](./CHANGELOG.md). It explains what changed and why without requiring release-engineering or code knowledge.

The newest section is work in progress. It is deliberately candid about what has passed and what still blocks release.

---

## Current work — native desktop companion (not released)

**Added**

- Added a native desktop app alongside the existing terminal interface. The normal terminal launch and its established controls remain unchanged.
- Added an explicit desktop launch command that opens the installed app or focuses the copy that is already running.
- Added a separate monitoring engine for the desktop app, so the graphical and terminal interfaces do not share mutable live state.
- Added complete User and Technician views for all nine diagnostic areas, with clear source labels and honest unavailable, unsupported, and permission-denied states.
- Added bounded history charts, process search and sorting, network and driver filters, connection paging, disk reliability details, driver-service details, and direct navigation from a warning to its explanation.
- Added safe background driver rescans, redacted report exports, interaction help, remembered navigation, and per-page data collection so hidden information is not needlessly refreshed.
- Added a proper app icon — a dark badge with an amber pulse line — so the app is recognizable in the taskbar, system tray, window corner, and installed-programs list instead of showing a blank default icon.
- Added app settings for theme, units, reduced motion, close behavior, tray behavior, startup, and refresh choices without changing terminal defaults.
- Added tray and launch-at-login support on Windows and macOS. Linux closes normally because the current desktop toolkit does not provide a Linux tray.
- Added single-instance behavior, graceful close requests, and safe installer/update handoff so a second launch focuses the existing app.
- Added pinned build tools and repeatable build wrappers for all supported Windows, macOS, and Linux targets.
- Added complete private Linux desktop-runtime packaging with architecture, dependency, license, checksum, and software-inventory validation on clean hosts.
- Added frozen compatibility examples from the last public terminal release so desktop work cannot silently change commands, help, redaction, output, or terminal behavior.
- Added installer, lifecycle, dependency, product-version, path-leak, performance, and compatibility checks for the combined product.

**Improved**

- Reworked the desktop renderer to avoid blank or partial frames and to redraw only the areas that actually changed.
- Kept once-per-second visible monitoring while removing repeated full process snapshots, redundant process scans, repeated timer setup, and avoidable memory copying.
- Turned off extremely expensive event tracing in release builds while retaining crash reports, self-tests, and opt-in diagnostic traces.
- Made settings recover safely from damaged files, kept exported reports through uninstall, and kept startup, tray, window, and terminal preferences separate.
- Made Windows takeover from a Cargo installation reversible, including restoration of the exact prior package records and binary if companion setup fails.
- Hardened downloaded-package extraction against unsafe links, special files, duplicate destinations, path traversal, mismatched inventories, and concurrent path changes.
- Expanded release qualification to use exact prior public installers and archives, synthetic older installs, repair paths, rollback injection, and branch-only runs that cannot publish.

**Fixed**

- Prevented Windows data collectors from flashing console windows behind the desktop app.
- Fixed blank or incomplete desktop frames caused by renderer limits, duplicated platform code, stale redraw regions, and incomplete text coverage.
- Removed several sources of high foreground processor use in rendering, process updates, presentation, tracing, and timer handling.
- Fixed a false one-frame process-usage spike after a failed Windows process scan.
- Fixed disagreements among tray, startup, single-instance, hidden-start, and close settings.
- Fixed several real installer defects involving oversized setup data, maintenance repair, upgrade properties, Cargo cleanup ordering, legal notices, and self-tests run without a console.
- Fixed a Windows installer-uninstall action that was unavailable in uninstall mode while preserving the correct user identity during install and update.
- Treated successful Windows installs that require a restart as committed success, preventing an unsafe rollback after Windows Installer had already committed.
- Made interrupted Windows cleanup retry owned shortcuts, app registrations, search entries, path entries, and receipts even when the main app folder is already gone.
- Fixed an uninstall failure that could occur when the program's small settings folder also held a person's own unrelated files. Uninstall now removes only its own records, leaves everything else byte-for-byte untouched, and removes the folder itself only when it is completely empty. Our automated checks now prove an unrelated file placed next to the program's records survives uninstall exactly as it was.
- Fixed a follow-on quirk where that same careful cleanup could report a failure even though it had done everything right, because of how an older built-in Windows component signals success. Real problems still stop the process and restore the previous state.
- Fixed updates and downloads failing on ordinary Windows computers that only have the built-in Windows automation shell and not the newer optional one. The program now finds the built-in shell directly in its trusted system location, which also protects against a class of tampering.
- Fixed updates failing for people who start them from the newer optional Windows shell: settings inherited from it could confuse the built-in shell the updater relies on. The updater now gives that built-in shell a clean start every time.
- Fixed the program leaving behind one small empty bookkeeping folder after certain installation transitions, which could also prevent its settings folder from being fully removed at uninstall. Bookkeeping folders are now tidied up when their work is done, and anything unrelated in them is always left alone.
- Fixed the desktop app becoming very laggy to scroll after being open for a minute or so. Fast mouse-wheel movement could pile up faster than the app could redraw, and once the live charts filled with a full minute of history the pile-up became noticeable. Wheel movement is now batched so scrolling stays smooth no matter how long the app has been open.
- Fixed a confusing situation where turning the tray icon on or off in settings and then closing the window could leave the app running invisibly with no way to bring it back (or quit it even though its tray icon was still there). Closing now behaves according to how the app actually started, matching the "restart required" note in settings.
- Fixed the app showing up to half a minute of outdated numbers right after being restored from the taskbar. A minimized window now keeps collecting every second, so it is always current the moment it reappears.
- Fixed Linux packaging across merged system folders, private desktop libraries, musl linking, container trust, dependency discovery, license evidence, and clean-host startup.
- Fixed macOS hosted builds involving system security headers, duplicated platform sources, stable engine identity, and developer-path leakage.
- Gave one intentionally large automated output test enough time on slow hosted Windows machines without relaxing normal collector deadlines.
- Replaced an unbounded workflow-lint command that could leave hung processes with bounded parsing and separate shell checks.

**Removed**

- Removed a broken automatic code-review robot from the project's checks. It had stopped working at a level outside our control and only produced a permanent red X without ever reviewing anything. Reviews still happen; they are just done directly now.

**Security**

- Removed the commercial primary font file from public source history and reconstructs it only on trusted build machines after an exact checksum check. This protects the file but does not prove permission to embed it in the app.
- Requires the source revision, product release, tag, package manifest, checksums, software inventory, and build attestations to agree before anything can be promoted.
- Requires exact ownership, publisher, installation scope, package kind, paths, and trusted system tools before Windows cleanup or takeover can change a machine.
- Preserves unrelated Cargo packages, user-exported reports, ambiguous folders, and nonempty shared state during takeover, repair, rollback, and uninstall.

**Qualified so far**

- Automated checks pass the Rust application and native desktop builds on all six supported target combinations.
- Physical Alienware testing passed every diagnostic destination in both audience modes, keyboard navigation, maximized display scaling, redacted export, single-instance focus, hidden startup, repeated tray close, startup registration, normal exit, and adjacent-engine self-test.
- A physical Corporate installer passed deliberate post-Cargo failure rollback, takeover from the last public Cargo install, missing-engine repair, terminal and desktop checks, supported uninstall, export preservation, and exact restoration of the original Cargo install.
- Foreground and hidden performance samples met the current average processor and memory budgets.
- Local Rust, command-compatibility, package, dependency, path-leak, and optimized desktop tests pass.

**Still open**

- Scrolling and input can become severely laggy on scrollable pages after the app has been open for about a minute. Average processor and memory results do not clear this release blocker.
- The first long endurance test ended early because a person closed the app window during the run — with the tray option off, closing the window intentionally exits the app. This was confirmed not to be a defect. The full endurance test now runs after release, on the published version, with the computer left untouched.
- Hosted Windows installer testing still catches an interactive prompt while removing a nonempty receipt folder. A conservative empty-folder-only correction and sibling-preservation test exist locally but are not yet qualified.
- Windows and Linux screen readers currently see a named application canvas rather than its internal controls. The terminal interface remains the documented accessible fallback.
- Font embedding rights, signed and notarized final packages, attestations, immutable publication, public-byte verification, website verification, and final physical acceptance remain incomplete.

**Release standard**

- The terminal behavior and all ownership-preserving install, update, repair, and uninstall routes must remain compatible on every supported platform.
- Foreground, tray, and long-running tests must meet the established processor, memory, frame-response, input-response, stall, and growth limits.
- Checksums, a software inventory, build provenance, exact released assets, and fresh public-download checks are required before this work can be called released.

---

## July 19, 2026 — Windows qualification correction

**Fixed**

- Corrected a Windows release check so a deliberately removed installer record counts as successful cleanup instead of an error. The failed candidate stayed unpublished and was not rewritten.

## July 19, 2026 — safe Windows takeover

**Fixed**

- Added a reversible handoff that lets an intentional managed reinstall replace a currently running Windows copy without colliding with the open program file.
- Added a tightly checked elevated helper for moving Global installer owners into the managed channel, with exact-release pinning, ownership rechecks, rollback, and bounded cleanup.
- Expanded hosted Windows qualification so every native installer channel must transfer ownership, remove its old registration and owned files, prove the new managed install, and uninstall cleanly.

## July 19, 2026 — driver and thermal accuracy

**Fixed**

- Stopped generic Windows status text from creating false driver warnings when the authoritative device manager reports no problem.
- Made driver warnings consistent across the overview, reports, and driver page, including genuine issues in less common device categories.
- Kept real graphics-card temperatures visible when processor sensors are missing and separated the availability of processor, graphics, combined-temperature, and fan readings.
- Added read-only temperature and fan readings from common Windows hardware monitors and guarded Dell firmware discovery. The app reports when administrator permission is required and never changes cooling controls.
- Added source and sensor-type labels to reports and Technician thermal details.

## July 19, 2026 — complete uninstall

**Fixed**

- Added a reversible Windows uninstall handoff so removing a running native install does not terminate the command before it can report the result.
- Removed empty product receipt folders after managed and Mac package uninstall while preserving shared Rust tools and nonempty shared folders.
- Made Windows uninstall call the trusted system installer directly instead of trusting the executable search path.
- Expanded release checks so the command-line uninstaller must completely remove every supported Windows, Mac, and Linux owner, including registrations, receipts, path entries, markers, and owned files.

## July 19, 2026 — managed receipt identity

**Fixed**

- Corrected the generated managed-install record to use the product identity, allowing real shell and PowerShell installations to be recognized and updated safely.
- Added release checks for the exact receipt identity and location in both managed installer formats.

## July 19, 2026 — updater reliability

**Fixed**

- Read updater output and errors at the same time so a large release response cannot fill a system pipe and deadlock the update check.
- Added repeatable post-publication artifact and lifecycle checks that do not republish an already released package.

## July 18, 2026 — cross-platform lifecycle and diagnostics

**Added**

- Added deliberate install and uninstall commands plus privacy-redacted snapshots and capability reports that return one predictable result for automation.
- Added consistent recovery guidance and a clear indication when a person must take action after any install, update, or uninstall attempt.
- Added ownership detection and same-channel updates for managed scripts, Cargo, both Windows installer scopes and formats, and the signed universal Mac package.
- Added stable public installer names, checksums, and compatibility routes so older installed copies can still find the right update without losing ownership history.
- Added authoritative fresh-install takeover with bounded cleanup, rollback, downgrade support, and refusal when a conflicting Windows install scope makes mutation unsafe.
- Added separate per-machine and per-user Windows installers, a direct signed/notarized universal Mac package, and draft qualification on every supported operating system before public promotion.
- Prevented Cargo publication until the complete native package and test matrix succeeds.
- Required structured Cargo ownership records, the exact package and command, and the matching installed release instead of trusting a familiar file path.
- Tightened managed-receipt checks so nested lookalike information cannot be mistaken for ownership proof.
- Added a Windows handoff and elevated same-channel helper so installer restart handling cannot kill the updater's final answer or strand the old executable.
- Resolves overlapping Cargo and managed ownership using the newer trustworthy record and refuses to guess when timestamps tie.
- Added hosted transitions from one candidate to the next for managed Windows, all native Windows channels, both Mac architectures, and Linux shell/Cargo owners.
- Made managed installer launchers verify the exact downloaded installer before running it.
- Added Windows memory-module, multiple-graphics-card, display, physical-disk reliability, battery, hardware identity, network-link, and driver-status diagnostics.
- Added clear source and availability states so missing, unsupported, denied, contradictory, or failed readings are never shown as fake zeroes.
- Added a privacy-sanitized Mac monitoring report grounded mainly in live Apple Silicon testing.
- Documented which Mac temperature, fan, energy, frequency, graphics, battery, storage, network, display, and device readings worked without elevated permission.
- Added implementation guidance for safe Mac interfaces, ownership, collection timing, availability, redaction, fixtures, and later fleet qualification.
- Documented useful newer Mac storage-health details while clearly marking that private behavior may change between system releases.
- Added sanitized examples and typed parsing guidance without retaining stable account, network, device, or machine identifiers.
- Added a reasoning record that separates proof from one physical Mac, public platform contracts, private model-specific behavior, and assumptions that still need wider testing.

**Improved**

- Made managed PowerShell the recommended Windows installation and managed shell the recommended Mac/Linux installation while keeping native installers available.
- Made a deliberate fresh official install the latest user intent regardless of installed release; raw Cargo installation remains an advanced unmanaged route.
- Corrected documentation to describe macOS as supported with known monitoring gaps instead of claiming complete hardware coverage.
- Recorded a native desktop toolkit as an experiment while keeping the Rust command line and terminal interface canonical.

## May 11, 2026 — final package-name correction

**Improved**

- Changed the public Cargo package name while keeping the product name and installed command unchanged.
- Updated Cargo-based updates and fallback installer links to use the publishable package identity without changing what users type to run the program.
- Clarified the standard user, technician, update, and legacy update commands after Cargo installation.
- Updated project and agent documentation to use the supported Cargo installation path.
- Kept the Windows installer branded as the product rather than the package name.
- Retained only legacy aliases that cannot collide during release upload.
- Moved forward with a fresh release rather than rewriting already attempted publication history.

## May 11, 2026 — lowercase package correction

**Improved**

- Switched the intended Cargo package metadata to a lowercase identity while keeping the installed command unchanged.
- Removed the accidental uppercase package from the release path and updated Cargo updates, installer names, documentation, and release behavior accordingly.
- Changed release ordering so Cargo publication happens only after artifacts build but before release hosting, reducing one kind of partial-publication failure.
- Added recovery for the case where Cargo publication succeeds but the matching hosted release is missing.

**Added**

- Kept legacy installer aliases in hosted releases so older installed updaters can still recover through their fallback path.

## May 11, 2026 — installation documentation correction

**Improved**

- Corrected Cargo installation instructions to match the package that had actually been published at the time while keeping the installed command lowercase.

## May 11, 2026 — first self-updating release

**Added**

- Added a normal update command while preserving the earlier update flag.
- Made updates run before the terminal interface starts so a failed update cannot leave the terminal display altered.
- Added hosted-release checks, sensible success and failure exits, semantic release comparison, and ordered Cargo, shell, and PowerShell update strategies.
- Added complete installation and update instructions for script, native installer, Cargo, and source-build routes.
- Added bounded subprocesses with timeouts and cancellation.
- Moved slow connectivity, disk-health, and driver work into the background so the terminal stays responsive.
- Added cross-platform automated format, lint, test, build, audit, target, packaging-plan, and release checks.
- Added release automation from the main branch while retaining explicit tag-driven releases.
- Added a pre-publication state check that skips complete releases and stops inconsistent partial-release states before publishing.
- Added the first Cargo publication path, later replaced by the corrected package identity and safer publication order.
- Added tests for update commands, strategy order, release comparison, command timeouts, network parsing, sockets, and Mac storage/system parsing.
- Added project context and agent instructions for installation, update, packaging, and release work.

**Improved**

- Established the package, command, library, and release naming used by the new publication system.
- Limited the Cargo package to required source, metadata, installer, license, and documentation files.
- Pinned the supported Rust toolchain and updated system-monitoring and terminal libraries.
- Removed an embedded web client from update checks and used bounded operating-system tools, reducing network and build complexity.
- Updated packaging names, prompts, installation folders, and paths for the corrected product identity.
- Made network speed use persistent measurements instead of rebuilding the network view every refresh.
- Added network operational state, better tables and tabs, reusable responsive panels, gauges, status rows, and scroll indicators.
- Made Mac audio and disk-health parsing use structured system output instead of fragile text scanning.

**Fixed**

- Prevented failed updates from leaving the terminal in a broken visual state.
- Preserved distinct Linux network states instead of treating every nonempty value as equivalent.
- Prevented slow or failed external diagnostics from freezing the app; they now time out and report unknown or unavailable.
- Made slow connectivity and disk-health refreshes complete in the background and update warnings afterward.
- Ensured Windows driver-scan resources are released even when a scan exits early.
- Bounded read-only Mac and Linux driver, disk, graphics, temperature, route, name-service, and socket probes.
- Made release metadata parsing work on the supported Ubuntu baseline without depending on a newer Python feature.
- Cleared strict lint failures in sorting, range handling, and keyboard-event code.

## March 12, 2026 — terminal usability

**Added**

- Added an automatically generated manual page and more complete help covering all controls and diagnostic areas.
- Added scrolling and position indicators to detailed driver and disk views.
- Added a consistent swap-history color and a reachable shortcut for sorting processes by memory.

**Improved**

- Made detailed disk panels match the rest of the interface and limited oversized network-interface lists with a clear remainder count.
- Standardized label spacing, chart colors, process-sort hints, and minimum-width behavior.

**Fixed**

- Fixed a keyboard conflict that made process sorting by memory impossible.

## March 12, 2026 — maintenance update

**Behind the scenes**

- Updated the release-packaging and hosted automation tools to include reliability and maintenance fixes. No user workflow changed.

## February 9, 2026 — Windows driver reliability

**Improved**

- Replaced fragile Windows management queries with the operating system's direct device, service, and registry interfaces for driver inventory, service status, release, and date information.
- Renamed failure messages so they describe the scan outcome rather than one retired implementation.

**Behind the scenes**

- Added the Windows system-library support required for direct device, registry, and service access.

## February 9, 2026 — interface overhaul

**Added**

- Added a header with the current audience mode and UTC time on every screen.
- Added consistent rounded panels, User-mode temperature history and fan speed, an animated driver-scan state, and clearer bottom navigation.

**Improved**

- Replaced the neon palette with a warm, readable earth-toned system across every page and mode.
- Standardized gauges, active navigation, mode selection, help, overview process count, and history-chart colors.

**Fixed**

- Moved slow Windows driver scanning off the main interface loop so opening or refreshing the driver page no longer freezes the terminal for several seconds.
- Improved permission guidance for driver-scan failures and removed obsolete internal imports.

## February 8, 2026 — controls and consistency

**Added**

- Added a universal quit shortcut, position indicators for long process and connection lists, connection scrolling help, and shared temperature thresholds.

**Improved**

- Consolidated repeated text and health-display behavior, named all refresh timings and history limits, and made temperature warnings consistent across overview, thermal, processor, User, and Technician views.

**Behind the scenes**

- Cleared a set of strict code-quality warnings without changing user behavior.

## February 7, 2026 — initial release

**Added**

- Launched with nine diagnostic sections in plain-language User mode and raw-data Technician mode.
- Added live processor, memory, disk, graphics, network, process, thermal, driver, and Windows disk-health monitoring.
- Supported Windows, macOS, and Linux on Intel/AMD and Arm computers.
