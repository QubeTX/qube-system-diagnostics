# Human Changelog

A plain-English companion to [CHANGELOG.md](./CHANGELOG.md). It explains what changed and why without requiring release-engineering or code knowledge.

The newest section is work in progress. It is deliberately candid about what has passed and what still blocks release.

---

## In progress — more trustworthy monitoring

- Behind the scenes: performance comparisons keep useful measurements when an older app crashes while closing, then continue testing its replacement. A failed older version remains clearly marked as failed and cannot make the new version pass.

- The desktop app now pairs Makira's headings and large readings with Gail Rock for navigation, controls and explanations. The fonts travel with the app, keeping the same appearance on every supported system, while dense technical values remain easy to scan.

- Quitting the Mac app now finishes its background monitoring cleanup before the operating system closes it. Behind the scenes, native checks verify that quitting leaves no collectors behind.

- Connection checks now say when their results are delayed or unavailable, even while other readings remain live. Clearer labels distinguish a network response from the time needed to look up a name, and long hardware explanations stay readable.

- Behind the scenes: Mac testing now explains which background work remains after the app closes and preserves performance traces even when another check fails, without recording private command lines.

- The desktop monitor puts useful readings first, makes its two viewing modes easier to reach, and keeps explanations and consent readable. Network scans and bandwidth tests have separate controls, and storage tools live with storage. Process rankings follow the values on screen, waiting states no longer look like idle readings, and graphics memory labels explain what is shared with the rest of the computer.

- Behind the scenes: project guidance now describes how the new monitor actually runs, and the task board separates completed features from the remaining release checks.

- Linux uses a lighter drawing path by default and closes through the same cleanup as its window button. Explicit rendering choices remain respected. Behind the scenes, native Linux lifecycle checks and separate Mac performance traces investigate the remaining release blockers.

- Behind the scenes: minimal Linux testing now captures where the app fails during shutdown, without mixing debugger results into performance acceptance. Repeated close checks investigate failures that disappear on a single retry.

- The app avoids keeping duplicate pictures of simple solid panels and borders, and fixes uneven shadows at some scaled edges. Behind the scenes, rendering checks now follow the app's actual drawing path and compare every pixel across scaling, scrolling and translucent overlays.

- Behind the scenes: long test runs keep their original app build until they finish. Memory reports now identify growth by component, and rendering tests exercise changing process readings as well as scrolling.

- Behind the scenes: a protected network helper no longer interrupts Linux app testing when the monitor's own collectors can still be verified.

- Behind the scenes: Linux memory testing now separates graphics-library memory from app allocations and compares rendering paths without changing the app's defaults or recording private file paths.

- Mac process readings now distinguish an idle application from an unreadable one and stop carrying a previous busy reading into an idle interval. Both views explain when the process inventory cannot be read and recover when access returns.

- Behind the scenes: minimal Linux testing now installs and checks its display test tools before the long build, so missing test dependencies are reported promptly.

- Storage and temperature checks reuse hardware discovery work and avoid reading disk activity twice. Live readings still refresh as often, including when disks appear or disappear.

- A clock adjustment no longer makes an old reading look brand new. Both views explain when capture age is unknown, charts restart across the clock change, and richer exports preserve that distinction. Behind the scenes, installation checks recognize the updated app and engine together.

- Hidden Mac launches request background monitoring immediately. Linux window changes now reach the monitoring engine, including a prompt refresh when the window returns. Behind the scenes, platform checks report more precise causes when background operation or performance fails.

- Occasional hardware inventory checks take turns to reduce memory peaks while live monitoring continues. Behind the scenes, checks verify that waiting work can stop promptly and a failed check cannot block later ones.

- Behind the scenes: longer testing exposed a memory peak missed by short checks. Reports now identify which background checks overlap at that peak so improvements target the measured cause.

- Behind the scenes: before-and-after comparisons cover the app as well as the terminal, including background operation. Baseline downloads are verified and every result identifies the exact files measured.

- Behind the scenes: isolated Linux checks can verify the mounted source safely without changing the machine's global trust settings.

- Behind the scenes: performance reports explain when a protected helper limits inspection, while preserving the resource measurements that remain readable.

- The Linux app can reduce background work when its window is hidden or minimized. Behind the scenes, Mac and Linux checks verify the app's window state, background collectors and clean shutdown before accepting performance results.

