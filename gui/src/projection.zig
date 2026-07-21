const std = @import("std");
const native_sdk = @import("native_sdk");
const engine = @import("engine.zig");

const canvas = native_sdk.canvas;

pub const max_cpu_cores: usize = 64;
pub const max_memory_modules: usize = 16;
pub const max_disks: usize = 16;
pub const max_gpus: usize = 8;
pub const max_interfaces: usize = 16;
pub const max_processes: usize = 16;
// Keep row identity stable for a full 30 one-second engine samples. The GUI
// still applies CPU and memory values from every projection it consumes, but a
// near-tie cannot invalidate every process-name/PID cell on each visual frame.
// Thirty samples is short enough to surface a genuinely sustained new top
// consumer while making the table readable and substantially cheaper to paint
// on the SDK's Windows/Linux software presentation paths.
const process_rank_reconcile_samples: u64 = 30;
pub const max_sensors: usize = 24;
pub const max_fans: usize = 12;
pub const max_connections: usize = 20;
pub const max_drivers: usize = 32;
pub const max_drive_health: usize = 16;
pub const max_displays: usize = 8;
pub const max_warnings: usize = 16;
pub const max_capabilities: usize = 48;
pub const max_network_adapters: usize = 16;
pub const max_services: usize = 24;
const topic_count: usize = 9;

pub const TopicMeta = struct {
    ready: bool = false,
    schema_version: u32 = 0,
    sequence: u64 = 0,
    captured_unix_ms: u64 = 0,
    freshness_ms: u64 = 0,
    topic_buffer: canvas.TextBuffer(24) = canvas.TextBuffer(24).init("pending"),
    availability_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("pending"),
    provenance_buffer: canvas.TextBuffer(160) = canvas.TextBuffer(160).init("collector topic pending"),
    target_buffer: canvas.TextBuffer(48) = canvas.TextBuffer(48).init("unknown target"),

    pub fn topic(meta: *const TopicMeta) []const u8 {
        return meta.topic_buffer.text();
    }
    pub fn availability(meta: *const TopicMeta) []const u8 {
        return meta.availability_buffer.text();
    }
    pub fn provenance(meta: *const TopicMeta) []const u8 {
        return meta.provenance_buffer.text();
    }
    pub fn target(meta: *const TopicMeta) []const u8 {
        return meta.target_buffer.text();
    }
};

pub const DisplayRow = struct {
    id: u32 = 0,
    active: bool = false,
    active_available: bool = false,
    brightness_percent: u8 = 0,
    brightness_available: bool = false,
    physical_width_cm: u16 = 0,
    physical_height_cm: u16 = 0,
    size_available: bool = false,
    label_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Display"),
    connection_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("Unknown"),
    source_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("platform display provider"),

    pub fn label(row: *const DisplayRow) []const u8 {
        return row.label_buffer.text();
    }
    pub fn connection(row: *const DisplayRow) []const u8 {
        return row.connection_buffer.text();
    }
    pub fn source(row: *const DisplayRow) []const u8 {
        return row.source_buffer.text();
    }
};

pub const WarningRow = struct {
    id: u32 = 0,
    source_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Collector"),
    message_buffer: canvas.TextBuffer(192) = canvas.TextBuffer(192).init("No detail reported"),
    severity_buffer: canvas.TextBuffer(16) = canvas.TextBuffer(16).init("info"),

    pub fn source(row: *const WarningRow) []const u8 {
        return row.source_buffer.text();
    }
    pub fn message(row: *const WarningRow) []const u8 {
        return row.message_buffer.text();
    }
    pub fn severity(row: *const WarningRow) []const u8 {
        return row.severity_buffer.text();
    }
};

pub const CapabilityRow = struct {
    id: u32 = 0,
    capability_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("unknown"),
    status_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("unavailable"),
    source_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("not_collected"),
    detail_buffer: canvas.TextBuffer(192) = canvas.TextBuffer(192).init(""),

    pub fn capability(row: *const CapabilityRow) []const u8 {
        return row.capability_buffer.text();
    }
    pub fn status(row: *const CapabilityRow) []const u8 {
        return row.status_buffer.text();
    }
    pub fn source(row: *const CapabilityRow) []const u8 {
        return row.source_buffer.text();
    }
    pub fn detail(row: *const CapabilityRow) []const u8 {
        return row.detail_buffer.text();
    }
};

pub const NetworkAdapterRow = struct {
    id: u32 = 0,
    link_speed_mbps: f64 = 0,
    link_speed_available: bool = false,
    hardware_interface: bool = false,
    hardware_available: bool = false,
    name_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Adapter"),
    description_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("Not reported"),
    status_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("Unknown"),

    pub fn name(row: *const NetworkAdapterRow) []const u8 {
        return row.name_buffer.text();
    }
    pub fn description(row: *const NetworkAdapterRow) []const u8 {
        return row.description_buffer.text();
    }
    pub fn status(row: *const NetworkAdapterRow) []const u8 {
        return row.status_buffer.text();
    }
};

pub const CpuCoreRow = struct {
    id: u32 = 0,
    usage_percent: f64 = 0,
    frequency_mhz: u64 = 0,
};

pub const MemoryModuleRow = struct {
    id: u32 = 0,
    capacity_gib: f64 = 0,
    configured_speed_mt_s: u32 = 0,
    rated_speed_mt_s: u32 = 0,
    locator_buffer: canvas.TextBuffer(48) = canvas.TextBuffer(48).init("Module"),
    type_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("Unknown"),
    manufacturer_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Unknown"),
    part_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Not reported"),

    pub fn locator(row: *const MemoryModuleRow) []const u8 {
        return row.locator_buffer.text();
    }
    pub fn memoryType(row: *const MemoryModuleRow) []const u8 {
        return row.type_buffer.text();
    }
    pub fn manufacturer(row: *const MemoryModuleRow) []const u8 {
        return row.manufacturer_buffer.text();
    }
    pub fn partNumber(row: *const MemoryModuleRow) []const u8 {
        return row.part_buffer.text();
    }
};

pub const DiskRow = struct {
    id: u32 = 0,
    total_gib: f64 = 0,
    used_gib: f64 = 0,
    available_gib: f64 = 0,
    usage_percent: f64 = 0,
    removable: bool = false,
    name_buffer: canvas.TextBuffer(72) = canvas.TextBuffer(72).init("Disk"),
    mount_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Unknown"),
    filesystem_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("Unknown"),
    type_buffer: canvas.TextBuffer(24) = canvas.TextBuffer(24).init("unknown"),

    pub fn name(row: *const DiskRow) []const u8 {
        return row.name_buffer.text();
    }
    pub fn mount(row: *const DiskRow) []const u8 {
        return row.mount_buffer.text();
    }
    pub fn filesystem(row: *const DiskRow) []const u8 {
        return row.filesystem_buffer.text();
    }
    pub fn diskType(row: *const DiskRow) []const u8 {
        return row.type_buffer.text();
    }
};

pub const GpuRow = struct {
    id: u32 = 0,
    utilization_percent: f64 = 0,
    memory_used_mib: f64 = 0,
    memory_total_mib: f64 = 0,
    temperature_celsius: f64 = 0,
    refresh_rate_hz: u32 = 0,
    telemetry_available: bool = false,
    temperature_available: bool = false,
    name_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("Graphics adapter"),
    driver_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Not reported"),
    status_buffer: canvas.TextBuffer(48) = canvas.TextBuffer(48).init("Unknown"),
    resolution_buffer: canvas.TextBuffer(48) = canvas.TextBuffer(48).init("Not reported"),
    source_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("platform inventory"),

    pub fn name(row: *const GpuRow) []const u8 {
        return row.name_buffer.text();
    }
    pub fn driver(row: *const GpuRow) []const u8 {
        return row.driver_buffer.text();
    }
    pub fn status(row: *const GpuRow) []const u8 {
        return row.status_buffer.text();
    }
    pub fn resolution(row: *const GpuRow) []const u8 {
        return row.resolution_buffer.text();
    }
    pub fn source(row: *const GpuRow) []const u8 {
        return row.source_buffer.text();
    }
};

