# Monitoring measurement contracts

This document records provider semantics and qualification requirements for v4.
Code fixtures prove parsing/calculation behavior. Native hosted execution proves
supported-runtime behavior. Neither is physical-device accuracy evidence.

## Counter rates and missing samples

Network rates use cumulative interface counters and actual monotonic elapsed
time, keyed by interface identity. Physical disk rates have their own one-second
lane, independent of the minute-scale reliability lane. The parent compares
successive completed counter frames on its monotonic clock. Probe duration and
completion jitter therefore belong in the aligned-window comparison tolerance.
First samples, rollbacks, identity changes, and gaps exceeding ten seconds reset
the baseline. Unavailable rates and absent latency are nullable internally.

An idle device can report a measured zero transfer rate. With no completed I/O,
the interval has no average service time; that latency is unavailable, not zero.
The legacy schema retains its historical scalar shape; schema 2 carries nullable
measurements and observation metadata.

## Windows storage

`IOCTL_DISK_PERFORMANCE` exposes cumulative byte/operation counters and cumulative
service times in **100 ns** units. Divide the service-time delta by completed
operations and convert to milliseconds. Query physical devices directly; never
match provider vector positions. The device key includes its path, storage-manager
device number, descriptor identity hash, and DOS device target. Hardware-health
rows also match physical device number rather than result order.

Primary reference: [Microsoft DISK_PERFORMANCE driver contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntdddisk/ns-ntdddisk-_disk_performance).

## Linux storage

Read `/sys/block/<device>/stat`: sectors are **512 bytes**, operation times are
milliseconds. Aggregate physical leaf devices, excluding partitions, logical
device-mapper/RAID layers with backing devices, loop, and RAM devices. Device
generation (`diskseq`) and available persistent identity reset replacement baselines.
Missing rotational metadata does not imply an SSD.