- Behind the scenes: Mac and Linux can run the same before-and-after resource comparisons as Windows on their own systems. Checks include work performed by background helpers and preserve failed results for investigation.

- Behind the scenes: both views build their shared monitoring code with the same supporting libraries. Release checks catch accidental differences that could make the terminal and app report different results.

- Linux and Mac network monitoring distinguishes failed reads from idle traffic and keeps other readable interfaces useful. Totals explain their scope, missing counters stay visibly unavailable, and the detailed graphics view explains where utilization came from.

- Windows graphics readings can use a lighter native counter query and retain small changes that rounded readings missed. Newly detected or reset counters warm up before displaying a value; older provider routes remain available when needed.

- Behind the scenes: performance checks count background collectors as well as the app window, verify that the requested checks are running, and keep failed measurements visible. More detailed profiling helps target costly work without reducing monitoring frequency.

- Behind the scenes: noisy background-check tests now recognize either safety limit stopping the check, while still requiring prompt cleanup. This avoids treating a correctly enforced timeout as a product failure on Mac.

- Windows connection checks use the operating system's actual route and ping reply, with fewer background program launches. Readings below the provider's timing precision remain unavailable, and blocked pings still do not prove the internet is down.

- Device checks report what the operating system actually detected. A missing keyboard query, unplugged cable or optional service no longer becomes an invented healthy or failed device. Both views explain limited access and distinguish services that are running, idle, absent or unreadable.

- Failed network and installation checks explain whether a helper was missing, denied access, took too long or returned unusable text. Behind the scenes, callers receive distinct failures instead of an ambiguous empty result.

- Linux terminal navigation receives the same resize protection verified on Mac. Behind the scenes, longer resource checks remain release requirements even when shorter checks look healthy.

- Mac terminal input uses a polling path that keeps pending keystrokes visible during resizing. Behind the scenes, native checks exercise rapid resizing and navigation together.

- Windows network readings use less repeated discovery and keep similarly named adapters separate. The total explains which interfaces it includes, virtual connections remain inspectable, and unavailable speeds or addresses no longer look like idle traffic or a missing address.

- Behind the scenes: background-check tests explain the failure they encountered, making platform-specific fixes easier to verify.

- A failed refresh keeps a previously reported drive warning visible and explains how old its evidence is. Missing timestamps and clock changes remain explicit, so older results do not appear freshly measured.

- Behind the scenes: a failed terminal check now keeps enough context to investigate without saving private readings from the screen.

- Background readings pass directly to the monitor without writing temporary reports. Broken or stuck checks still stop cleanly, and a noisy helper cannot make cancellation unresponsive.

- Behind the scenes: real terminal checks now exercise navigation, search, resizing, paused views and clean exit on every supported operating system. Timing measurements help identify slow work, and failed performance checks stay visible in the release evidence.

- Process CPU readings keep the same meaning when Windows limits which processors the monitor can use. A busy thread counts as one processor, and missing readings or a reused process number start with a clear warmup.

- Windows monitoring uses less memory by avoiding unused console hosts for background checks. Readings keep their existing update frequency, and checks still stop cleanly when cancelled.

- NVIDIA monitoring can read the installed driver directly, reducing repeated helper launches. Graphics memory keeps its allocated-memory meaning, and unsupported or denied readings stay visibly separate from real zero values. Existing helper-based readings remain available as a fallback.

- Connection monitoring does less background work on Windows, recognizes more connection states on Linux, and includes UDP on Mac computers. Minimal Linux systems can still show endpoints without an extra tool. Missing ownership and failed or partial checks are clearly identified instead of looking like an empty, healthy result.

- Disk speeds stay accurate when a background response arrives late. Restarting a check or waking the computer shows a brief warmup instead of an artificial spike, and sample timing follows the actual readings.

- Background inventory and storage-health checks release their helper processes between readings, reducing idle memory without slowing updates.

- Windows background checks spend less time starting helper programs. Behind the scenes, performance measurements now include the whole monitoring process family, including helpers that have already exited.

- Desktop searches now reach every collected connection and device, with pages for longer lists and clear empty results. Missing process identities stay visibly unavailable. Searching older readings does not make them appear freshly measured.