pub const InterfaceRow = struct {
    id: u32 = 0,
    download_kib_s: f64 = 0,
    upload_kib_s: f64 = 0,
    received_gib: f64 = 0,
    transmitted_gib: f64 = 0,
    is_up: bool = false,
    name_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("Interface"),
    state_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("unknown"),
    address_buffer: canvas.TextBuffer(160) = canvas.TextBuffer(160).init("No address"),
    mac_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("Not reported"),

    pub fn name(row: *const InterfaceRow) []const u8 {
        return row.name_buffer.text();
    }
    pub fn state(row: *const InterfaceRow) []const u8 {
        return row.state_buffer.text();
    }
    pub fn address(row: *const InterfaceRow) []const u8 {
        return row.address_buffer.text();
    }
    pub fn mac(row: *const InterfaceRow) []const u8 {
        return row.mac_buffer.text();
    }
};

pub const ProcessRow = struct {
    id: u32 = 0,
    pid: u32 = 0,
    cpu_percent: f64 = 0,
    memory_mib: f64 = 0,
    memory_percent: f64 = 0,
    name_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Process"),
    friendly_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Process"),
    status_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("unknown"),

    pub fn name(row: *const ProcessRow) []const u8 {
        return row.name_buffer.text();
    }
    pub fn friendlyName(row: *const ProcessRow) []const u8 {
        return row.friendly_buffer.text();
    }
    pub fn status(row: *const ProcessRow) []const u8 {
        return row.status_buffer.text();
    }
};

pub const SensorRow = struct {
    id: u32 = 0,
    temperature_celsius: f64 = 0,
    critical_celsius: f64 = 0,
    critical_available: bool = false,
    label_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("Sensor"),
    kind_buffer: canvas.TextBuffer(24) = canvas.TextBuffer(24).init("other"),
    source_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("platform sensor"),

    pub fn label(row: *const SensorRow) []const u8 {
        return row.label_buffer.text();
    }
    pub fn kind(row: *const SensorRow) []const u8 {
        return row.kind_buffer.text();
    }
    pub fn source(row: *const SensorRow) []const u8 {
        return row.source_buffer.text();
    }
};

pub const FanRow = struct {
    id: u32 = 0,
    rpm: u64 = 0,
    label_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("Fan"),
    source_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("platform sensor"),

    pub fn label(row: *const FanRow) []const u8 {
        return row.label_buffer.text();
    }
    pub fn source(row: *const FanRow) []const u8 {
        return row.source_buffer.text();
    }
};

pub const ConnectionRow = struct {
    id: u32 = 0,
    local_port: u16 = 0,
    remote_port: u16 = 0,
    pid: u32 = 0,
    protocol_buffer: canvas.TextBuffer(12) = canvas.TextBuffer(12).init("tcp"),
    local_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("*"),
    remote_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("*"),
    state_buffer: canvas.TextBuffer(48) = canvas.TextBuffer(48).init("unknown"),
    process_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("Not reported"),

    pub fn protocol(row: *const ConnectionRow) []const u8 {
        return row.protocol_buffer.text();
    }
    pub fn local(row: *const ConnectionRow) []const u8 {
        return row.local_buffer.text();
    }
    pub fn remote(row: *const ConnectionRow) []const u8 {
        return row.remote_buffer.text();
    }
    pub fn state(row: *const ConnectionRow) []const u8 {
        return row.state_buffer.text();
    }
    pub fn processName(row: *const ConnectionRow) []const u8 {
        return row.process_buffer.text();
    }
};

pub const DriverRow = struct {
    id: u32 = 0,
    attention: bool = false,
    name_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("Device"),
    category_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("other"),
    version_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Not reported"),
    date_buffer: canvas.TextBuffer(40) = canvas.TextBuffer(40).init("Not reported"),
    status_buffer: canvas.TextBuffer(112) = canvas.TextBuffer(112).init("unknown"),
    detail_buffer: canvas.TextBuffer(160) = canvas.TextBuffer(160).init(""),

    pub fn name(row: *const DriverRow) []const u8 {
        return row.name_buffer.text();
    }
    pub fn category(row: *const DriverRow) []const u8 {
        return row.category_buffer.text();
    }
    pub fn version(row: *const DriverRow) []const u8 {
        return row.version_buffer.text();
    }
    pub fn date(row: *const DriverRow) []const u8 {
        return row.date_buffer.text();
    }
    pub fn status(row: *const DriverRow) []const u8 {
        return row.status_buffer.text();
    }
    pub fn detail(row: *const DriverRow) []const u8 {
        return row.detail_buffer.text();
    }
};

pub const DriveHealthRow = struct {
    id: u32 = 0,
    temperature_celsius: f64 = 0,
    temperature_available: bool = false,
    wear_percent: u8 = 0,
    wear_available: bool = false,
    power_on_hours: u64 = 0,
    power_on_hours_available: bool = false,
    read_errors_total: u64 = 0,
    write_errors_total: u64 = 0,
    error_counts_available: bool = false,
    read_mib_s: f64 = 0,
    write_mib_s: f64 = 0,
    queue_depth: f64 = 0,
    read_latency_ms: f64 = 0,
    write_latency_ms: f64 = 0,
    io_available: bool = false,
    model_buffer: canvas.TextBuffer(112) = canvas.TextBuffer(112).init("Physical drive"),
    media_buffer: canvas.TextBuffer(24) = canvas.TextBuffer(24).init("unknown"),
    health_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("unknown"),
    source_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("platform storage provider"),

    pub fn model(row: *const DriveHealthRow) []const u8 {
        return row.model_buffer.text();
    }
    pub fn mediaType(row: *const DriveHealthRow) []const u8 {
        return row.media_buffer.text();
    }
    pub fn health(row: *const DriveHealthRow) []const u8 {
        return row.health_buffer.text();
    }
    pub fn source(row: *const DriveHealthRow) []const u8 {
        return row.source_buffer.text();
    }
};

pub const ServiceRow = struct {
    id: u32 = 0,
    running: bool = false,
    name_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Service"),
    display_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("Service"),

    pub fn name(row: *const ServiceRow) []const u8 {
        return row.name_buffer.text();
    }
    pub fn displayName(row: *const ServiceRow) []const u8 {
        return row.display_buffer.text();
    }
    pub fn state(row: *const ServiceRow) []const u8 {
        return if (row.running) "running" else "not running";
    }
};