Primary references: [kernel I/O statistics](https://www.kernel.org/doc/html/latest/admin-guide/iostats.html), [block-stat units](https://www.kernel.org/doc/html/latest/block/stat.html).

## macOS storage

Enumerate `diskutil list -plist physical` and query each validated whole-disk
identifier with `diskutil info -plist`. Read SMARTStatus when provided. A device
without that field remains unknown. For activity, decode the IOKit block-storage
driver's `Statistics` dictionary and associate it with a whole IOMedia BSD name;
registry-entry identity distinguishes driver instances. Byte counters are bytes
and total operation times are **nanoseconds**. Drivers without these counters
remain explicitly unavailable.

Primary reference: [Apple IOBlockStorageDriver statistics definitions](https://github.com/apple-oss-distributions/IOStorageFamily/blob/main/IOBlockStorageDriver.h).

## Optional SMART

An already installed `smartctl` is queried using structured JSON and a read-only,
no-wake request. Installation/elevation must be an explicit user action. Interpret
the exit status as a bitmask and check it against the JSON status. Bits indicating
command/access failures do not prove device failure. Retain valid partial fields;
only an explicit health result or hardware-status bit can establish a fault.
Historical log flags are warnings, not a prediction of imminent failure.

Primary reference: [smartmontools exit-status definitions](https://github.com/smartmontools/smartmontools/blob/main/src/smartctl.h).

## Reports and process interpretation

`sd300 snapshot --json` and `sd300 capabilities --json` retain the frozen schema-1 projection. Add `--schema-version 2` for nullable unavailable measurements, sample capture/interval/sequence metadata and centrally derived findings. GUI exports use schema 2. A measured zero stays numeric; an inaccessible or unprimed process field is null. Process CPU is percent of one logical processor and can exceed 100%. PID plus creation time identifies an instance, including when a PID is reused.

The internal GUI process ABI is version 2; CLI and engine ship together. Both sides assert the revised row and summary layouts. Resource pressure is a workload observation, not a hardware diagnosis. Storage fault findings require the health provider's report. Incomplete or stale collection is kept separate. TUI `F` and the GUI overview expose the same evidence and next steps.

## Power, displays and hardware inventory

Linux power-supply capacity is percent; energy is micro-watt-hours and design voltage is microvolts. Charge-only readings are never converted to energy without a valid model. Unknown power state remains unavailable. The primary battery view is explicitly a selected battery, not a multi-battery aggregate. Network sysfs speed is megabits per second and negative/absent values remain unavailable. DRM connector inventory requires connected status; physical dimensions require a valid EDID base block. A global backlight is not assumed to belong to an arbitrary connector. Firmware identity uses readable DMI data or an ARM device-tree model without elevated reads.

macOS uses CoreGraphics active displays and IOKit's power-source snapshot APIs. Capacity is the ratio of current to maximum in matching provider units, never an assumed energy value; time estimates are minutes and negative values mean unknown. Public enumeration does not provide brightness. Every retained CoreFoundation reference is released; native calls stay inside the cancellable probe process.

Primary references: [Linux power-supply units](https://www.kernel.org/doc/html/latest/power/power_supply_class.html), [network sysfs ABI](https://github.com/torvalds/linux/blob/master/Documentation/ABI/testing/sysfs-class-net), [Apple power-source key contracts](https://github.com/apple-oss-distributions/IOKitUser/blob/main/ps.subproj/IOPSKeys.h), [CoreGraphics active displays](https://developer.apple.com/documentation/coregraphics/cggetactivedisplaylist(_:_:_:)).

## Graphics identity and memory semantics

Windows DXGI adapters use LUID identity and distinct dedicated video, dedicated system, and shared system memory values. D3DKMT maps LUID to PCI bus/device/function; SetupAPI maps PnP inventory to the same location. NVIDIA telemetry is joined only by a unique matching PCI address, never a model-name resemblance. Windows location APIs do not expose PCI segment, so only segment-zero vendor records match. Formatted WDDM counters are summed across processes for each physical engine, then the busiest engine is shown; rounded percentages are bounded at 100. Independent engine percentages are not summed.

Linux inventory uses canonical DRM device paths/PCI addresses. AMD gpu_busy_percent, VRAM byte counters and hwmon millidegrees remain independent nullable fields. macOS uses MTLCopyAllDevices and registry IDs. hasUnifiedMemory describes sharing with the CPU. recommendedMaxWorkingSetSize is an allocation recommendation, not VRAM capacity; currentAllocatedSize is not used as system-wide usage. No public Metal utilization or temperature metric is invented.

Primary references: [DXGI memory categories](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_6/ns-dxgi1_6-dxgi_adapter_desc3), [D3DKMT adapter address](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/ns-d3dkmthk-_d3dkmt_adapteraddress), [Microsoft GPU-engine aggregation](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/), [AMD sysfs monitoring](https://docs.kernel.org/gpu/amdgpu/thermal.html), [Metal device enumeration](https://developer.apple.com/documentation/metal/mtlcopyalldevices()), [Metal memory budget semantics](https://developer.apple.com/documentation/metal/mtldevice/recommendedmaxworkingsetsize).


### Thermal channels

Linux hwmon temperature channels use millidegrees Celsius and fan channels use RPM. Faulted or disabled channels are not measurements. Legacy thermistor voltage channels (`tempN_type=4`) require board calibration and are excluded. Device canonical paths, chip identifiers and channel filenames identify readings; labels never deduplicate channels. A provider that exposes no stable identity remains explicitly unidentified. [Kernel hwmon interface](https://docs.kernel.org/hwmon/sysfs-interface.html).

Windows hardware-monitor and Dell read-only bridges retain provider identifiers. Each graphics adapter contributes its own identified temperature. A generic package or ACPI zone name is insufficient evidence that the reading belongs to the CPU. Technician views expose source and identity; redacted reports remove sensor identifiers. Fixture evidence covers duplicate labels, invalid channels and multiple identical graphics adapters; this does not certify untested physical sensors.


### Native macOS counter access

The reusable Activity worker reads `IOBlockStorageDriver` properties directly through
IOKit, avoiding an `ioreg` launch each second. It releases every service/iterator and
Create/Copy reference, bounds registry traversal and property-list sizes, and requires a
registry ID before accepting counters. The cached structured `diskutil ... physical`
inventory filters out virtual devices and invalidates when registry identities change.
The implementation follows Apple's [IOKit ownership and property APIs](https://github.com/apple-oss-distributions/IOKitUser/blob/main/IOKitLib.h)
and [storage statistics definitions](https://github.com/apple-oss-distributions/IOStorageFamily/blob/main/IOBlockStorageDriver.h).
Intel/ARM cross-checks prove Rust type correctness; native CI provides linking/runtime
coverage. Hardware accuracy still requires the aligned-counter qualification window.


Optional smartctl setup verifies the JSON interface (`json_format_version` major
1, smartctl major 7, exit status zero) without accessing devices. Windows pins
the official smartmontools 7.5 installer SHA-256 from the reviewed winget
manifest and selects only x64, smartctl, drivedb and documentation components.
RunAsInvoker keeps the caller's token; it grants no privilege. macOS uses the
existing Homebrew formula without starting services. Debian/Ubuntu use apt's
authenticated download and dpkg-deb extraction; Alpine explicitly verifies the
package checksum/signature before extraction and fetches repository indexes without a system cache. Missing runtime dependencies are
reported rather than silently modifying the host package database.

Primary references: [smartmontools installer component contract](https://github.com/smartmontools/smartmontools/blob/RELEASE_7_5/smartmontools/os_win32/installer.nsi),
[reviewed Windows checksum](https://github.com/microsoft/winget-pkgs/blob/master/manifests/s/smartmontools/smartmontools/7.5/smartmontools.smartmontools.installer.yaml),
[Homebrew formula](https://formulae.brew.sh/formula/smartmontools),
[Alpine package verification](https://github.com/alpinelinux/apk-tools/blob/v2.14.4/doc/apk-verify.8.scd).

Alpine index behavior follows the [apk no-cache contract](https://github.com/alpinelinux/apk-tools/blob/v2.14.4/doc/apk.8.scd) and its [signed index loader](https://github.com/alpinelinux/apk-tools/blob/v2.14.4/src/database.c).