- Save a private, redacted report of your terminal session, including completed optional diagnostics. Saving keeps monitoring responsive and preserves earlier reports; a paused view saves the readings you froze.
- Missing storage error readings stay visibly missing, and the desktop shows how many drives supplied each counter. An unfinished bandwidth test stays marked incomplete. Behind the scenes, optional downloads unpack with consistent settings.

- Optional administrator-authorized storage reads wait for their result correctly on Windows and Mac computers. Behind the scenes, native checks caught a platform difference that Linux-only testing would have missed.

- Review an optional administrator-authorized storage read before allowing it. Monitoring stays responsive, cancellation keeps earlier results, and private details stay out of redacted exports. Behind the scenes, synthetic checks exercise the privileged process without opening a real drive.

- Storage health stays attached to the correct drive, including systems with identical models. Conflicting checks remain visible, and a reassuring response no longer hides a fault reported by another check.

- Optional storage-health setup works on fresh Alpine Linux systems even when their package cache is empty.

- Behind the scenes: optional-tool checks now reveal why a Linux package download failed, so a fix can target the actual cause.

- Optional storage-health setup explains the platform-specific operation before asking to install. It keeps existing tools and leaves background services alone. Missing system libraries get a useful explanation, and successful setup refreshes ordinary storage readings.

- Optional network-tool setup explains the download and asks before installing. Existing installations are preserved, and the companion remains available if you remove the monitor. Changing desktop preferences keeps your chosen diagnostic tools.

- A failed optional scan can be cancelled or retried without taking down monitoring. Missing speed measurements are clearly marked, and privacy checks cover combined diagnostic exports.

- Run optional network checks from either interface while monitoring continues. Bandwidth tests show their budget and ask separately about publishing measurements through M-Lab. Cancelled or unfinished checks do not become confirmed network faults, and private report details stay out of redacted exports.
- Behind the scenes: failure tests now create predictable workloads so slow test-machine startup cannot be mistaken for broken monitoring.

- Find any running process from the app, including quieter processes beyond the first page, and browse the results without losing the live view.

**Fixed**

- Macs read storage activity directly from the operating system, avoiding a helper launch on every sample. Virtual disks are kept out of the physical-drive total so activity is not counted twice.

- Monitoring reuses its isolated workers instead of repeatedly starting them. Hardware discovery is reused when devices are unchanged, and unavailable optional sensors are retried less aggressively. New devices, resume and explicit retry refresh discovery.

- The terminal dashboard now fits smaller screens and uses extra space for charts and useful details. Search the full list, keep your place as applications update, and pause what you see while monitoring continues. Keyboard navigation remains complete; mouse support is optional.
- Both interfaces show gaps when readings are missing instead of drawing invented continuity. Processor and graphics temperatures stay separate, and desktop storage activity updates promptly. Terminal appearance preferences remain independent of the desktop app.

- Separate sensors keep their own identity even when they share a label. Linux reports readable temperatures and fan speeds while excluding readings the device marks invalid. Graphics temperatures stay separate from processor temperatures.

- Graphics readings stay attached to the correct device, even when two cards have the same name. Graphics monitoring now includes Linux driver readings and Mac graphics inventory. Shared memory and recommended allocation budgets are clearly distinguished from dedicated video memory.
- Windows graphics load reflects the busiest engine. A card that only exposes temperature no longer appears idle just because its utilization is missing.

- Linux now reports readable battery, display, adapter and computer details. Macs gain native display and battery discovery. Missing brightness or energy readings stay unavailable instead of being guessed from unrelated values.

- Detailed reports distinguish missing readings from measured zero and explain when observations were captured. Existing report consumers keep their familiar format unless they choose the richer one.
- Both interfaces preserve the identity of a running application and show when its processor or memory reading is unavailable. Windows can gather the complete application list more efficiently.
- Findings explain their evidence and suggest a next step. Busy processors are described as heavy demand; they are not treated as proof of broken hardware.