pub const Projection = struct {
    static_ready: bool = false,
    fast_ready: bool = false,
    slow_ready: bool = false,
    medium_ready: bool = false,
    diagnostics_ready: bool = false,
    health_ready: bool = false,
    drivers_ready: bool = false,
    warnings_ready: bool = false,
    capabilities_ready: bool = false,
    topic_meta: [topic_count]TopicMeta = [_]TopicMeta{.{}} ** topic_count,
    os_name_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Operating system"),
    os_version_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Unknown version"),
    hostname_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Unknown host"),
    system_cpu_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("Unknown processor"),
    architecture_buffer: canvas.TextBuffer(32) = canvas.TextBuffer(32).init("unknown"),
    kernel_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Unknown kernel"),
    manufacturer_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Not reported"),
    model_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Not reported"),
    bios_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("Not reported"),
    uptime_seconds: u64 = 0,
    hypervisor_present: bool = false,
    hypervisor_available: bool = false,
    cpu_model_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("Waiting for CPU identity"),
    physical_core_count: u32 = 0,
    logical_thread_count: u32 = 0,
    memory_available_gib: f64 = 0,
    swap_used_gib: f64 = 0,
    swap_total_gib: f64 = 0,
    total_download_kib_s: f64 = 0,
    total_upload_kib_s: f64 = 0,
    process_total_count: u32 = 0,
    process_total_threads: u32 = 0,
    process_samples_observed: u8 = 0,
    process_values_warmed: bool = false,
    cpu_temperature_celsius: f64 = 0,
    gpu_temperature_celsius: f64 = 0,
    cpu_temperature_available: bool = false,
    gpu_temperature_available: bool = false,
    battery_available: bool = false,
    battery_percent: f64 = 0,
    battery_charging: bool = false,
    battery_on_ac: bool = false,
    battery_capacity_mwh: u64 = 0,
    battery_capacity_available: bool = false,
    battery_voltage_mv: u64 = 0,
    battery_voltage_available: bool = false,
    battery_cycle_count: u32 = 0,
    battery_cycle_available: bool = false,
    power_source_buffer: canvas.TextBuffer(24) = canvas.TextBuffer(24).init("unknown"),
    battery_time_buffer: canvas.TextBuffer(48) = canvas.TextBuffer(48).init("Not reported"),
    battery_provider_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init("Not reported"),
    gateway_reachable: bool = false,
    gateway_latency_ms: f64 = 0,
    gateway_latency_available: bool = false,
    dns_resolved: bool = false,
    dns_latency_ms: f64 = 0,
    dns_latency_available: bool = false,
    internet_reachable: bool = false,
    internet_latency_ms: f64 = 0,
    internet_latency_available: bool = false,
    gateway_target_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Not reported"),
    gateway_error_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init(""),
    dns_domain_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("Not reported"),
    dns_result_buffer: canvas.TextBuffer(80) = canvas.TextBuffer(80).init("Not reported"),
    dns_error_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init(""),
    internet_target_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Not reported"),
    internet_error_buffer: canvas.TextBuffer(128) = canvas.TextBuffer(128).init(""),
    driver_total_count: u32 = 0,
    driver_attention_count: u32 = 0,
    driver_scan_buffer: canvas.TextBuffer(96) = canvas.TextBuffer(96).init("not scanned"),
    cpu_core_rows: [max_cpu_cores]CpuCoreRow = [_]CpuCoreRow{.{}} ** max_cpu_cores,
    cpu_core_count: usize = 0,
    memory_module_rows: [max_memory_modules]MemoryModuleRow = [_]MemoryModuleRow{.{}} ** max_memory_modules,
    memory_module_count: usize = 0,
    disk_rows: [max_disks]DiskRow = [_]DiskRow{.{}} ** max_disks,
    disk_count: usize = 0,
    gpu_rows: [max_gpus]GpuRow = [_]GpuRow{.{}} ** max_gpus,
    gpu_count: usize = 0,
    interface_rows: [max_interfaces]InterfaceRow = [_]InterfaceRow{.{}} ** max_interfaces,
    interface_count: usize = 0,
    process_rows: [max_processes]ProcessRow = [_]ProcessRow{.{}} ** max_processes,
    process_count: usize = 0,
    process_order_sequence: u64 = 0,
    sensor_rows: [max_sensors]SensorRow = [_]SensorRow{.{}} ** max_sensors,
    sensor_count: usize = 0,
    fan_rows: [max_fans]FanRow = [_]FanRow{.{}} ** max_fans,
    fan_count: usize = 0,
    connection_rows: [max_connections]ConnectionRow = [_]ConnectionRow{.{}} ** max_connections,
    connection_count: usize = 0,
    connection_total_count: u32 = 0,
    listening_count: u32 = 0,
    driver_rows: [max_drivers]DriverRow = [_]DriverRow{.{}} ** max_drivers,
    driver_count: usize = 0,
    drive_health_rows: [max_drive_health]DriveHealthRow = [_]DriveHealthRow{.{}} ** max_drive_health,
    drive_health_count: usize = 0,
    disk_io_available: bool = false,
    disk_read_mib_s: f64 = 0,
    disk_write_mib_s: f64 = 0,
    disk_queue_depth: f64 = 0,
    disk_read_latency_ms: f64 = 0,
    disk_write_latency_ms: f64 = 0,
    disk_read_errors_total: u64 = 0,
    disk_write_errors_total: u64 = 0,
    disk_errors_available: bool = false,
    display_rows: [max_displays]DisplayRow = [_]DisplayRow{.{}} ** max_displays,
    display_count: usize = 0,
    warning_rows: [max_warnings]WarningRow = [_]WarningRow{.{}} ** max_warnings,
    warning_row_count: usize = 0,
    warning_total_count: u32 = 0,
    capability_rows: [max_capabilities]CapabilityRow = [_]CapabilityRow{.{}} ** max_capabilities,
    capability_count: usize = 0,
    network_adapter_rows: [max_network_adapters]NetworkAdapterRow = [_]NetworkAdapterRow{.{}} ** max_network_adapters,
    network_adapter_count: usize = 0,
    service_rows: [max_services]ServiceRow = [_]ServiceRow{.{}} ** max_services,
    service_count: usize = 0,

    pub fn osName(self: *const Projection) []const u8 {
        return self.os_name_buffer.text();
    }
    pub fn osVersion(self: *const Projection) []const u8 {
        return self.os_version_buffer.text();
    }
    pub fn hostname(self: *const Projection) []const u8 {
        return self.hostname_buffer.text();
    }
    pub fn systemCpu(self: *const Projection) []const u8 {
        return self.system_cpu_buffer.text();
    }
    pub fn architecture(self: *const Projection) []const u8 {
        return self.architecture_buffer.text();
    }
    pub fn kernel(self: *const Projection) []const u8 {
        return self.kernel_buffer.text();
    }
    pub fn manufacturer(self: *const Projection) []const u8 {
        return self.manufacturer_buffer.text();
    }
    pub fn systemModel(self: *const Projection) []const u8 {
        return self.model_buffer.text();
    }
    pub fn bios(self: *const Projection) []const u8 {
        return self.bios_buffer.text();
    }

    pub fn cpuModel(self: *const Projection) []const u8 {
        return self.cpu_model_buffer.text();
    }
    pub fn driverScanStatus(self: *const Projection) []const u8 {
        return self.driver_scan_buffer.text();
    }
    pub fn powerSource(self: *const Projection) []const u8 {
        return self.power_source_buffer.text();
    }
    pub fn batteryTime(self: *const Projection) []const u8 {
        return self.battery_time_buffer.text();
    }
    pub fn batteryProvider(self: *const Projection) []const u8 {
        return self.battery_provider_buffer.text();
    }
    pub fn gatewayTarget(self: *const Projection) []const u8 {
        return self.gateway_target_buffer.text();
    }
    pub fn gatewayError(self: *const Projection) []const u8 {
        return self.gateway_error_buffer.text();
    }
    pub fn dnsDomain(self: *const Projection) []const u8 {
        return self.dns_domain_buffer.text();
    }
    pub fn dnsResult(self: *const Projection) []const u8 {
        return self.dns_result_buffer.text();
    }
    pub fn dnsError(self: *const Projection) []const u8 {
        return self.dns_error_buffer.text();
    }
    pub fn internetTarget(self: *const Projection) []const u8 {
        return self.internet_target_buffer.text();
    }
    pub fn internetError(self: *const Projection) []const u8 {
        return self.internet_error_buffer.text();
    }
    pub fn cpuCores(self: *const Projection) []const CpuCoreRow {
        return self.cpu_core_rows[0..self.cpu_core_count];
    }
    pub fn memoryModules(self: *const Projection) []const MemoryModuleRow {
        return self.memory_module_rows[0..self.memory_module_count];
    }
    pub fn disks(self: *const Projection) []const DiskRow {
        return self.disk_rows[0..self.disk_count];
    }
    pub fn gpus(self: *const Projection) []const GpuRow {
        return self.gpu_rows[0..self.gpu_count];
    }
    pub fn interfaces(self: *const Projection) []const InterfaceRow {
        return self.interface_rows[0..self.interface_count];
    }
    pub fn processes(self: *const Projection) []const ProcessRow {
        return self.process_rows[0..self.process_count];
    }
    pub fn sensors(self: *const Projection) []const SensorRow {
        return self.sensor_rows[0..self.sensor_count];
    }
    pub fn fans(self: *const Projection) []const FanRow {
        return self.fan_rows[0..self.fan_count];
    }
    pub fn connections(self: *const Projection) []const ConnectionRow {
        return self.connection_rows[0..self.connection_count];
    }
    pub fn drivers(self: *const Projection) []const DriverRow {
        return self.driver_rows[0..self.driver_count];
    }
    pub fn driveHealth(self: *const Projection) []const DriveHealthRow {
        return self.drive_health_rows[0..self.drive_health_count];
    }
    pub fn displays(self: *const Projection) []const DisplayRow {
        return self.display_rows[0..self.display_count];
    }
    pub fn warnings(self: *const Projection) []const WarningRow {
        return self.warning_rows[0..self.warning_row_count];
    }
    pub fn capabilities(self: *const Projection) []const CapabilityRow {
        return self.capability_rows[0..self.capability_count];
    }
    pub fn networkAdapters(self: *const Projection) []const NetworkAdapterRow {
        return self.network_adapter_rows[0..self.network_adapter_count];
    }
    pub fn services(self: *const Projection) []const ServiceRow {
        return self.service_rows[0..self.service_count];
    }
    pub fn topicMeta(self: *const Projection, index: usize) *const TopicMeta {
        return &self.topic_meta[@min(index, topic_count - 1)];
    }

    fn captureTopicMeta(self: *Projection, index: usize, envelope: anytype) void {
        var meta = TopicMeta{
            .ready = true,
            .schema_version = envelope.schema_version,
            .sequence = envelope.sequence,
            .captured_unix_ms = envelope.captured_unix_ms,
            .freshness_ms = envelope.freshness_ms,
        };
        meta.topic_buffer.set(envelope.topic);
        meta.availability_buffer.set(envelope.availability);
        meta.provenance_buffer.set(envelope.provenance);
        meta.target_buffer.set(envelope.target);
        self.topic_meta[index] = meta;
    }

    pub fn applyStaticJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope(StaticDataJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.captureTopicMeta(0, parsed.value);
        const data = parsed.value.data;
        self.static_ready = true;
        self.os_name_buffer.set(data.system.os_name);
        self.os_version_buffer.set(data.system.os_version);
        self.hostname_buffer.set(data.system.hostname);
        self.system_cpu_buffer.set(data.system.cpu_model);
        self.architecture_buffer.set(data.system.architecture);
        self.kernel_buffer.set(data.system.kernel_version);
        self.manufacturer_buffer.set(data.system.manufacturer orelse "Not reported");
        self.model_buffer.set(data.system.model orelse "Not reported");
        self.bios_buffer.set(data.system.bios_version orelse "Not reported");
        self.uptime_seconds = data.system.uptime_seconds;
        self.hypervisor_present = data.system.hypervisor_present orelse false;
        self.hypervisor_available = data.system.hypervisor_present != null;

        self.network_adapter_count = @min(data.network_adapters.len, max_network_adapters);
        for (data.network_adapters[0..self.network_adapter_count], 0..) |item, index| {
            var row = NetworkAdapterRow{ .id = @intCast(index) };
            row.name_buffer.set(item.name);
            row.description_buffer.set(item.description orelse "Not reported");
            row.status_buffer.set(item.status orelse "Unknown");
            row.link_speed_mbps = if (item.link_speed_bps) |speed| @as(f64, @floatFromInt(speed)) / 1_000_000.0 else 0;
            row.link_speed_available = item.link_speed_bps != null;
            row.hardware_interface = item.hardware_interface orelse false;
            row.hardware_available = item.hardware_interface != null;
            self.network_adapter_rows[index] = row;
        }

        self.display_count = @min(data.displays.displays.len, max_displays);
        for (data.displays.displays[0..self.display_count], 0..) |item, index| {
            var row = DisplayRow{ .id = @intCast(index) };
            row.label_buffer.set(item.label);
            row.connection_buffer.set(item.connection);
            row.source_buffer.set(item.source);
            row.active = item.active orelse false;
            row.active_available = item.active != null;
            row.brightness_percent = item.brightness_percent orelse 0;
            row.brightness_available = item.brightness_percent != null;
            row.physical_width_cm = item.physical_width_cm orelse 0;
            row.physical_height_cm = item.physical_height_cm orelse 0;
            row.size_available = item.physical_width_cm != null and item.physical_height_cm != null;
            self.display_rows[index] = row;
        }
    }

    pub fn applyWarningsJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope([]const WarningJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.captureTopicMeta(7, parsed.value);
        self.warnings_ready = true;
        self.warning_total_count = saturatedU32(parsed.value.data.len);
        self.warning_row_count = @min(parsed.value.data.len, max_warnings);
        for (parsed.value.data[0..self.warning_row_count], 0..) |item, index| {
            var row = WarningRow{ .id = @intCast(index) };
            row.source_buffer.set(item.source);
            row.message_buffer.set(item.message);
            row.severity_buffer.set(item.severity);
            self.warning_rows[index] = row;
        }
    }

    pub fn applyCapabilitiesJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope([]const CapabilityJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.captureTopicMeta(8, parsed.value);
        self.capabilities_ready = true;
        self.capability_count = @min(parsed.value.data.len, max_capabilities);
        for (parsed.value.data[0..self.capability_count], 0..) |item, index| {
            var row = CapabilityRow{ .id = @intCast(index) };
            row.capability_buffer.set(item.id);
            row.status_buffer.set(item.status);
            row.source_buffer.set(item.source);
            row.detail_buffer.set(item.detail orelse "");
            self.capability_rows[index] = row;
        }
    }

    pub fn applyFastJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope(FastDataJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        const envelope = parsed.value;
        self.captureTopicMeta(1, envelope);
        const data = envelope.data;
        self.fast_ready = true;
        self.cpu_model_buffer.set(data.cpu.cpu_model);
        self.physical_core_count = saturatedU32(data.cpu.core_count);
        self.logical_thread_count = saturatedU32(data.cpu.thread_count);

        self.cpu_core_count = @min(data.cpu.per_core_usage.len, max_cpu_cores);
        for (data.cpu.per_core_usage[0..self.cpu_core_count], 0..) |usage, index| {
            self.cpu_core_rows[index] = .{
                .id = @intCast(index),
                .usage_percent = usage,
                .frequency_mhz = if (index < data.cpu.per_core_frequency.len) data.cpu.per_core_frequency[index] else 0,
            };
        }

        const gib = 1024.0 * 1024.0 * 1024.0;
        self.memory_available_gib = @as(f64, @floatFromInt(data.memory.available_bytes)) / gib;
        self.swap_used_gib = @as(f64, @floatFromInt(data.memory.swap_used_bytes)) / gib;
        self.swap_total_gib = @as(f64, @floatFromInt(data.memory.swap_total_bytes)) / gib;
        self.memory_module_count = @min(data.memory.modules.len, max_memory_modules);
        for (data.memory.modules[0..self.memory_module_count], 0..) |module, index| {
            var row = MemoryModuleRow{ .id = @intCast(index) };
            row.capacity_gib = @as(f64, @floatFromInt(module.capacity_bytes)) / gib;
            row.configured_speed_mt_s = module.configured_speed_mt_s orelse 0;
            row.rated_speed_mt_s = module.rated_speed_mt_s orelse 0;
            row.locator_buffer.set(module.locator orelse "Module");
            row.type_buffer.set(module.memory_type orelse "Unknown");
            row.manufacturer_buffer.set(module.manufacturer orelse "Unknown");
            row.part_buffer.set(module.part_number orelse "Not reported");
            self.memory_module_rows[index] = row;
        }

        self.total_download_kib_s = @as(f64, @floatFromInt(data.network.total_download_rate)) / 1024.0;
        self.total_upload_kib_s = @as(f64, @floatFromInt(data.network.total_upload_rate)) / 1024.0;
        self.interface_count = @min(data.network.interfaces.len, max_interfaces);
        for (data.network.interfaces[0..self.interface_count], 0..) |item, index| {
            var row = InterfaceRow{ .id = @intCast(index) };
            row.name_buffer.set(item.name);
            row.state_buffer.set(item.operational_state);
            row.mac_buffer.set(item.mac_address);
            row.address_buffer.set(if (item.ip_addresses.len > 0) item.ip_addresses[0] else "No address");
            row.download_kib_s = @as(f64, @floatFromInt(item.download_rate)) / 1024.0;
            row.upload_kib_s = @as(f64, @floatFromInt(item.upload_rate)) / 1024.0;
            row.received_gib = @as(f64, @floatFromInt(item.received_bytes)) / gib;
            row.transmitted_gib = @as(f64, @floatFromInt(item.transmitted_bytes)) / gib;
            row.is_up = item.is_up;
            self.interface_rows[index] = row;
        }

        const candidate_count = @min(data.processes.list.len, max_processes);
        var candidate_rows = [_]ProcessRow{.{}} ** max_processes;
        for (data.processes.list[0..candidate_count], 0..) |item, index| {
            candidate_rows[index] = processRow(item);
        }
        self.applyProcessRows(
            envelope.sequence,
            saturatedU32(data.processes.total_count),
            saturatedU32(data.processes.total_threads),
            candidate_rows[0..candidate_count],
        );
    }

    pub fn applyProcessSummary(self: *Projection, summary: engine.ProcessSummary) void {
        var meta = self.topic_meta[1];
        meta.ready = true;
        meta.schema_version = 1;
        meta.sequence = summary.sequence;
        meta.captured_unix_ms = summary.captured_unix_ms;
        meta.freshness_ms = 0;
        meta.topic_buffer.set("fast");
        meta.availability_buffer.set("available");
        meta.provenance_buffer.set("SD-300 platform process collector");
        if (!self.topic_meta[1].ready) {
            meta.target_buffer.set(if (self.topic_meta[0].ready) self.topic_meta[0].target() else "active target");
        }
        self.topic_meta[1] = meta;
        self.fast_ready = true;

        const candidate_count = @min(@as(usize, @intCast(summary.row_count)), max_processes);
        var candidate_rows = [_]ProcessRow{.{}} ** max_processes;
        for (summary.rows[0..candidate_count], 0..) |item, index| {
            var row = ProcessRow{ .id = item.pid, .pid = item.pid };
            row.name_buffer.set(summaryText(&item.name, item.name_len));
            row.friendly_buffer.set(summaryText(&item.friendly_name, item.friendly_name_len));
            row.status_buffer.set(summaryText(&item.status, item.status_len));
            row.cpu_percent = item.cpu_percent;
            row.memory_mib = @as(f64, @floatFromInt(item.memory_bytes)) / (1024.0 * 1024.0);
            row.memory_percent = item.memory_percent;
            candidate_rows[index] = row;
        }
        self.applyProcessRows(
            summary.sequence,
            summary.total_count,
            summary.total_threads,
            candidate_rows[0..candidate_count],
        );
    }

    fn applyProcessRows(
        self: *Projection,
        sequence: u64,
        total_count: u32,
        total_threads: u32,
        candidates: []const ProcessRow,
    ) void {
        self.process_total_count = total_count;
        self.process_total_threads = total_threads;
        if (total_count > 0) {
            self.process_samples_observed +|= 1;
            self.process_values_warmed = self.process_samples_observed >= 2;
        }
        const candidate_count = candidates.len;
        const reorder_due = self.process_count == 0 or
            sequence <= self.process_order_sequence or
            sequence - self.process_order_sequence >= process_rank_reconcile_samples;
        if (reorder_due) {
            self.process_count = candidate_count;
            self.process_order_sequence = sequence;
            for (candidates, 0..) |item, index| {
                self.process_rows[index] = item;
            }
        } else {
            // Live CPU and memory values still update every fast sample. Keep
            // row positions stable between sustained rank reconciliations so
            // a one-second sample does not repaint every name/status cell just
            // because two near-equal processes swapped order.
            const previous_rows = self.process_rows;
            const previous_count = self.process_count;
            var next_rows = [_]ProcessRow{.{}} ** max_processes;
            var used = [_]bool{false} ** max_processes;
            var next_count: usize = 0;

            for (previous_rows[0..previous_count]) |previous| {
                for (candidates, 0..) |item, candidate_index| {
                    if (!used[candidate_index] and item.pid == previous.pid) {
                        next_rows[next_count] = item;
                        used[candidate_index] = true;
                        next_count += 1;
                        break;
                    }
                }
            }
            for (candidates, 0..) |item, candidate_index| {
                if (next_count >= candidate_count) break;
                if (used[candidate_index]) continue;
                next_rows[next_count] = item;
                next_count += 1;
            }
            self.process_rows = next_rows;
            self.process_count = next_count;
        }
    }

    pub fn applySlowJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope(SlowDataJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.captureTopicMeta(2, parsed.value);
        const data = parsed.value.data;
        self.slow_ready = true;
        const gib = 1024.0 * 1024.0 * 1024.0;

        self.disk_count = @min(data.disk.partitions.len, max_disks);
        for (data.disk.partitions[0..self.disk_count], 0..) |item, index| {
            var row = DiskRow{ .id = @intCast(index), .removable = item.is_removable };
            row.name_buffer.set(item.name);
            row.mount_buffer.set(item.mount_point);
            row.filesystem_buffer.set(item.filesystem);
            row.type_buffer.set(item.disk_type);
            row.total_gib = @as(f64, @floatFromInt(item.total_bytes)) / gib;
            row.used_gib = @as(f64, @floatFromInt(item.used_bytes)) / gib;
            row.available_gib = @as(f64, @floatFromInt(item.available_bytes)) / gib;
            row.usage_percent = if (item.total_bytes == 0) 0 else @as(f64, @floatFromInt(item.used_bytes)) / @as(f64, @floatFromInt(item.total_bytes)) * 100;
            self.disk_rows[index] = row;
        }

        self.gpu_count = @min(data.gpu.adapters.len, max_gpus);
        for (data.gpu.adapters[0..self.gpu_count], 0..) |item, index| {
            var row = GpuRow{ .id = @intCast(index), .telemetry_available = item.telemetry_available };
            row.name_buffer.set(item.name);
            row.driver_buffer.set(item.driver_version orelse "Not reported");
            row.status_buffer.set(item.status orelse "Unknown");
            row.resolution_buffer.set(item.current_resolution orelse "Not reported");
            row.source_buffer.set(item.source);
            row.utilization_percent = item.utilization_percent orelse 0;
            row.memory_used_mib = @floatFromInt(item.memory_used_mb orelse 0);
            row.memory_total_mib = @floatFromInt(item.dedicated_memory_mb orelse 0);
            row.temperature_celsius = item.temperature_celsius orelse 0;
            row.temperature_available = item.temperature_celsius != null;
            row.refresh_rate_hz = item.refresh_rate_hz orelse 0;
            self.gpu_rows[index] = row;
        }

        self.cpu_temperature_celsius = data.thermals.cpu_temp orelse 0;
        self.gpu_temperature_celsius = data.thermals.gpu_temp orelse 0;
        self.cpu_temperature_available = data.thermals.cpu_temp != null;
        self.gpu_temperature_available = data.thermals.gpu_temp != null;
        self.power_source_buffer.set(data.thermals.power_source);
        if (data.thermals.battery) |battery| {
            self.battery_available = true;
            self.battery_percent = battery.percent;
            self.battery_charging = battery.is_charging;
            self.battery_on_ac = battery.is_on_ac;
            self.battery_time_buffer.set(battery.time_remaining orelse "Not reported");
            self.battery_capacity_mwh = battery.full_charged_capacity_mwh orelse 0;
            self.battery_capacity_available = battery.full_charged_capacity_mwh != null;
            self.battery_voltage_mv = battery.design_voltage_mv orelse 0;
            self.battery_voltage_available = battery.design_voltage_mv != null;
            self.battery_cycle_count = battery.cycle_count orelse 0;
            self.battery_cycle_available = battery.cycle_count != null;
            self.battery_provider_buffer.set(battery.provider_status orelse "Available");
        } else {
            self.battery_available = false;
        }
        self.sensor_count = @min(data.thermals.sensors.len, max_sensors);
        for (data.thermals.sensors[0..self.sensor_count], 0..) |item, index| {
            var row = SensorRow{ .id = @intCast(index) };
            row.label_buffer.set(item.label);
            row.kind_buffer.set(item.kind);
            row.source_buffer.set(item.source);
            row.temperature_celsius = item.temperature;
            row.critical_celsius = item.critical orelse 0;
            row.critical_available = item.critical != null;
            self.sensor_rows[index] = row;
        }
        self.fan_count = @min(data.thermals.fans.len, max_fans);
        for (data.thermals.fans[0..self.fan_count], 0..) |item, index| {
            var row = FanRow{ .id = @intCast(index), .rpm = item.rpm };
            row.label_buffer.set(item.label);
            row.source_buffer.set(item.source);
            self.fan_rows[index] = row;
        }
    }

    pub fn applyMediumJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope(MediumJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.captureTopicMeta(3, parsed.value);
        self.medium_ready = true;
        const data = parsed.value.data;
        self.listening_count = saturatedU32(data.listening_ports.len);
        self.connection_total_count = saturatedU32(data.active_connections.len);
        self.connection_count = @min(data.active_connections.len, max_connections);
        for (data.active_connections[0..self.connection_count], 0..) |item, index| {
            var row = ConnectionRow{ .id = @intCast(index), .local_port = item.local_port, .remote_port = item.remote_port, .pid = item.pid orelse 0 };
            row.protocol_buffer.set(item.protocol);
            row.local_buffer.set(item.local_addr);
            row.remote_buffer.set(item.remote_addr);
            row.process_buffer.set(item.process_name orelse "Not reported");
            setValueLabel(&row.state_buffer, item.state);
            self.connection_rows[index] = row;
        }
    }

    pub fn applyDiagnosticsJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope(DiagnosticsJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.captureTopicMeta(4, parsed.value);
        const data = parsed.value.data;
        self.diagnostics_ready = true;
        self.gateway_reachable = data.gateway.reachable;
        self.gateway_latency_ms = data.gateway.latency_ms orelse 0;
        self.gateway_latency_available = data.gateway.latency_ms != null;
        self.gateway_target_buffer.set(data.gateway.target);
        self.gateway_error_buffer.set(data.gateway.@"error" orelse "");
        self.dns_resolved = data.dns.resolved;
        self.dns_latency_ms = data.dns.resolution_ms orelse 0;
        self.dns_latency_available = data.dns.resolution_ms != null;
        self.dns_domain_buffer.set(data.dns.domain);
        self.dns_result_buffer.set(data.dns.resolved_ip orelse "Not resolved");
        self.dns_error_buffer.set(data.dns.@"error" orelse "");
        self.internet_reachable = data.internet.reachable;
        self.internet_latency_ms = data.internet.latency_ms orelse 0;
        self.internet_latency_available = data.internet.latency_ms != null;
        self.internet_target_buffer.set(data.internet.target);
        self.internet_error_buffer.set(data.internet.@"error" orelse "");
    }

    pub fn applyHealthJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope(HealthJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.captureTopicMeta(5, parsed.value);
        self.health_ready = true;
        self.disk_io_available = false;
        self.disk_read_mib_s = 0;
        self.disk_write_mib_s = 0;
        self.disk_queue_depth = 0;
        self.disk_read_latency_ms = 0;
        self.disk_write_latency_ms = 0;
        self.disk_read_errors_total = 0;
        self.disk_write_errors_total = 0;
        self.disk_errors_available = false;
        self.drive_health_count = @min(parsed.value.data.drives.len, max_drive_health);
        for (parsed.value.data.drives[0..self.drive_health_count], 0..) |item, index| {
            var row = DriveHealthRow{ .id = @intCast(index) };
            row.model_buffer.set(item.model);
            row.media_buffer.set(item.media_type);
            row.health_buffer.set(item.health_status);
            row.source_buffer.set(item.health_source);
            row.temperature_celsius = item.temperature_celsius orelse 0;
            row.temperature_available = item.temperature_celsius != null;
            row.wear_percent = item.wear_percent orelse 0;
            row.wear_available = item.wear_percent != null;
            row.power_on_hours = item.power_on_hours orelse 0;
            row.power_on_hours_available = item.power_on_hours != null;
            row.read_errors_total = item.read_errors_total orelse 0;
            row.write_errors_total = item.write_errors_total orelse 0;
            row.error_counts_available = item.read_errors_total != null or item.write_errors_total != null;
            if (item.io_stats) |io| {
                const mib = 1024.0 * 1024.0;
                row.io_available = true;
                row.read_mib_s = @as(f64, @floatFromInt(io.read_bytes_per_sec)) / mib;
                row.write_mib_s = @as(f64, @floatFromInt(io.write_bytes_per_sec)) / mib;
                row.queue_depth = io.queue_depth;
                row.read_latency_ms = io.avg_read_latency_ms;
                row.write_latency_ms = io.avg_write_latency_ms;
                self.disk_io_available = true;
                self.disk_read_mib_s += row.read_mib_s;
                self.disk_write_mib_s += row.write_mib_s;
                self.disk_queue_depth += row.queue_depth;
                self.disk_read_latency_ms = @max(self.disk_read_latency_ms, row.read_latency_ms);
                self.disk_write_latency_ms = @max(self.disk_write_latency_ms, row.write_latency_ms);
            }
            if (row.error_counts_available) {
                self.disk_errors_available = true;
                self.disk_read_errors_total +|= row.read_errors_total;
                self.disk_write_errors_total +|= row.write_errors_total;
            }
            self.drive_health_rows[index] = row;
        }
    }

    pub fn applyDriversJson(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const parsed = try std.json.parseFromSlice(Envelope(DriversJson), allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.captureTopicMeta(6, parsed.value);
        const data = parsed.value.data;
        self.drivers_ready = true;
        setValueLabel(&self.driver_scan_buffer, data.scan_status);
        const groups = [_][]const DriverJson{
            data.network, data.bluetooth, data.audio,  data.input, data.display,
            data.storage, data.usb,       data.system, data.other,
        };
        var total: usize = 0;
        var attention: usize = 0;
        for (groups) |group| {
            total += group.len;
            for (group) |device| if (driverNeedsAttention(device.status)) {
                attention += 1;
            };
        }
        self.driver_total_count = saturatedU32(total);
        self.driver_attention_count = saturatedU32(attention);
        self.driver_count = 0;
        for ([_]bool{ true, false }) |attention_pass| {
            for (groups) |group| {
                for (group) |device| {
                    const needs_attention = driverNeedsAttention(device.status);
                    if (needs_attention != attention_pass or self.driver_count >= max_drivers) continue;
                    var row = DriverRow{ .id = @intCast(self.driver_count), .attention = needs_attention };
                    row.name_buffer.set(device.name);
                    row.category_buffer.set(device.category);
                    row.version_buffer.set(if (device.driver_version.len > 0) device.driver_version else "Not reported");
                    row.date_buffer.set(if (device.driver_date.len > 0) device.driver_date else "Not reported");
                    row.detail_buffer.set(device.extra);
                    setValueLabel(&row.status_buffer, device.status);
                    self.driver_rows[self.driver_count] = row;
                    self.driver_count += 1;
                }
            }
        }
        self.service_count = @min(data.services.len, max_services);
        for (data.services[0..self.service_count], 0..) |service, index| {
            var row = ServiceRow{ .id = @intCast(index), .running = service.is_running };
            row.name_buffer.set(service.name);
            row.display_buffer.set(service.display_name);
            self.service_rows[index] = row;
        }
    }
};

fn processRow(item: ProcessJson) ProcessRow {
    var row = ProcessRow{ .id = item.pid, .pid = item.pid };
    row.name_buffer.set(item.name);
    row.friendly_buffer.set(item.friendly_name);
    row.status_buffer.set(item.status);
    row.cpu_percent = item.cpu_percent;
    row.memory_mib = @as(f64, @floatFromInt(item.memory_bytes)) / (1024.0 * 1024.0);
    row.memory_percent = item.memory_percent;
    return row;
}

fn summaryText(bytes: []const u8, length: u32) []const u8 {
    return bytes[0..@min(bytes.len, @as(usize, @intCast(length)))];
}

fn Envelope(comptime Data: type) type {
    return struct {
        schema_version: u32 = 0,
        product_version: []const u8 = "unknown",
        target: []const u8 = "unknown target",
        topic: []const u8 = "unknown",
        sequence: u64 = 0,
        captured_unix_ms: u64 = 0,
        freshness_ms: u64 = 0,
        availability: []const u8 = "unavailable",
        provenance: []const u8 = "not reported",
        data: Data,
    };
}

const SystemJson = struct {
    os_name: []const u8 = "Unknown",
    os_version: []const u8 = "Unknown",
    hostname: []const u8 = "Unknown",
    cpu_model: []const u8 = "Unknown",
    architecture: []const u8 = "unknown",
    uptime_seconds: u64 = 0,
    kernel_version: []const u8 = "Unknown",
    manufacturer: ?[]const u8 = null,
    model: ?[]const u8 = null,
    bios_version: ?[]const u8 = null,
    hypervisor_present: ?bool = null,
};
const DisplayJson = struct {
    label: []const u8 = "Display",
    active: ?bool = null,
    connection: []const u8 = "Unknown",
    brightness_percent: ?u8 = null,
    physical_width_cm: ?u16 = null,
    physical_height_cm: ?u16 = null,
    source: []const u8 = "platform display provider",
};
const DisplaysJson = struct { displays: []const DisplayJson = &.{} };
const StaticDataJson = struct {
    system: SystemJson = .{},
    displays: DisplaysJson = .{},
    network_adapters: []const NetworkAdapterJson = &.{},
};
const NetworkAdapterJson = struct {
    name: []const u8 = "Adapter",
    description: ?[]const u8 = null,
    status: ?[]const u8 = null,
    link_speed_bps: ?u64 = null,
    hardware_interface: ?bool = null,
};
const WarningJson = struct {
    source: []const u8 = "Collector",
    message: []const u8 = "No detail reported",
    severity: []const u8 = "info",
};
const CapabilityJson = struct {
    id: []const u8 = "unknown",
    status: []const u8 = "unavailable",
    source: []const u8 = "not_collected",
    detail: ?[]const u8 = null,
};

const CpuJson = struct {
    total_usage: f64 = 0,
    per_core_usage: []const f64 = &.{},
    per_core_frequency: []const u64 = &.{},
    cpu_model: []const u8 = "",
    core_count: usize = 0,
    thread_count: usize = 0,
};
const MemoryModuleJson = struct {
    capacity_bytes: u64 = 0,
    configured_speed_mt_s: ?u32 = null,
    rated_speed_mt_s: ?u32 = null,
    manufacturer: ?[]const u8 = null,
    part_number: ?[]const u8 = null,
    locator: ?[]const u8 = null,
    memory_type: ?[]const u8 = null,
};
const MemoryJson = struct {
    available_bytes: u64 = 0,
    swap_used_bytes: u64 = 0,
    swap_total_bytes: u64 = 0,
    modules: []const MemoryModuleJson = &.{},
};
const InterfaceJson = struct {
    name: []const u8 = "",
    ip_addresses: []const []const u8 = &.{},
    mac_address: []const u8 = "",
    received_bytes: u64 = 0,
    transmitted_bytes: u64 = 0,
    download_rate: u64 = 0,
    upload_rate: u64 = 0,
    is_up: bool = false,
    operational_state: []const u8 = "unknown",
};
const NetworkJson = struct {
    interfaces: []const InterfaceJson = &.{},
    total_download_rate: u64 = 0,
    total_upload_rate: u64 = 0,
};
const ProcessJson = struct {
    pid: u32 = 0,
    name: []const u8 = "",
    friendly_name: []const u8 = "",
    cpu_percent: f64 = 0,
    memory_bytes: u64 = 0,
    memory_percent: f64 = 0,
    status: []const u8 = "unknown",
};
const ProcessesJson = struct {
    list: []const ProcessJson = &.{},
    total_count: usize = 0,
    total_threads: usize = 0,
};
const FastDataJson = struct {
    cpu: CpuJson,
    memory: MemoryJson,
    network: NetworkJson,
    processes: ProcessesJson,
};

const PartitionJson = struct {
    name: []const u8 = "",
    mount_point: []const u8 = "",
    filesystem: []const u8 = "",
    total_bytes: u64 = 0,
    used_bytes: u64 = 0,
    available_bytes: u64 = 0,
    is_removable: bool = false,
    disk_type: []const u8 = "unknown",
};
const DiskJson = struct { partitions: []const PartitionJson = &.{} };
const GpuAdapterJson = struct {
    name: []const u8 = "",
    driver_version: ?[]const u8 = null,
    status: ?[]const u8 = null,
    dedicated_memory_mb: ?u64 = null,
    utilization_percent: ?f64 = null,
    memory_used_mb: ?u64 = null,
    temperature_celsius: ?f64 = null,
    current_resolution: ?[]const u8 = null,
    refresh_rate_hz: ?u32 = null,
    telemetry_available: bool = false,
    source: []const u8 = "platform inventory",
};
const GpuJson = struct { adapters: []const GpuAdapterJson = &.{} };
const SensorJson = struct {
    label: []const u8 = "",
    temperature: f64 = 0,
    critical: ?f64 = null,
    kind: []const u8 = "other",
    source: []const u8 = "platform sensor",
};
const FanJson = struct {
    label: []const u8 = "",
    rpm: u64 = 0,
    source: []const u8 = "platform sensor",
};
const ThermalsJson = struct {
    cpu_temp: ?f64 = null,
    gpu_temp: ?f64 = null,
    sensors: []const SensorJson = &.{},
    fans: []const FanJson = &.{},
    battery: ?BatteryJson = null,
    power_source: []const u8 = "unknown",
};
const BatteryJson = struct {
    percent: f64 = 0,
    is_charging: bool = false,
    is_on_ac: bool = false,
    time_remaining: ?[]const u8 = null,
    full_charged_capacity_mwh: ?u64 = null,
    design_voltage_mv: ?u64 = null,
    cycle_count: ?u32 = null,
    provider_status: ?[]const u8 = null,
};
const SlowDataJson = struct {
    disk: DiskJson,
    gpu: GpuJson,
    thermals: ThermalsJson,
};

const ConnectionJson = struct {
    protocol: []const u8 = "tcp",
    local_addr: []const u8 = "*",
    local_port: u16 = 0,
    remote_addr: []const u8 = "*",
    remote_port: u16 = 0,
    state: std.json.Value,
    pid: ?u32 = null,
    process_name: ?[]const u8 = null,
};
const MediumJson = struct {
    active_connections: []const ConnectionJson = &.{},
    listening_ports: []const ConnectionJson = &.{},
};
const ConnectivityJson = struct {
    reachable: bool = false,
    latency_ms: ?f64 = null,
    target: []const u8 = "Not reported",
    @"error": ?[]const u8 = null,
};
const DnsJson = struct {
    resolved: bool = false,
    resolution_ms: ?f64 = null,
    domain: []const u8 = "Not reported",
    resolved_ip: ?[]const u8 = null,
    @"error": ?[]const u8 = null,
};
const DiagnosticsJson = struct {
    gateway: ConnectivityJson,
    dns: DnsJson,
    internet: ConnectivityJson,
};
const DriveHealthJson = struct {
    model: []const u8 = "",
    media_type: []const u8 = "unknown",
    health_status: []const u8 = "unknown",
    temperature_celsius: ?f64 = null,
    power_on_hours: ?u64 = null,
    wear_percent: ?u8 = null,
    read_errors_total: ?u64 = null,
    write_errors_total: ?u64 = null,
    io_stats: ?DiskIoJson = null,
    health_source: []const u8 = "platform storage provider",
};
const DiskIoJson = struct {
    read_bytes_per_sec: u64 = 0,
    write_bytes_per_sec: u64 = 0,
    queue_depth: f64 = 0,
    avg_read_latency_ms: f64 = 0,
    avg_write_latency_ms: f64 = 0,
};
const HealthJson = struct { drives: []const DriveHealthJson = &.{} };
const DriverJson = struct {
    name: []const u8 = "",
    driver_version: []const u8 = "",
    driver_date: []const u8 = "",
    status: std.json.Value,
    category: []const u8 = "other",
    extra: []const u8 = "",
};
const DriversJson = struct {
    network: []const DriverJson = &.{},
    bluetooth: []const DriverJson = &.{},
    audio: []const DriverJson = &.{},
    input: []const DriverJson = &.{},
    display: []const DriverJson = &.{},
    storage: []const DriverJson = &.{},
    usb: []const DriverJson = &.{},
    system: []const DriverJson = &.{},
    other: []const DriverJson = &.{},
    services: []const ServiceJson = &.{},
    scan_status: std.json.Value,
};
const ServiceJson = struct {
    name: []const u8 = "",
    display_name: []const u8 = "",
    is_running: bool = false,
};

fn saturatedU32(value: anytype) u32 {
    return std.math.cast(u32, value) orelse std.math.maxInt(u32);
}

fn driverNeedsAttention(value: std.json.Value) bool {
    return switch (value) {
        .string => |label| std.mem.eql(u8, label, "disabled") or std.mem.eql(u8, label, "not_found"),
        .object => |object| object.contains("degraded") or object.contains("error"),
        else => false,
    };
}

fn setValueLabel(buffer: anytype, value: std.json.Value) void {
    switch (value) {
        .string => |label| buffer.set(label),
        .object => |object| {
            var iterator = object.iterator();
            if (iterator.next()) |entry| {
                switch (entry.value_ptr.*) {
                    .string => |detail| {
                        var scratch: [192]u8 = undefined;
                        const label = std.fmt.bufPrint(&scratch, "{s}: {s}", .{ entry.key_ptr.*, detail }) catch entry.key_ptr.*;
                        buffer.set(label);
                    },
                    else => buffer.set(entry.key_ptr.*),
                }
            } else buffer.set("unknown");
        },
        else => buffer.set("unknown"),
    }
}

test "fast topic projection is bounded and copies borrowed strings" {
    const fixture =
        \\{"sequence":4,"data":{"cpu":{"total_usage":25.0,"per_core_usage":[10.0,20.0],"per_core_frequency":[3200,3300],"cpu_model":"Test CPU","core_count":1,"thread_count":2},"memory":{"available_bytes":8589934592,"swap_used_bytes":0,"swap_total_bytes":0,"modules":[]},"network":{"interfaces":[],"total_download_rate":1024,"total_upload_rate":2048},"processes":{"list":[{"pid":7,"name":"test.exe","friendly_name":"Test","cpu_percent":4.5,"memory_bytes":1048576,"memory_percent":1.0,"status":"Run"}],"total_count":1,"total_threads":3}}}
    ;
    var projection = Projection{};
    try projection.applyFastJson(std.testing.allocator, fixture);
    try std.testing.expect(projection.fast_ready);
    try std.testing.expectEqualStrings("Test CPU", projection.cpuModel());
    try std.testing.expectEqual(@as(usize, 2), projection.cpuCores().len);
    try std.testing.expectEqual(@as(u64, 3300), projection.cpuCores()[1].frequency_mhz);
    try std.testing.expectEqualStrings("Test", projection.processes()[0].friendlyName());
}

test "typed process summary preserves bounded live process parity" {
    var summary = engine.ProcessSummary{
        .sequence = 9,
        .captured_unix_ms = 1_777_777_777_000,
        .total_count = 321,
        .total_threads = 4_567,
        .row_count = 1,
    };
    const name = "native.exe";
    const friendly = "Native Monitor";
    const status = "Run";
    @memcpy(summary.rows[0].name[0..name.len], name);
    @memcpy(summary.rows[0].friendly_name[0..friendly.len], friendly);
    @memcpy(summary.rows[0].status[0..status.len], status);
    summary.rows[0].pid = 42;
    summary.rows[0].cpu_percent = 7.5;
    summary.rows[0].memory_bytes = 128 * 1024 * 1024;
    summary.rows[0].memory_percent = 0.4;
    summary.rows[0].name_len = @intCast(name.len);
    summary.rows[0].friendly_name_len = @intCast(friendly.len);
    summary.rows[0].status_len = @intCast(status.len);

    var projection = Projection{};
    projection.applyProcessSummary(summary);

    try std.testing.expectEqual(@as(u32, 321), projection.process_total_count);
    try std.testing.expectEqual(@as(u32, 4_567), projection.process_total_threads);
    try std.testing.expectEqual(@as(usize, 1), projection.processes().len);
    try std.testing.expectEqual(@as(u32, 42), projection.processes()[0].pid);
    try std.testing.expectEqualStrings(name, projection.processes()[0].name());
    try std.testing.expectEqualStrings(friendly, projection.processes()[0].friendlyName());
    try std.testing.expectEqualStrings(status, projection.processes()[0].status());
    try std.testing.expectEqual(@as(f64, 128), projection.processes()[0].memory_mib);
    try std.testing.expectEqual(@as(u64, 9), projection.topicMeta(1).sequence);
}

test "process values stay live while rank order reconciles every thirty samples" {
    const first =
        \\{"sequence":1,"data":{"cpu":{},"memory":{},"network":{},"processes":{"list":[{"pid":7,"name":"alpha.exe","friendly_name":"Alpha","cpu_percent":10,"memory_bytes":1048576,"status":"Run"},{"pid":8,"name":"beta.exe","friendly_name":"Beta","cpu_percent":9,"memory_bytes":2097152,"status":"Run"}],"total_count":2}}}
    ;
    const second =
        \\{"sequence":2,"data":{"cpu":{},"memory":{},"network":{},"processes":{"list":[{"pid":8,"name":"beta.exe","friendly_name":"Beta","cpu_percent":11,"memory_bytes":2097152,"status":"Run"},{"pid":7,"name":"alpha.exe","friendly_name":"Alpha","cpu_percent":5,"memory_bytes":1048576,"status":"Run"}],"total_count":2}}}
    ;
    const sixth =
        \\{"sequence":31,"data":{"cpu":{},"memory":{},"network":{},"processes":{"list":[{"pid":8,"name":"beta.exe","friendly_name":"Beta","cpu_percent":12,"memory_bytes":2097152,"status":"Run"},{"pid":7,"name":"alpha.exe","friendly_name":"Alpha","cpu_percent":4,"memory_bytes":1048576,"status":"Run"}],"total_count":2}}}
    ;

    var projection = Projection{};
    try projection.applyFastJson(std.testing.allocator, first);
    try std.testing.expectEqual(@as(u32, 7), projection.processes()[0].pid);
    try std.testing.expect(!projection.process_values_warmed);

    try projection.applyFastJson(std.testing.allocator, second);
    try std.testing.expect(projection.process_values_warmed);
    try std.testing.expectEqual(@as(u32, 7), projection.processes()[0].pid);
    try std.testing.expectEqual(@as(f64, 5), projection.processes()[0].cpu_percent);
    try std.testing.expectEqual(@as(u32, 8), projection.processes()[1].pid);

    try projection.applyFastJson(std.testing.allocator, sixth);
    try std.testing.expectEqual(@as(u32, 8), projection.processes()[0].pid);
    try std.testing.expectEqual(@as(f64, 12), projection.processes()[0].cpu_percent);
}

test "static warnings and capability topics preserve explicit provenance" {
    const static_fixture =
        \\{"sequence":1,"data":{"system":{"os_name":"Windows","os_version":"11","hostname":"ALIEN","cpu_model":"Test CPU","architecture":"x86_64","uptime_seconds":7200,"kernel_version":"10.0","manufacturer":"Dell","model":"Alienware","bios_version":"1.2.3","hypervisor_present":false},"displays":{"displays":[{"label":"Display 1","active":true,"connection":"DisplayPort","brightness_percent":80,"physical_width_cm":60,"physical_height_cm":34,"source":"fixture"}]}}}
    ;
    const warning_fixture =
        \\{"sequence":1,"data":[{"source":"Thermals","message":"Provider denied access","severity":"warning"}]}
    ;
    const capability_fixture =
        \\{"sequence":1,"data":[{"id":"thermal.cpu","status":"permission_denied","source":"WMI","detail":"Run the provider with access"}]}
    ;
    var value = Projection{};
    try value.applyStaticJson(std.testing.allocator, static_fixture);
    try value.applyWarningsJson(std.testing.allocator, warning_fixture);
    try value.applyCapabilitiesJson(std.testing.allocator, capability_fixture);
    try std.testing.expect(value.static_ready);
    try std.testing.expectEqualStrings("Dell", value.manufacturer());
    try std.testing.expectEqualStrings("DisplayPort", value.displays()[0].connection());
    try std.testing.expectEqualStrings("Provider denied access", value.warnings()[0].message());
    try std.testing.expectEqualStrings("permission_denied", value.capabilities()[0].status());
    try std.testing.expectEqualStrings("Run the provider with access", value.capabilities()[0].detail());
}

test "topic metadata disk reliability and driver services remain explicit" {
    const health_fixture =
        \\{"schema_version":1,"target":"x86_64-windows","topic":"health","sequence":9,"captured_unix_ms":1777777777000,"freshness_ms":0,"availability":"available","provenance":"platform SMART and reliability provider","data":{"drives":[{"model":"Fixture NVMe","media_type":"nvme","health_status":"healthy","temperature_celsius":42.5,"read_errors_total":2,"write_errors_total":3,"io_stats":{"read_bytes_per_sec":1048576,"write_bytes_per_sec":2097152,"queue_depth":1.25,"avg_read_latency_ms":0.5,"avg_write_latency_ms":0.75},"health_source":"fixture"}]}}
    ;
    const drivers_fixture =
        \\{"schema_version":1,"target":"x86_64-windows","topic":"drivers","sequence":4,"captured_unix_ms":1777777778000,"availability":"available","provenance":"SetupAPI","data":{"network":[],"bluetooth":[],"audio":[],"input":[],"display":[],"storage":[],"usb":[],"system":[],"other":[],"services":[{"name":"FixtureSvc","display_name":"Fixture Service","is_running":true}],"scan_status":"success"}}
    ;

    var value = Projection{};
    try value.applyHealthJson(std.testing.allocator, health_fixture);
    try value.applyDriversJson(std.testing.allocator, drivers_fixture);

    try std.testing.expect(value.disk_io_available);
    try std.testing.expectEqual(@as(f64, 1), value.disk_read_mib_s);
    try std.testing.expectEqual(@as(f64, 2), value.disk_write_mib_s);
    try std.testing.expectEqual(@as(u64, 2), value.disk_read_errors_total);
    try std.testing.expectEqual(@as(u64, 3), value.disk_write_errors_total);
    try std.testing.expectEqual(@as(usize, 1), value.services().len);
    try std.testing.expectEqualStrings("Fixture Service", value.services()[0].displayName());
    try std.testing.expectEqualStrings("running", value.services()[0].state());

    const health_meta = value.topicMeta(5);
    try std.testing.expect(health_meta.ready);
    try std.testing.expectEqual(@as(u64, 9), health_meta.sequence);
    try std.testing.expectEqualStrings("health", health_meta.topic());
    try std.testing.expectEqualStrings("platform SMART and reliability provider", health_meta.provenance());
    try std.testing.expectEqualStrings("x86_64-windows", health_meta.target());
}
