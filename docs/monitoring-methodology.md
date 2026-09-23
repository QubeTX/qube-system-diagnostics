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