- Storage activity updates promptly, independently of slower health checks. Idle drives no longer show a made-up response time, and drive health is attached to the correct physical device.
- Mac storage discovery includes additional drives. Optional storage-health tools preserve useful partial readings and distinguish an unreadable device from a failing one.
- Exporting a diagnostic snapshot now has a finite collection wait, including when a hardware provider stops responding.
- The startup message appears only while a live monitor is warming up; it does not cover already available diagnostic information.
- The monitor opens before hardware discovery finishes and remains usable while slow checks run. Failed checks show their status and can recover without restarting the app.
- Charts no longer repeat an old temperature or graphics reading as if it were new. Both interfaces show when their readings were captured.
- Network traffic readings account for the time between measurements, and connection checks report the reply time rather than the time taken to start a helper.
- Process sorting can find memory-heavy applications even when they are using little processor time.
- A stuck or excessively noisy helper can no longer leave output-reader threads waiting indefinitely. Filtered connection checks no longer automatically label the internet offline.

**Behind the scenes**

- Installer checks recognize the updated internal interface and report the actual failing result, making platform-specific failures diagnosable.

- Run the shared monitoring tests on every supported kind of operating system and processor, including the lightweight Linux build.

- Strengthened interface checks so they validate the current screen bindings after native tests instead of relying on an older generated description.
- Set up tracked work and verification for the monitoring improvements. The complete update will ship together after testing.

---

## July 25, 2026 — a lighter, platform-aware app icon

**Improved**

- Removed the black square behind the SD/300 app mark on Windows and Linux.
  The orange-and-white cube now sits directly on the desktop surface instead
  of looking like artwork pasted onto a dark tile.
- Gave the Mac version its own treatment: the corners stay transparent, while
  a softened isometric graphite shape supports the same orange-and-white mark
  so it feels intentional in the Dock without becoming another square badge.
- Kept the monochrome tray symbol exactly as selected, including the system's
  ability to adapt it for light and dark menu bars.

**Behind the scenes**

- Tightened the repeatable icon check so a future change cannot accidentally
  bring back the dark plate, lose transparency, or erase the defining orange
  and light parts at small desktop sizes.
- Rebuilt the real Windows app and update archive, confirmed Windows extracts
  the chosen mark from the app itself, and confirmed the replacement icon
  files remain covered and damage-checked by the existing update and uninstall
  ownership rules.
- Fixed the final Mac installer check so it follows the chosen prebuilt icon's
  real filename instead of looking for the old automatically generated name.

---

## July 23, 2026 — SD-300 now looks like SD-300 and keeps monitoring

**Added**

- Replaced the generic heart-monitor-style artwork with the chosen isometric
  SD/300 block. The app now has a matching, simpler tray symbol that stays
  legible in both light and dark system themes.
- The desktop tray now starts by default on Windows and macOS. Closing the
  window keeps monitoring active, and hovering over the tray symbol shows a
  concise live summary of the computer’s basic health.
- Added a desktop setting for people who prefer the X button to quit the whole
  app and remove the tray symbol. Terminal launches remain exactly as before
  and never create a tray icon.

**Fixed**

- Fixed the blank Windows icon at its source: the artwork is now built into the
  app itself and appears consistently in the window, task switcher, shortcuts,
  installer, and installed-programs list.
- Fixed updates and every installer format so the new app and tray artwork
  travels with the rest of the product, is checked for damage during an update,
  and is removed with the app during uninstall.
- Completed the equivalent app-icon packaging for macOS and all standard Linux
  desktop icon sizes, while keeping the macOS menu-bar symbol monochrome so the
  system can adapt it automatically.

**Behind the scenes**

- Added one repeatable icon-export check so future builds can reproduce the
  exact same platform files from the chosen source artwork instead of relying
  on hand-edited copies.
- Fixed a build-only conflict between the Windows icon resource and the app’s
  full code-analysis check, so adding the real icon does not weaken the release
  test suite on Windows, macOS, or Linux.
- Made the app-discovery test independent of whichever public SD-300 version is
  installed on the computer running it.
- Wrote down, for future maintainers, exactly what went wrong during a recent publishing hiccup and how to avoid it: a change was pushed to the project while a release was still being assembled, which briefly confused the automated packaging steps and led to some being stopped by mistake. Nothing that reached the public was ever affected. The recovery steps, the "don't push while a release is running" rule, and a note that a one-off Apple signing timeout is just a transient glitch are now all documented, plus a follow-up task to make the automation itself more resistant to this in the future.

---

## July 23, 2026 — a quieter release process

**Fixed**

