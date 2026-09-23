# Device inventory evidence

Inventory presence is not a hardware-health measurement. macOS and Linux rows
retain unknown health unless a provider actually reports health. An empty or
unreadable inventory does not create synthetic failed devices. Per-provider and
service observations are retained in both frontends, shared findings and schema 2;
unreadable service booleans are null in that export. Schema 1 retains its original
keys and compatibility booleans.

## Primary contracts

- Linux distinguishes administrative state from operational link state.
  Operational `down` can mean an unplugged cable, so it cannot independently
  establish a disabled driver. See the [kernel operational-state contract](https://www.kernel.org/doc/html/latest/networking/operstates.html).
- The [systemd command documentation](https://github.com/systemd/systemd/blob/main/man/systemctl.xml)
  defines machine-readable `show` properties, load versus active/sub states,
  `MainPID`, and system/user scopes. Audio daemons are queried in the user scope;
  missing managers or units stay unavailable. A loaded one-shot service without
  a PID is not a running process. No service is started or changed.
- Apple's [ioreg manual](https://github.com/apple-oss-distributions/IOKitTools/blob/main/ioreg.tproj/ioreg.8)
  documents XML archives and roots selected by class. SD-300 reads IOHIDDevice
  roots and excludes subordinate registry objects. It preserves separate devices
  with identical product names. [Apple's HID keys](https://github.com/apple-oss-distributions/IOHIDFamily/blob/main/IOHIDFamily/IOHIDKeys.h)
  explain why primary usage alone does not describe all capabilities; the report
  does not infer keyboard or trackpad presence from a single usage or machine type.
- Apple's [launchctl manual](https://github.com/apple-oss-distributions/launchd/blob/main/man/launchctl.1)
  distinguishes a running PID from the last exit status and loaded on-demand jobs.
  Read-only `list` is scoped to the caller's bootstrap domain; an absent row is
  explicitly inconclusive about the system-domain service.

## Verification and limits

Fixtures run on every host for empty/malformed structured data, same-name HID
devices, audio grouping labels, arbitrary input names, missing kernel interfaces,
link down, permission denial, on-demand services and failed/one-shot systemd units.
Native Mac tests execute the bounded registry probe; native Linux tests read the
actual kernel inventory. They establish parser/runtime compatibility, not physical
keyboard operation, audio playback, Bluetooth radio health or driver correctness.

The GUI keeps at most 32 provider observations and bounded service rows. Shared
engine tests cover projection bounds; GUI and export tests distinguish denied
queries from measured inactive services. The TUI inspector exposes the same detail
in both modes. Full release qualification remains required.