- Fixed the automated release process so that ordinary follow-up changes to the project (like documentation) no longer show a false alarm. Publishing a version permanently locks that version to the exact snapshot it was built from; afterward, everyday edits move the project forward past that snapshot, which the safety check used to flag as if something were wrong. It now recognizes that this is normal and stays quiet, while still stopping any real attempt to change what an already-published version means.

**Behind the scenes**

- Added a clear note in the contributor guide explaining that release safety alarm — what it means, and to investigate rather than override it — so future maintainers don't mistake a genuine warning for noise or a harmless case for a problem.
- Wrote down, in the project's permanent decision records, how the new in-app update actually works under the hood and why it was built that way, plus the judgment calls made while getting the desktop release out the door. Future maintainers — human or AI — can now read the reasoning instead of re-deriving it.
- Gave the project's front page an accuracy pass: it no longer describes the desktop app as "still being tested," it explains the new update-from-the-app option, and its command list is complete.
- Fixed a misleading line in the contributor guide that pointed at a test command which quietly skipped one component's tests; the guide now spells out the extra command that actually runs them.

---

## July 22, 2026 — update from inside the app

**Added**

- You can now update SD-300 from inside the desktop app or from its tray menu. One click runs the very same trusted update the terminal command performs: the app closes while the update installs, then reopens on its own once it succeeds. If you're already up to date, the app simply stays open.
- If an update can't start — for example when the installation is damaged — the app says so plainly and points you to the terminal command, and a small log file records what happened so it can be diagnosed afterward. A failed update never leaves you without the working copy you already had.

**Fixed**

- Fixed a leftover internal label in the app's sidebar that read "QUALIFICATION" next to the version. It now simply shows the version the app is actually running.
- Fixed an automated post-release check that reported a problem even when a release had published correctly, because the service it asked has started turning away anonymous requests. It now asks the same source the package manager itself uses.

**Behind the scenes**

- The version shown on the app's About panel now always comes from the running engine itself — the same arrangement the sidebar already uses — so neither can ever drift out of date, and the release checks were taught about that arrangement.
- The About panel's wording now explains that the in-app update hands the real work to the command-line tool, which stays in charge of installation ownership, elevation, and recovery; removing SD-300 still happens from the terminal.

---

## July 22, 2026 — the desktop app arrives

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
- Fixed a brief follow-up bug from that change where a single wheel click could fling the page all the way to the top or bottom. Each click now moves one steady, predictable step.
- Fixed a subtle compatibility problem where installation records written by the older built-in Windows shell carried an invisible marker that made the program doubt its own installation. The program now reads those records correctly no matter which shell wrote them.
- Extended that same fix to the records that prove a copy was installed through Rust's package manager, so the invisible marker can never make the program refuse an update there either.
- Fixed switching install methods occasionally leaving one dead leftover file from the previous installer behind. The switch now waits for the old installer to finish tidying itself up and clears the leftover if it lingers.
- Fixed uninstalling on fast computers sometimes reporting a failure even though the uninstall was completing correctly — the final tidy-up runs a moment after the main uninstall finishes, and the program was checking too early. It now waits for that tidy-up. When an uninstall genuinely fails, the exact reason is now reported instead of a bare error number.
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

**Verified on the published version**

- The scrolling problem is fixed: wheel movement is batched, each click moves one predictable step, and scrolling stays smooth however long the app has been open.
- Every Windows installation and removal route passed automated testing, including switching between installation methods and upgrading from the previous version.
- The Mac package is signed and notarized by Apple, and the Linux packages carry their own private desktop libraries.
- Installing the published version on a real Windows computer worked end to end: the app opened with live data, exported a report, and uninstalled completely with nothing left behind. Installing through the Rust package manager also works.
- The release carries checksums, a software inventory, and build-provenance records that can be independently verified.

**Deliberately scheduled for after release**

- The full two-hour endurance run and the formal responsiveness measurements now happen against the published version on a quiet machine, rather than delaying the release.
- Broader automated interface testing and a wider performance sweep are queued as follow-up work.
- Windows and Linux screen readers currently see a named application canvas rather than its internal controls. The terminal interface remains the documented accessible fallback.
- Written font-embedding permission is still being obtained from the vendor for a font that was purchased for this purpose.
- Hands-on testing on real Mac hardware is pending access to a Mac.

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
