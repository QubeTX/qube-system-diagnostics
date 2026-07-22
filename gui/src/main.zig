const std = @import("std");
const builtin = @import("builtin");
const runner = @import("runner");
const native_sdk = @import("native_sdk");
const engine = @import("engine.zig");
const projection = @import("projection.zig");
const settings = @import("settings.zig");
const window_visibility = @import("platform/window_visibility.zig");

pub const panic = std.debug.FullPanic(native_sdk.debug.capturePanic);

const canvas = native_sdk.canvas;
const geometry = native_sdk.geometry;

const canvas_label = "main-canvas";
const window_width: f32 = 1180;
const window_height: f32 = 760;
const refresh_timer_key: u64 = 300;
const export_timer_key: u64 = 301;
// The engine and visible presentation both advance once per second. The
// downstream reference renderer retains stable base fragments, glyph coverage,
// and damage regions so one live value change does not repaint the 0.9 MP
// window. Latest-only topic reads still discard superseded samples and can
// never build a renderer backlog.
const visible_refresh_ms: u64 = 1000;
const hidden_refresh_ms: u64 = 30000;
const fast_summary_stale_after_ms: u64 = 2500;
const history_sample_count: usize = 60;
const primary_process_row_count: usize = 8;
const tray_supported = builtin.os.tag == .windows or builtin.os.tag == .macos;
const makira_font_id: canvas.FontId = canvas.min_registered_font_id;
const plex_mono_font_id: canvas.FontId = canvas.min_registered_font_id + 1;

pub const ProcessSort = enum(u32) {
    cpu = 0,
    memory = 1,
    pid = 2,
    name = 3,
};

const app_permissions = [_][]const u8{ native_sdk.security.permission_command, native_sdk.security.permission_view };
const shell_views = [_]native_sdk.ShellView{
    .{ .label = canvas_label, .kind = .gpu_surface, .fill = true, .role = "SD-300 system monitor", .accessibility_label = "Live SD-300 system diagnostics", .gpu_backend = .metal, .gpu_pixel_format = .bgra8_unorm, .gpu_present_mode = .timer, .gpu_alpha_mode = .@"opaque", .gpu_color_space = .srgb, .gpu_vsync = true },
};
var active_engine: ?*engine.Runtime = null;
var active_app_state: ?*NativeApp = null;
var external_open_pending = std.atomic.Value(bool).init(false);
var startup_should_show = true;

pub const Msg = union(enum) {
    refresh_now,
    refresh_tick: native_sdk.EffectTimer,
    select_overview,
    select_cpu,
    select_memory,
    select_disk,
    select_gpu,
    select_network,
    select_processes,
    show_cpu_processes,
    show_memory_processes,
    process_filter_edit: canvas.TextInputEvent,
    connection_filter_edit: canvas.TextInputEvent,
    driver_filter_edit: canvas.TextInputEvent,
    toggle_driver_attention,
    toggle_process_rows,
    sort_process_cpu,
    sort_process_memory,
    sort_process_pid,
    sort_process_name,
    select_thermals,
    select_drivers,
    select_settings,
    toggle_audience_mode,
    toggle_temperature_unit,
    toggle_tray,
    toggle_launch_at_login,
    toggle_reduced_motion,
    density_compact,
    density_balanced,
    density_comfortable,
    export_redacted_snapshot,
    export_capabilities,
    export_poll: native_sdk.EffectTimer,
    content_scrolled: canvas.ScrollState,
    scan_drivers,
    open_window,
    quit_app,

    pub const view_unbound = .{ "refresh_tick", "export_poll", "open_window", "quit_app" };
};

pub const Model = struct {
    engine_ready: bool = false,
    fast_summary_seen: bool = false,
    fast_summary_failed: bool = false,
    fast_summary_stale: bool = false,
    fast_summary_last_advance_ms: u64 = 0,
    sequence: u64 = 0,
    cpu_percent: f64 = 0,
    memory_percent: f64 = 0,
    memory_used_gib: f64 = 0,
    memory_total_gib: f64 = 0,
    logical_processors: u32 = 0,
    warning_count: u32 = 0,
    overview_topic_meta: projection.TopicMeta = .{},
    cpu_history: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    memory_history: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    swap_history: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    network_download_history: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    network_upload_history: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    gpu_history: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    temperature_history_celsius: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    temperature_history_fahrenheit: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    disk_read_history: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    disk_write_history: [history_sample_count]f64 = [_]f64{0} ** history_sample_count,
    clock: native_sdk.Clock = .system,
    active_section: u8 = 0,
    show_all_processes: bool = false,
    process_sort: ProcessSort = .cpu,
    process_filter_buffer: canvas.TextBuffer(64) = .{},
    filtered_process_rows: [projection.max_processes]projection.ProcessRow = [_]projection.ProcessRow{.{}} ** projection.max_processes,
    filtered_process_count: usize = 0,
    connection_filter_buffer: canvas.TextBuffer(64) = .{},
    filtered_connection_rows: [projection.max_connections]projection.ConnectionRow = [_]projection.ConnectionRow{.{}} ** projection.max_connections,
    filtered_connection_count: usize = 0,
    driver_filter_buffer: canvas.TextBuffer(64) = .{},
    filtered_driver_rows: [projection.max_drivers]projection.DriverRow = [_]projection.DriverRow{.{}} ** projection.max_drivers,
    filtered_driver_count: usize = 0,
    driver_attention_only: bool = false,
    scroll_top: f64 = 0,
    detail: projection.Projection = .{},
    // Presentation-active: the window is on screen OR minimized — i.e. NOT
    // policy-hidden (close-to-tray). A minimized window stays presentation-active
    // on purpose so it keeps the 1 s foreground cadence and engine profile and
    // restore is instantly fresh; only a tray-hidden window is inactive (30 s).
    // Every write derives this from `!mainWindowPolicyHidden()` (or `true` on an
    // explicit open), never from raw visibility.
    window_visible: bool = true,
    audience_mode: settings.AudienceMode = .user,
    temperature_unit: settings.TemperatureUnit = .celsius,
    tray_enabled: bool = false,
    // Whether a tray status item actually exists for THIS running session,
    // fixed once at startup from the effective tray decision (tray_supported AND
    // the persisted `tray_enabled`) that also gates status-item creation. This
    // is deliberately distinct from `tray_enabled`: that field is the user's
    // persisted preference for the NEXT launch and drives settings display and
    // persistence, whereas a mid-session tray toggle only takes effect on
    // relaunch (RESTART REQUIRED). Any decision that turns on the tray icon
    // truly being present — chiefly whether a policy-hidden close may keep the
    // process alive instead of quitting — must read this, never the preference.
    tray_session_active: bool = false,
    launch_at_login: bool = false,
    reduced_motion: bool = true,
    chart_density: settings.ChartDensity = .balanced,
    last_monitor_section: u8 = 0,
    settings_restart_required: bool = false,
    export_pending: bool = false,
    status_buffer: canvas.TextBuffer(384) = canvas.TextBuffer(384).init("Loading the SD-300 engine…"),
    tray_cpu_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("CPU · waiting for live data"),
    tray_memory_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Memory · waiting for live data"),
    tray_gpu_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("GPU · unavailable"),
    tray_storage_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Storage · waiting for inventory"),
    tray_health_buffer: canvas.TextBuffer(64) = canvas.TextBuffer(64).init("Disk health · scanning"),

    pub const view_unbound = .{
        "window_visible",
        "tray_session_active",
        "show_all_processes",
        "process_sort",
        "process_filter_buffer",
        "filtered_process_rows",
        "filtered_process_count",
        "connection_filter_buffer",
        "filtered_connection_rows",
        "filtered_connection_count",
        "driver_filter_buffer",
        "filtered_driver_rows",
        "filtered_driver_count",
        "audience_mode",
        "temperature_unit",
        "chart_density",
        "status_buffer",
        "tray_cpu_buffer",
        "tray_memory_buffer",
        "tray_gpu_buffer",
        "tray_storage_buffer",
        "tray_health_buffer",
        "last_monitor_section",
        "clock",
        "engine_ready",
        "fast_summary_failed",
        "fast_summary_stale",
        "fast_summary_last_advance_ms",
        "overview_topic_meta",
    };

    pub fn status(model: *const Model) []const u8 {
        return model.status_buffer.text();
    }

    pub fn cpuModel(model: *const Model) []const u8 {
        return model.detail.cpuModel();
    }
    pub fn driverScanStatus(model: *const Model) []const u8 {
        return model.detail.driverScanStatus();
    }
    pub fn cpuCores(model: *const Model) []const projection.CpuCoreRow {
        return model.detail.cpuCores();
    }
    pub fn memoryModules(model: *const Model) []const projection.MemoryModuleRow {
        return model.detail.memoryModules();
    }
    pub fn disks(model: *const Model) []const projection.DiskRow {
        return model.detail.disks();
    }
    pub fn driveHealth(model: *const Model) []const projection.DriveHealthRow {
        return model.detail.driveHealth();
    }
    pub fn displays(model: *const Model) []const projection.DisplayRow {
        return model.detail.displays();
    }
    pub fn warnings(model: *const Model) []const projection.WarningRow {
        return model.detail.warnings();
    }
    pub fn capabilities(model: *const Model) []const projection.CapabilityRow {
        return model.detail.capabilities();
    }
    pub fn networkAdapters(model: *const Model) []const projection.NetworkAdapterRow {
        return model.detail.networkAdapters();
    }
    pub fn services(model: *const Model) []const projection.ServiceRow {
        return model.detail.services();
    }
    pub fn osName(model: *const Model) []const u8 {
        return model.detail.osName();
    }
    pub fn osVersion(model: *const Model) []const u8 {
        return model.detail.osVersion();
    }
    pub fn hostname(model: *const Model) []const u8 {
        return model.detail.hostname();
    }
    pub fn systemCpu(model: *const Model) []const u8 {
        return model.detail.systemCpu();
    }
    pub fn architecture(model: *const Model) []const u8 {
        return model.detail.architecture();
    }
    pub fn kernel(model: *const Model) []const u8 {
        return model.detail.kernel();
    }
    pub fn manufacturer(model: *const Model) []const u8 {
        return model.detail.manufacturer();
    }
    pub fn systemModel(model: *const Model) []const u8 {
        return model.detail.systemModel();
    }
    pub fn bios(model: *const Model) []const u8 {
        return model.detail.bios();
    }
    pub fn uptimeHours(model: *const Model) f64 {
        return @as(f64, @floatFromInt(model.detail.uptime_seconds)) / 3600.0;
    }
    pub fn gpus(model: *const Model) []const projection.GpuRow {
        return model.detail.gpus();
    }
    pub fn interfaces(model: *const Model) []const projection.InterfaceRow {
        return model.detail.interfaces();
    }
    pub fn connections(model: *const Model) []const projection.ConnectionRow {
        return if (model.connection_filter_buffer.text().len == 0)
            model.detail.connections()
        else
            model.filtered_connection_rows[0..model.filtered_connection_count];
    }
    pub fn connectionFilter(model: *const Model) []const u8 {
        return model.connection_filter_buffer.text();
    }
    pub fn connectionMatchCount(model: *const Model) usize {
        return if (model.connection_filter_buffer.text().len == 0)
            model.detail.connection_count
        else
            model.filtered_connection_count;
    }
    pub fn processes(model: *const Model) []const projection.ProcessRow {
        const rows = if (model.process_filter_buffer.text().len == 0)
            model.detail.processes()
        else
            model.filtered_process_rows[0..model.filtered_process_count];
        if (model.show_all_processes) return rows;
        return rows[0..@min(rows.len, primary_process_row_count)];
    }
    pub fn processFilter(model: *const Model) []const u8 {
        return model.process_filter_buffer.text();
    }
    pub fn processMatchCount(model: *const Model) usize {
        return if (model.process_filter_buffer.text().len == 0)
            model.detail.process_count
        else
            model.filtered_process_count;
    }
    pub fn visibleProcessCount(model: *const Model) usize {
        return model.processes().len;
    }
    pub fn processToggleLabel(model: *const Model) []const u8 {
        return if (model.show_all_processes) "Show primary 8" else "Show all matches";
    }
    pub fn processCpuSort(model: *const Model) bool {
        return model.process_sort == .cpu;
    }
    pub fn processMemorySort(model: *const Model) bool {
        return model.process_sort == .memory;
    }
    pub fn processPidSort(model: *const Model) bool {
        return model.process_sort == .pid;
    }
    pub fn processNameSort(model: *const Model) bool {
        return model.process_sort == .name;
    }
    pub fn hypervisorState(model: *const Model) []const u8 {
        if (!model.detail.hypervisor_available) return "unknown";
        return if (model.detail.hypervisor_present) "present" else "not detected";
    }
    pub fn primaryGpuTelemetryAvailable(model: *const Model) bool {
        return model.detail.gpu_count > 0 and model.detail.gpu_rows[0].telemetry_available;
    }
    pub fn thermalHistoryAvailable(model: *const Model) bool {
        return model.detail.cpu_temperature_available or model.detail.gpu_temperature_available;
    }
    pub fn summaryLive(model: *const Model) bool {
        return model.fast_summary_seen and model.engine_ready and !model.fast_summary_failed and !model.fast_summary_stale;
    }
    pub fn summaryStale(model: *const Model) bool {
        return model.fast_summary_seen and model.fast_summary_stale and !model.fast_summary_failed;
    }
    pub fn summaryFailed(model: *const Model) bool {
        return model.fast_summary_failed;
    }
    pub fn cpuAssessment(model: *const Model) []const u8 {
        if (model.cpu_percent >= 90) return "CPU demand is critical right now; open Processes to identify sustained consumers.";
        if (model.cpu_percent >= 70) return "CPU demand is high. Short bursts are normal; sustained load can reduce responsiveness.";
        if (model.cpu_percent >= 35) return "CPU demand is moderate and leaves working headroom.";
        return "CPU demand is light; the processor has substantial headroom.";
    }
    pub fn memoryAssessment(model: *const Model) []const u8 {
        if (model.memory_percent >= 90) return "Physical memory pressure is critical; paging and application slowdown are likely.";
        if (model.memory_percent >= 75) return "Memory pressure is elevated. Processes and swap usage can explain where capacity went.";
        if (model.memory_percent >= 50) return "Memory use is moderate with usable capacity remaining.";
        return "Memory pressure is low and physical capacity is readily available.";
    }
    pub fn sensors(model: *const Model) []const projection.SensorRow {
        return model.detail.sensors();
    }
    pub fn fans(model: *const Model) []const projection.FanRow {
        return model.detail.fans();
    }
    pub fn drivers(model: *const Model) []const projection.DriverRow {
        return if (model.driver_filter_buffer.text().len == 0 and !model.driver_attention_only)
            model.detail.drivers()
        else
            model.filtered_driver_rows[0..model.filtered_driver_count];
    }
    pub fn driverFilter(model: *const Model) []const u8 {
        return model.driver_filter_buffer.text();
    }
    pub fn driverMatchCount(model: *const Model) usize {
        return if (model.driver_filter_buffer.text().len == 0 and !model.driver_attention_only)
            model.detail.driver_count
        else
            model.filtered_driver_count;
    }
    pub fn driverAttentionFilterLabel(model: *const Model) []const u8 {
        return if (model.driver_attention_only) "Attention only · show all" else "Show attention only";
    }
    pub fn technicianMode(model: *const Model) bool {
        return model.audience_mode == .technician;
    }
    pub fn userMode(model: *const Model) bool {
        return model.audience_mode == .user;
    }
    pub fn fahrenheitMode(model: *const Model) bool {
        return model.temperature_unit == .fahrenheit;
    }
    pub fn compactDensity(model: *const Model) bool {
        return model.chart_density == .compact;
    }
    pub fn balancedDensity(model: *const Model) bool {
        return model.chart_density == .balanced;
    }
    pub fn comfortableDensity(model: *const Model) bool {
        return model.chart_density == .comfortable;
    }
    pub fn trayAvailable(model: *const Model) bool {
        _ = model;
        return tray_supported;
    }
};

pub const Effects = native_sdk.Effects(Msg);

pub fn update(model: *Model, msg: Msg, fx: *Effects) void {
    switch (msg) {
        .refresh_now => sampleEngine(model),
        .refresh_tick => |timer| {
            if (timer.outcome == .fired) {
                // Quit on a policy-hidden close only when NO tray status item
                // exists this session. This must read the startup-effective
                // presence, not the persisted `tray_enabled` preference: a
                // mid-session toggle is RESTART REQUIRED, so enabling tray now
                // must not strand a hidden, icon-less process, and disabling it
                // now must not kill a still-live tray icon.
                if (shouldQuitForHiddenWindow(model.tray_session_active, window_visibility.mainWindowPolicyHidden())) {
                    fx.quitApp();
                    return;
                }
                // Track PRESENTATION by policy-hidden (close-to-tray), not raw
                // visibility. A minimized (iconic) window is a transient
                // background state, not a request to stop collecting: restoring
                // it from the taskbar emits no message, so dropping to the 30 s
                // hidden cadence would show up to 30 s of stale data on restore.
                // Treating iconic as foreground keeps the 1 s sampling cadence
                // and the foreground engine profile, so restore is instantly
                // fresh. This costs foreground-level CPU (~1.6% of one logical
                // core) while minimized, which is acceptable for a transient
                // state; a genuine tray-hidden window stays policy-hidden and
                // keeps the cheaper 30 s cadence.
                const visibility_changed = applyWindowVisibility(model, !window_visibility.mainWindowPolicyHidden());
                sampleEngine(model);
                if (visibility_changed) scheduleRefresh(fx, model.window_visible);
            }
        },
        .select_overview => selectSection(model, 0),
        .select_cpu => selectSection(model, 1),
        .select_memory => selectSection(model, 2),
        .select_disk => selectSection(model, 3),
        .select_gpu => selectSection(model, 4),
        .select_network => selectSection(model, 5),
        .select_processes => selectSection(model, 6),
        .show_cpu_processes => {
            selectSection(model, 6);
            setProcessSort(model, .cpu);
        },
        .show_memory_processes => {
            selectSection(model, 6);
            setProcessSort(model, .memory);
        },
        .process_filter_edit => |edit| {
            model.process_filter_buffer.apply(edit);
            model.show_all_processes = true;
            rebuildProcessFilter(model);
        },
        .connection_filter_edit => |edit| {
            model.connection_filter_buffer.apply(edit);
            rebuildConnectionFilter(model);
        },
        .driver_filter_edit => |edit| {
            model.driver_filter_buffer.apply(edit);
            rebuildDriverFilter(model);
        },
        .toggle_driver_attention => {
            model.driver_attention_only = !model.driver_attention_only;
            rebuildDriverFilter(model);
        },
        .toggle_process_rows => model.show_all_processes = !model.show_all_processes,
        .sort_process_cpu => setProcessSort(model, .cpu),
        .sort_process_memory => setProcessSort(model, .memory),
        .sort_process_pid => setProcessSort(model, .pid),
        .sort_process_name => setProcessSort(model, .name),
        .select_thermals => selectSection(model, 7),
        .select_drivers => selectSection(model, 8),
        .select_settings => {
            model.scroll_top = 0;
            model.active_section = 9;
            if (active_engine) |runtime| runtime.setView(model.window_visible, 9) catch {};
            sampleEngine(model);
        },
        .toggle_audience_mode => {
            model.audience_mode = if (model.audience_mode == .user) .technician else .user;
            persistSettings(model);
        },
        .toggle_temperature_unit => {
            model.temperature_unit = if (model.temperature_unit == .celsius) .fahrenheit else .celsius;
            persistSettings(model);
        },
        .toggle_tray => {
            if (comptime tray_supported) {
                const previous = model.tray_enabled;
                model.tray_enabled = !previous;
                if (model.launch_at_login) {
                    const runtime = active_engine orelse {
                        model.tray_enabled = previous;
                        model.status_buffer.set("Tray could not change while launch-at-login registration is unavailable.");
                        return;
                    };
                    runtime.setLaunchAtLogin(true, model.tray_enabled) catch {
                        model.tray_enabled = previous;
                        model.status_buffer.set("Tray change was not saved because startup registration could not be updated safely.");
                        return;
                    };
                }
                model.settings_restart_required = true;
                persistSettings(model);
            } else {
                model.status_buffer.set("Tray is unavailable on Linux in Native SDK 0.5.4.");
            }
        },
        .toggle_launch_at_login => {
            const desired = !model.launch_at_login;
            const runtime = active_engine orelse {
                model.status_buffer.set("Launch-at-login is unavailable because the GUI engine is not loaded.");
                return;
            };
            runtime.setLaunchAtLogin(desired, tray_supported and model.tray_enabled) catch {
                model.status_buffer.set("Launch-at-login was not changed; an ambiguous or inaccessible registration was preserved.");
                return;
            };
            model.launch_at_login = desired;
            persistSettings(model);
        },
        .toggle_reduced_motion => {
            model.reduced_motion = !model.reduced_motion;
            persistSettings(model);
        },
        .density_compact => {
            model.chart_density = .compact;
            persistSettings(model);
        },
        .density_balanced => {
            model.chart_density = .balanced;
            persistSettings(model);
        },
        .density_comfortable => {
            model.chart_density = .comfortable;
            persistSettings(model);
        },
        .export_redacted_snapshot => requestExport(model, fx, .redacted_snapshot),
        .export_capabilities => requestExport(model, fx, .capabilities),
        .export_poll => |timer| {
            if (timer.outcome == .fired and model.export_pending) pollExport(model, fx);
        },
        .content_scrolled => |state| model.scroll_top = state.offset,
        .scan_drivers => requestDriverScan(model),
        .open_window => {
            _ = applyWindowVisibility(model, true);
            sampleEngine(model);
            scheduleRefresh(fx, true);
            // The SDK effect keeps its runtime window table coherent. The
            // platform call is an idempotent safety net for a host-level
            // close-policy hide, whose frame event may reach the model after
            // a singleton or tray Open command is already being handled.
            window_visibility.showMainWindow();
            fx.showWindow("main");
        },
        .quit_app => fx.quitApp(),
    }
}

pub fn onCommand(name: []const u8) ?Msg {
    if (std.mem.eql(u8, name, "app.open")) return .open_window;
    if (std.mem.eql(u8, name, "app.quit")) return .quit_app;
    return null;
}

pub fn initEffects(model: *Model, fx: *Effects) void {
    // Install the Windows singleton-open message route from the actual SDK UI
    // thread before an external instance is allowed to signal this window.
    window_visibility.installOpenMessageRoute();
    // Native SDK restores window geometry and may also restore a previous
    // policy-hidden state. Startup intent wins: only the explicit managed
    // `--startup --hidden` route may remain hidden; an ordinary launch must
    // always put the monitor on screen.
    if (startup_should_show) {
        window_visibility.showMainWindow();
    } else {
        window_visibility.hideMainWindow();
    }
    // `window_visible` means "presentation-active" — on screen OR minimized,
    // i.e. NOT policy-hidden — so a minimized window keeps the foreground
    // cadence and engine profile (see the refresh_tick handler). At startup the
    // window is freshly shown or freshly policy-hidden and never iconic, so both
    // predicates agree here; using policy-hidden keeps the field's meaning
    // singular across every site that reads it.
    _ = applyWindowVisibility(model, !window_visibility.mainWindowPolicyHidden());
    if (active_engine) |runtime| runtime.setView(model.window_visible, model.active_section) catch {};
    sampleEngine(model);
    scheduleRefresh(fx, model.window_visible);
    if (external_open_pending.swap(false, .acq_rel)) {
        update(model, .open_window, fx);
    }
}

/// Platform singleton routing marshals this callback onto the Native SDK UI
/// thread. Execute the same typed message as the tray command so visibility,
/// engine profile, sampling cadence, and presentation change atomically.
pub export fn sd300_model_open() callconv(.c) void {
    if (active_app_state) |app_state| {
        update(&app_state.model, .open_window, &app_state.effects);
    } else {
        external_open_pending.store(true, .release);
    }
}

pub fn refreshIntervalMs(visible: bool) u64 {
    return if (visible) visible_refresh_ms else hidden_refresh_ms;
}

/// Windows and macOS use a host-level hide close policy so tray mode can keep
/// collecting. With no live tray status item THIS session, turn that same close
/// into the normal, graceful quit on the next already-scheduled model tick. The
/// first argument is the STARTUP-EFFECTIVE tray presence, never the persisted
/// `tray_enabled` preference: a mid-session settings toggle only takes effect on
/// the next launch (RESTART REQUIRED), so the icon's real existence — not a
/// pending preference — governs whether a policy-hidden window may keep running.
/// A minimized window is deliberately not policy-hidden and must never satisfy
/// this predicate.
pub fn shouldQuitForHiddenWindow(tray_session_active: bool, policy_hidden: bool) bool {
    return !tray_session_active and policy_hidden;
}

fn scheduleRefresh(fx: *Effects, visible: bool) void {
    fx.startTimer(.{
        .key = refresh_timer_key,
        .interval_ms = refreshIntervalMs(visible),
        // A repeating timer preserves the one-second presentation cadence
        // without retiring and rebuilding the effect slot after every sample.
        // Re-registering the same key on a visibility transition atomically
        // replaces it with the 30-second hidden cadence.
        .mode = .repeating,
        .on_fire = Effects.timerMsg(.refresh_tick),
    });
}

fn scheduleExportPoll(fx: *Effects) void {
    fx.startTimer(.{
        .key = export_timer_key,
        .interval_ms = 250,
        .mode = .one_shot,
        .on_fire = Effects.timerMsg(.export_poll),
    });
}

fn applyWindowVisibility(model: *Model, visible: bool) bool {
    if (model.window_visible == visible) return false;
    model.window_visible = visible;
    if (active_engine) |runtime| runtime.setView(visible, model.active_section) catch {};
    return true;
}

fn selectSection(model: *Model, section: u8) void {
    model.scroll_top = 0;
    if (model.active_section == section) return;
    model.active_section = section;
    model.last_monitor_section = section;
    if (active_engine) |runtime| runtime.setView(model.window_visible, section) catch {};
    sampleEngine(model);
    persistSettings(model);
}

fn setProcessSort(model: *Model, process_sort: ProcessSort) void {
    model.process_sort = process_sort;
    sortProcesses(model);
    const runtime = active_engine orelse {
        model.status_buffer.set("Current process rows were sorted locally; the engine is unavailable for a full-inventory refresh.");
        return;
    };
    runtime.setProcessSort(@intFromEnum(process_sort)) catch {
        model.status_buffer.set("Current process rows were sorted locally; the engine rejected the full-inventory ranking request.");
        return;
    };
    model.status_buffer.set("Process ranking changed · full inventory refresh requested");
}

fn sortProcesses(model: *Model) void {
    const rows = model.detail.process_rows[0..model.detail.process_count];
    std.mem.sort(projection.ProcessRow, rows, model.process_sort, processLessThan);
    rebuildProcessFilter(model);
}

pub fn rebuildProcessFilter(model: *Model) void {
    model.filtered_process_count = 0;
    const needle = model.process_filter_buffer.text();
    if (needle.len == 0) return;
    for (model.detail.processes()) |row| {
        if (!containsIgnoreCase(row.friendlyName(), needle) and
            !containsIgnoreCase(row.name(), needle) and
            !containsIgnoreCase(row.status(), needle)) continue;
        model.filtered_process_rows[model.filtered_process_count] = row;
        model.filtered_process_count += 1;
    }
}

pub fn rebuildConnectionFilter(model: *Model) void {
    model.filtered_connection_count = 0;
    const needle = model.connection_filter_buffer.text();
    if (needle.len == 0) return;
    for (model.detail.connections()) |row| {
        var numeric_buffer: [48]u8 = undefined;
        const numeric = std.fmt.bufPrint(&numeric_buffer, "{d} {d} {d}", .{ row.local_port, row.remote_port, row.pid }) catch "";
        if (!containsIgnoreCase(row.protocol(), needle) and
            !containsIgnoreCase(row.local(), needle) and
            !containsIgnoreCase(row.remote(), needle) and
            !containsIgnoreCase(row.state(), needle) and
            !containsIgnoreCase(row.processName(), needle) and
            !containsIgnoreCase(numeric, needle)) continue;
        model.filtered_connection_rows[model.filtered_connection_count] = row;
        model.filtered_connection_count += 1;
    }
}

pub fn rebuildDriverFilter(model: *Model) void {
    model.filtered_driver_count = 0;
    const needle = model.driver_filter_buffer.text();
    for (model.detail.drivers()) |row| {
        if (model.driver_attention_only and !row.attention) continue;
        if (needle.len > 0 and
            !containsIgnoreCase(row.name(), needle) and
            !containsIgnoreCase(row.category(), needle) and
            !containsIgnoreCase(row.status(), needle) and
            !containsIgnoreCase(row.version(), needle) and
            !containsIgnoreCase(row.detail(), needle)) continue;
        model.filtered_driver_rows[model.filtered_driver_count] = row;
        model.filtered_driver_count += 1;
    }
}

fn containsIgnoreCase(haystack: []const u8, needle: []const u8) bool {
    if (needle.len == 0) return true;
    if (needle.len > haystack.len) return false;
    var index: usize = 0;
    while (index + needle.len <= haystack.len) : (index += 1) {
        if (std.ascii.eqlIgnoreCase(haystack[index .. index + needle.len], needle)) return true;
    }
    return false;
}

fn processLessThan(process_sort: ProcessSort, left: projection.ProcessRow, right: projection.ProcessRow) bool {
    return switch (process_sort) {
        .cpu => if (left.cpu_percent == right.cpu_percent) left.pid < right.pid else left.cpu_percent > right.cpu_percent,
        .memory => if (left.memory_mib == right.memory_mib) left.pid < right.pid else left.memory_mib > right.memory_mib,
        .pid => left.pid < right.pid,
        .name => if (!std.ascii.eqlIgnoreCase(left.friendlyName(), right.friendlyName()))
            std.ascii.lessThanIgnoreCase(left.friendlyName(), right.friendlyName())
        else if (!std.ascii.eqlIgnoreCase(left.name(), right.name()))
            std.ascii.lessThanIgnoreCase(left.name(), right.name())
        else
            left.pid < right.pid,
    };
}

fn settingsDocument(model: *const Model) settings.Document {
    return .{ .gui = .{
        .audience_mode = model.audience_mode,
        .temperature_unit = model.temperature_unit,
        .tray_enabled = model.tray_enabled,
        .launch_at_login = model.launch_at_login,
        .reduced_motion = model.reduced_motion,
        .chart_density = model.chart_density,
        .last_section = model.last_monitor_section,
    } };
}

fn persistSettings(model: *Model) void {
    const runtime = active_engine orelse {
        model.status_buffer.set("Settings could not be saved because the GUI engine is unavailable.");
        return;
    };
    settings.save(runtime, std.heap.page_allocator, settingsDocument(model)) catch {
        model.status_buffer.set("Settings could not be saved; the previous document remains intact.");
        return;
    };
    if (model.settings_restart_required) {
        model.status_buffer.set("Settings saved · restart SD-300 to apply the tray/close behavior change.");
    } else {
        model.status_buffer.set("GUI settings saved · terminal defaults remain unchanged.");
    }
}

fn requestDriverScan(model: *Model) void {
    const runtime = active_engine orelse {
        model.status_buffer.set("Driver scan unavailable — the GUI engine companion is not loaded.");
        return;
    };
    runtime.requestDriverScan() catch {
        model.status_buffer.set("Driver scan request failed; the previous inventory remains available.");
        return;
    };
    model.detail.drivers_ready = false;
    model.status_buffer.set("Driver scan requested · running asynchronously outside the renderer thread");
}

const ExportStatus = struct {
    state: []const u8,
    path: ?[]const u8 = null,
    @"error": ?[]const u8 = null,
};

fn requestExport(model: *Model, fx: *Effects, kind: engine.ExportKind) void {
    const runtime = active_engine orelse {
        model.status_buffer.set("Export unavailable — the GUI engine companion is not loaded.");
        return;
    };
    runtime.requestExport(kind) catch {
        model.status_buffer.set("Export request was not accepted; another export may still be running.");
        return;
    };
    model.export_pending = true;
    model.status_buffer.set("Preparing a redacted report from the current bounded collector state…");
    scheduleExportPoll(fx);
}

fn pollExport(model: *Model, fx: *Effects) void {
    const runtime = active_engine orelse {
        model.export_pending = false;
        model.status_buffer.set("Export failed because the GUI engine became unavailable.");
        return;
    };
    const allocator = std.heap.page_allocator;
    const payload = runtime.readExportStatusAlloc(allocator) catch {
        model.export_pending = false;
        model.status_buffer.set("Export status could not be read from the GUI engine.");
        return;
    };
    defer allocator.free(payload);
    const parsed = std.json.parseFromSlice(ExportStatus, allocator, payload, .{ .ignore_unknown_fields = true }) catch {
        model.export_pending = false;
        model.status_buffer.set("Export status was incompatible with this GUI version.");
        return;
    };
    defer parsed.deinit();
    const state = parsed.value;
    if (std.mem.eql(u8, state.state, "pending") or std.mem.eql(u8, state.state, "idle")) {
        scheduleExportPoll(fx);
        return;
    }
    model.export_pending = false;
    if (std.mem.eql(u8, state.state, "complete")) {
        var scratch: [384]u8 = undefined;
        const message = std.fmt.bufPrint(&scratch, "Export saved · {s}", .{state.path orelse "report directory"}) catch "Export saved.";
        model.status_buffer.set(message);
    } else {
        var scratch: [384]u8 = undefined;
        const message = std.fmt.bufPrint(&scratch, "Export failed · {s}", .{state.@"error" orelse "unknown engine error"}) catch "Export failed.";
        model.status_buffer.set(message);
    }
}

fn sampleEngine(model: *Model) void {
    const runtime = active_engine orelse {
        markFastSummaryFailed(model);
        model.status_buffer.set("Engine unavailable — run sd300 update to repair the GUI companion.");
        return;
    };
    const summary = runtime.readFastSummary() catch |err| {
        if (err == error.EngineDataPending) {
            updateFastSummaryFreshness(model);
            if (!model.fast_summary_seen) {
                model.engine_ready = false;
                model.status_buffer.set("Engine connected — waiting for the first live sample…");
            }
        } else {
            markFastSummaryFailed(model);
            model.status_buffer.set("Engine read failed — collection remains isolated from the TUI.");
        }
        return;
    };
    applySummary(model, summary);
    if (runtime.readTraySummary()) |tray_summary| applyTraySummary(model, tray_summary) else |_| {}
    sampleTopic(runtime, model, std.heap.page_allocator, .static);
    sampleTopic(runtime, model, std.heap.page_allocator, .warnings);
    sampleTopic(runtime, model, std.heap.page_allocator, .capabilities);
    if (model.active_section == 6) {
        sampleProcessSummary(runtime, model);
    } else if (model.active_section == 0) {
        sampleTopic(runtime, model, std.heap.page_allocator, .fast);
    } else {
        sampleDetailedTopics(runtime, model);
    }
}

fn sampleDetailedTopics(runtime: *engine.Runtime, model: *Model) void {
    const allocator = std.heap.page_allocator;
    switch (model.active_section) {
        1 => {
            sampleTopic(runtime, model, allocator, .fast);
            sampleTopic(runtime, model, allocator, .slow);
        },
        2 => sampleTopic(runtime, model, allocator, .fast),
        3 => {
            sampleTopic(runtime, model, allocator, .slow);
            sampleTopic(runtime, model, allocator, .health);
        },
        4 => sampleTopic(runtime, model, allocator, .slow),
        5 => {
            sampleTopic(runtime, model, allocator, .fast);
            sampleTopic(runtime, model, allocator, .medium);
            sampleTopic(runtime, model, allocator, .diagnostics);
        },
        7 => sampleTopic(runtime, model, allocator, .slow),
        8 => sampleTopic(runtime, model, allocator, .drivers),
        else => {},
    }
}

fn sampleProcessSummary(runtime: *engine.Runtime, model: *Model) void {
    const summary = runtime.readProcessSummary() catch {
        model.status_buffer.set("The bounded process projection could not be read; the last complete sample remains visible.");
        return;
    } orelse return;
    model.detail.applyProcessSummary(summary);
    if (model.process_sort != .cpu) {
        sortProcesses(model);
    } else {
        rebuildProcessFilter(model);
    }
}

fn sampleTopic(runtime: *engine.Runtime, model: *Model, allocator: std.mem.Allocator, topic: engine.Topic) void {
    const payload = runtime.readTopicAlloc(allocator, topic) catch {
        model.status_buffer.set("A detailed collector topic could not be read; other live topics remain available.");
        return;
    } orelse return;
    defer payload.deinit(allocator);

    const apply_result = switch (topic) {
        .static => model.detail.applyStaticJson(allocator, payload.bytes),
        .fast => model.detail.applyFastJson(allocator, payload.bytes),
        .medium => model.detail.applyMediumJson(allocator, payload.bytes),
        .slow => model.detail.applySlowJson(allocator, payload.bytes),
        .diagnostics => model.detail.applyDiagnosticsJson(allocator, payload.bytes),
        .health => model.detail.applyHealthJson(allocator, payload.bytes),
        .drivers => model.detail.applyDriversJson(allocator, payload.bytes),
        .warnings => model.detail.applyWarningsJson(allocator, payload.bytes),
        .capabilities => model.detail.applyCapabilitiesJson(allocator, payload.bytes),
    };
    apply_result catch {
        model.status_buffer.set("A detailed collector topic had an incompatible payload; the bounded overview remains live.");
        return;
    };
    switch (topic) {
        .fast => {
            const swap_percent = if (model.detail.swap_total_gib > 0)
                model.detail.swap_used_gib / model.detail.swap_total_gib * 100
            else
                0;
            pushHistory(&model.swap_history, swap_percent);
            pushHistoryRaw(&model.network_download_history, model.detail.total_download_kib_s);
            pushHistoryRaw(&model.network_upload_history, model.detail.total_upload_kib_s);
        },
        .slow => {
            if (model.detail.gpu_count > 0 and model.detail.gpu_rows[0].telemetry_available) {
                pushHistory(&model.gpu_history, model.detail.gpu_rows[0].utilization_percent);
            }
            const temperature = if (model.detail.cpu_temperature_available)
                model.detail.cpu_temperature_celsius
            else if (model.detail.gpu_temperature_available)
                model.detail.gpu_temperature_celsius
            else
                -1;
            if (temperature >= 0) {
                pushHistoryRaw(&model.temperature_history_celsius, temperature);
                pushHistoryRaw(&model.temperature_history_fahrenheit, (temperature * 9 / 5) + 32);
            }
        },
        .health => if (model.detail.disk_io_available) {
            pushHistoryRaw(&model.disk_read_history, model.detail.disk_read_mib_s);
            pushHistoryRaw(&model.disk_write_history, model.detail.disk_write_mib_s);
        },
        .medium => rebuildConnectionFilter(model),
        .drivers => rebuildDriverFilter(model),
        else => {},
    }
    // The engine's CPU projection is already rank-stabilized by
    // `Projection.applyFastJson`. Re-sorting it after every visual sample
    // defeats that contract: near-tied rows swap places, every text cell in
    // the table becomes dirty, and the SDK's software presenter must repaint
    // most of the panel. Memory/name are explicit user-selected orderings and
    // remain locally sorted until the engine owns those full-inventory sorts.
    if (topic == .fast and model.active_section == 6 and model.process_sort != .cpu) {
        sortProcesses(model);
    } else if (topic == .fast and model.active_section == 6) {
        rebuildProcessFilter(model);
    }
}

pub fn applySummary(model: *Model, summary: engine.FastSummary) void {
    const gib = 1024.0 * 1024.0 * 1024.0;
    const sequence_advanced = summary.sequence != model.sequence;
    const capture_advanced = summary.captured_unix_ms == 0 or
        model.overview_topic_meta.captured_unix_ms == 0 or
        summary.captured_unix_ms > model.overview_topic_meta.captured_unix_ms;
    const sample_advanced = !model.fast_summary_seen or (sequence_advanced and capture_advanced);
    model.engine_ready = true;
    model.fast_summary_seen = true;
    model.fast_summary_failed = false;
    if (sample_advanced) model.fast_summary_last_advance_ms = model.clock.monotonicMs();
    model.sequence = summary.sequence;
    model.cpu_percent = @as(f64, summary.cpu_percent);
    model.memory_percent = @as(f64, summary.memory_percent);
    model.memory_used_gib = @as(f64, @floatFromInt(summary.memory_used_bytes)) / gib;
    model.memory_total_gib = @as(f64, @floatFromInt(summary.memory_total_bytes)) / gib;
    model.logical_processors = summary.logical_processors;
    model.warning_count = summary.warning_count;
    var overview_meta = model.overview_topic_meta;
    overview_meta.ready = true;
    overview_meta.schema_version = 1;
    overview_meta.sequence = summary.sequence;
    overview_meta.captured_unix_ms = summary.captured_unix_ms;
    overview_meta.freshness_ms = 0;
    overview_meta.topic_buffer.set("fast-summary");
    overview_meta.availability_buffer.set("available");
    overview_meta.provenance_buffer.set("SD-300 platform CPU and memory collectors");
    const static_meta = model.detail.topicMeta(0);
    overview_meta.target_buffer.set(if (static_meta.ready) static_meta.target() else "active target");
    model.overview_topic_meta = overview_meta;
    if (sequence_advanced) {
        pushHistory(&model.cpu_history, model.cpu_percent);
        pushHistory(&model.memory_history, model.memory_percent);
    }
    var cpu_text: [64]u8 = undefined;
    const cpu_label = std.fmt.bufPrint(&cpu_text, "CPU · {d:.1}%", .{model.cpu_percent}) catch "CPU · live";
    model.tray_cpu_buffer.set(cpu_label);

    var memory_text: [64]u8 = undefined;
    const memory_label = std.fmt.bufPrint(&memory_text, "Memory · {d:.1}%", .{model.memory_percent}) catch "Memory · live";
    model.tray_memory_buffer.set(memory_label);
    updateFastSummaryFreshness(model);
}

/// A successful engine call is not enough to claim LIVE: the bounded fast
/// projection must keep advancing. This monotonic observation clock catches a
/// wedged collector even when the last payload remains readable, while the
/// capture clock catches an already-old payload immediately after resume.
pub fn updateFastSummaryFreshness(model: *Model) void {
    if (!model.fast_summary_seen) {
        model.fast_summary_stale = false;
        return;
    }
    const monotonic_age = model.clock.monotonicMs() -| model.fast_summary_last_advance_ms;
    const wall_ms = model.clock.wallMs();
    const now: u64 = if (wall_ms <= 0) 0 else @intCast(wall_ms);
    const captured = model.overview_topic_meta.captured_unix_ms;
    const capture_age = if (captured == 0 or now == 0) 0 else now -| captured;
    model.fast_summary_stale = monotonic_age > fast_summary_stale_after_ms or capture_age > fast_summary_stale_after_ms;
}

pub fn markFastSummaryFailed(model: *Model) void {
    model.engine_ready = false;
    model.fast_summary_failed = true;
    updateFastSummaryFreshness(model);
}

pub fn applyTraySummary(model: *Model, summary: engine.TraySummary) void {
    var gpu_text: [64]u8 = undefined;
    const gpu_label = if (summary.gpu_available == 1)
        std.fmt.bufPrint(&gpu_text, "GPU · {d:.1}%", .{summary.gpu_percent}) catch "GPU · live"
    else
        "GPU · unavailable";
    model.tray_gpu_buffer.set(gpu_label);

    var storage_text: [64]u8 = undefined;
    const storage_label = if (summary.storage_available == 1)
        std.fmt.bufPrint(&storage_text, "Storage free · {d:.1}%", .{summary.storage_free_percent}) catch "Storage · live"
    else
        "Storage · unavailable";
    model.tray_storage_buffer.set(storage_label);
    model.tray_health_buffer.set(switch (summary.disk_health) {
        1 => "Disk health · good",
        2 => "Disk health · warning",
        3 => "Disk health · critical",
        else => "Disk health · unknown",
    });
}

fn pushHistory(history: *[history_sample_count]f64, sample: f64) void {
    std.mem.copyForwards(f64, history[0 .. history.len - 1], history[1..]);
    history[history.len - 1] = std.math.clamp(sample, 0, 100);
}

fn pushHistoryRaw(history: *[history_sample_count]f64, sample: f64) void {
    std.mem.copyForwards(f64, history[0 .. history.len - 1], history[1..]);
    history[history.len - 1] = @max(sample, 0);
}

pub const AppUi = canvas.Ui(Msg);
pub const app_markup = @embedFile("app.native");
const CompiledAppView = canvas.CompiledMarkupView(Model, Msg, app_markup);

pub fn initialModel() Model {
    return initialModelWithSettings(.{});
}

pub fn initialModelWithSettings(document: settings.Document) Model {
    return .{
        .active_section = document.gui.last_section,
        .last_monitor_section = document.gui.last_section,
        .audience_mode = document.gui.audience_mode,
        .temperature_unit = document.gui.temperature_unit,
        .tray_enabled = document.gui.tray_enabled,
        // Session-effective tray presence, computed identically to `effective_tray`
        // in main() (which gates status-item creation). Captured once here so the
        // running session never conflates the persisted preference with the icon
        // that actually exists on screen.
        .tray_session_active = tray_supported and document.gui.tray_enabled,
        .launch_at_login = document.gui.launch_at_login,
        .reduced_motion = document.gui.reduced_motion,
        .chart_density = document.gui.chart_density,
    };
}

pub fn qubeTokens(model: *const Model) canvas.DesignTokens {
    var tokens = canvas.DesignTokens.theme(.{ .color_scheme = .dark, .reduce_motion = model.reduced_motion });
    tokens.colors.background = canvas.Color.rgb8(9, 9, 9);
    tokens.colors.surface = canvas.Color.rgb8(14, 14, 14);
    tokens.colors.surface_subtle = canvas.Color.rgb8(18, 18, 18);
    tokens.colors.surface_pressed = canvas.Color.rgb8(27, 22, 19);
    tokens.colors.text = canvas.Color.rgb8(232, 232, 232);
    tokens.colors.text_muted = canvas.Color.rgb8(156, 156, 164);
    tokens.colors.border = canvas.Color.rgb8(48, 48, 48);
    tokens.colors.accent = canvas.Color.rgb8(255, 94, 26);
    tokens.colors.accent_text = canvas.Color.rgb8(9, 9, 9);
    tokens.colors.info = canvas.Color.rgb8(255, 94, 26);
    tokens.colors.focus_ring = canvas.Color.rgb8(255, 94, 26);
    tokens.typography.font_id = makira_font_id;
    tokens.typography.mono_font_id = plex_mono_font_id;
    tokens.typography.button_font_id = makira_font_id;
    tokens.typography.body_size = 15;
    tokens.typography.label_size = 13;
    tokens.typography.title_size = 21;
    tokens.typography.heading_size = 30;
    tokens.typography.display_size = 50;
    tokens.radius = .{ .sm = 0, .md = 2, .lg = 2, .xl = 4 };
    tokens.controls.list_item = .{
        .background = canvas.Color.rgba8(255, 255, 255, 0),
        .hover_background = canvas.Color.rgba8(255, 255, 255, 10),
        .active_background = canvas.Color.rgba8(255, 255, 255, 14),
        .pressed_background = canvas.Color.rgba8(255, 94, 26, 20),
        .foreground = tokens.colors.text,
        .active_foreground = tokens.colors.text,
        .radius = 2,
        .stroke_width = 0,
    };
    tokens.controls.panel = .{
        .background = tokens.colors.surface,
        .border = tokens.colors.border,
        .radius = 2,
        .stroke_width = 1,
    };
    tokens.controls.badge = .{
        .background = tokens.colors.surface,
        .foreground = tokens.colors.accent,
        .border = tokens.colors.border,
        .radius = 2,
        .stroke_width = 1,
    };
    tokens.controls.button_outline = .{
        .background = tokens.colors.surface,
        .hover_background = canvas.Color.rgba8(255, 255, 255, 10),
        .pressed_background = canvas.Color.rgba8(255, 94, 26, 20),
        .foreground = tokens.colors.text,
        .border = tokens.colors.border,
        .radius = 2,
        .stroke_width = 1,
    };
    tokens.pixel_snap = .{ .geometry = true, .text = true };
    return tokens;
}

const chrome_grid_columns: usize = 9;
const chrome_grid_rows: usize = 5;
const chrome_command_count: usize = 2 + chrome_grid_columns + chrome_grid_rows;
const chrome_background_id: canvas.ObjectId = 0x5344_3300_0000_0001;
const chrome_warm_edge_id: canvas.ObjectId = 0x5344_3300_0000_0002;
const chrome_grid_id_base: canvas.ObjectId = 0x5344_3300_0000_0100;

const carbon_stops = [_]canvas.GradientStop{
    .{ .offset = 0, .color = canvas.Color.rgb8(9, 9, 9) },
    .{ .offset = 0.58, .color = canvas.Color.rgb8(14, 14, 14) },
    .{ .offset = 1, .color = canvas.Color.rgb8(9, 9, 9) },
};

const warm_edge_stops = [_]canvas.GradientStop{
    .{ .offset = 0, .color = canvas.Color.rgba8(255, 94, 26, 0) },
    .{ .offset = 0.72, .color = canvas.Color.rgba8(255, 94, 26, 0) },
    .{ .offset = 1, .color = canvas.Color.rgba8(255, 94, 26, 13) },
};

const vertical_grid_stops = [_]canvas.GradientStop{
    .{ .offset = 0, .color = canvas.Color.rgba8(232, 232, 232, 0) },
    .{ .offset = 0.22, .color = canvas.Color.rgba8(232, 232, 232, 10) },
    .{ .offset = 0.72, .color = canvas.Color.rgba8(232, 232, 232, 8) },
    .{ .offset = 1, .color = canvas.Color.rgba8(255, 94, 26, 0) },
};

const horizontal_grid_stops = [_]canvas.GradientStop{
    .{ .offset = 0, .color = canvas.Color.rgba8(232, 232, 232, 0) },
    .{ .offset = 0.18, .color = canvas.Color.rgba8(232, 232, 232, 9) },
    .{ .offset = 0.82, .color = canvas.Color.rgba8(232, 232, 232, 7) },
    .{ .offset = 1, .color = canvas.Color.rgba8(255, 94, 26, 0) },
};

comptime {
    const chrome_gradient_stop_count = carbon_stops.len +
        warm_edge_stops.len +
        chrome_grid_columns * vertical_grid_stops.len +
        chrome_grid_rows * horizontal_grid_stops.len;
    if (chrome_gradient_stop_count > native_sdk.runtime.max_canvas_gradient_stops_per_view) {
        @compileError("Warm Carbon chrome exceeds Native SDK's per-view gradient-stop budget");
    }
}

pub fn warmCarbonChrome(model: *const Model, builder: *canvas.Builder, size: geometry.SizeF, tokens: canvas.DesignTokens) anyerror!void {
    _ = model;
    _ = tokens;

    try builder.fillRect(.{
        .id = chrome_background_id,
        .rect = geometry.RectF.init(0, 0, size.width, size.height),
        .fill = .{ .linear_gradient = .{
            .start = geometry.PointF.init(0, 0),
            .end = geometry.PointF.init(size.width, size.height),
            .stops = &carbon_stops,
        } },
    });
    try builder.fillRect(.{
        .id = chrome_warm_edge_id,
        .rect = geometry.RectF.init(0, 0, size.width, size.height),
        .fill = .{ .linear_gradient = .{
            .start = geometry.PointF.init(size.width * 0.34, size.height * 0.22),
            .end = geometry.PointF.init(size.width, size.height),
            .stops = &warm_edge_stops,
        } },
    });

    const content_left = @min(size.width, 220);
    const content_top = @min(size.height, 88);
    const content_bottom = @max(content_top, size.height - 32);
    const content_width = @max(0, size.width - content_left);
    const content_height = @max(0, content_bottom - content_top);

    for (0..chrome_grid_columns) |index| {
        const fraction = @as(f32, @floatFromInt(index + 1)) / @as(f32, @floatFromInt(chrome_grid_columns + 1));
        const x = content_left + content_width * fraction;
        try builder.fillRect(.{
            .id = chrome_grid_id_base + index,
            .rect = geometry.RectF.init(x, content_top, 1, content_height),
            .fill = .{ .linear_gradient = .{
                .start = geometry.PointF.init(x, content_top),
                .end = geometry.PointF.init(x, content_bottom),
                .stops = &vertical_grid_stops,
            } },
        });
    }
    for (0..chrome_grid_rows) |index| {
        const fraction = @as(f32, @floatFromInt(index + 1)) / @as(f32, @floatFromInt(chrome_grid_rows + 1));
        const y = content_top + content_height * fraction;
        try builder.fillRect(.{
            .id = chrome_grid_id_base + chrome_grid_columns + index,
            .rect = geometry.RectF.init(content_left, y, content_width, 1),
            .fill = .{ .linear_gradient = .{
                .start = geometry.PointF.init(content_left, y),
                .end = geometry.PointF.init(size.width, y),
                .stops = &horizontal_grid_stops,
            } },
        });
    }
}

const app_features: native_sdk.UiAppFeatures = .{ .runtime_markup = builtin.mode == .Debug };
pub const NativeApp = native_sdk.UiAppWithFeatures(Model, Msg, app_features);

const app_fonts = [_]NativeApp.FontRegistration{
    .{
        .id = makira_font_id,
        .name = "Makira-Regular.ttf",
        .ttf = @embedFile("fonts/Makira-Regular.ttf"),
    },
    .{
        .id = plex_mono_font_id,
        .name = "IBMPlexMono-Regular.ttf",
        .ttf = @embedFile("fonts/IBMPlexMono-Regular.ttf"),
    },
};

const initial_tray_items = [_]native_sdk.platform.TrayMenuItem{
    .{ .label = "CPU · waiting for live data", .enabled = false },
    .{ .label = "Memory · waiting for live data", .enabled = false },
    .{ .label = "GPU · unavailable", .enabled = false },
    .{ .label = "Storage · waiting for inventory", .enabled = false },
    .{ .label = "Disk health · scanning", .enabled = false },
    .{ .separator = true },
    .{ .id = 1, .label = "Open SD-300", .command = "app.open" },
    .{ .separator = true },
    .{ .id = 2, .label = "Quit SD-300", .command = "app.quit" },
};

pub fn statusItem(model: *const Model, scratch: *NativeApp.StatusItemScratch) NativeApp.StatusItemState {
    scratch.items[0] = .{ .label = model.tray_cpu_buffer.text(), .enabled = false };
    scratch.items[1] = .{ .label = model.tray_memory_buffer.text(), .enabled = false };
    scratch.items[2] = .{ .label = model.tray_gpu_buffer.text(), .enabled = false };
    scratch.items[3] = .{ .label = model.tray_storage_buffer.text(), .enabled = false };
    scratch.items[4] = .{ .label = model.tray_health_buffer.text(), .enabled = false };
    scratch.items[5] = .{ .separator = true };
    scratch.items[6] = .{ .id = 1, .label = "Open SD-300", .command = "app.open" };
    scratch.items[7] = .{ .separator = true };
    scratch.items[8] = .{ .id = 2, .label = "Quit SD-300", .command = "app.quit" };
    return .{ .title = "SD", .items = scratch.items[0..9] };
}

pub fn isSelfTest(args: []const []const u8) bool {
    return hasArg(args, "--self-test");
}

pub fn startsHidden(args: []const []const u8) bool {
    return hasArg(args, "--startup") and hasArg(args, "--hidden");
}

fn hasArg(args: []const []const u8, expected: []const u8) bool {
    for (args) |arg| {
        if (std.mem.eql(u8, arg, expected)) return true;
    }
    return false;
}

fn iconPath(allocator: std.mem.Allocator, executable_dir: []const u8) ![]u8 {
    return if (builtin.os.tag == .macos)
        std.fs.path.resolve(allocator, &.{ executable_dir, "..", "Resources", "assets", "icon.png" })
    else
        std.fs.path.join(allocator, &.{ executable_dir, "assets", "icon.png" });
}

fn writeJsonLine(io: std.Io, allocator: std.mem.Allocator, value: anytype) !void {
    const json = try std.json.Stringify.valueAlloc(allocator, value, .{});
    defer allocator.free(json);
    const stdout = std.Io.File.stdout();
    try stdout.writeStreamingAll(io, json);
    try stdout.writeStreamingAll(io, "\n");
}

fn runSelfTest(init: std.process.Init) u8 {
    var runtime = engine.Runtime.init(init.io, init.gpa) catch |err| {
        writeJsonLine(init.io, init.gpa, .{
            .schema = 1,
            .success = false,
            .product = "SD-300",
            .product_version = engine.expected_product_version,
            .abi_version = engine.expected_abi_version,
            .engine_schema_version = engine.expected_schema_version,
            .target_os = @tagName(builtin.os.tag),
            .target_arch = @tagName(builtin.cpu.arch),
            .error_code = @errorName(err),
        }) catch {};
        return 2;
    };
    defer runtime.deinit();

    var attempts: usize = 0;
    while (attempts < 100) : (attempts += 1) {
        const summary = runtime.readFastSummary() catch |err| {
            if (err == error.EngineDataPending) {
                std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(100), .awake) catch break;
                continue;
            }
            writeJsonLine(init.io, init.gpa, .{
                .schema = 1,
                .success = false,
                .product = "SD-300",
                .product_version = engine.expected_product_version,
                .abi_version = engine.expected_abi_version,
                .engine_schema_version = engine.expected_schema_version,
                .target_os = @tagName(builtin.os.tag),
                .target_arch = @tagName(builtin.cpu.arch),
                .error_code = @errorName(err),
            }) catch {};
            return 2;
        };
        if (summary.sequence == 0 or summary.memory_total_bytes == 0 or summary.logical_processors == 0) {
            std.Io.sleep(init.io, std.Io.Duration.fromMilliseconds(100), .awake) catch break;
            continue;
        }
        writeJsonLine(init.io, init.gpa, .{
            .schema = 1,
            .success = true,
            .product = "SD-300",
            .product_version = engine.expected_product_version,
            .abi_version = engine.expected_abi_version,
            .engine_schema_version = engine.expected_schema_version,
            .target_os = @tagName(builtin.os.tag),
            .target_arch = @tagName(builtin.cpu.arch),
            .live_sequence = summary.sequence,
            .logical_processors = summary.logical_processors,
            .memory_total_bytes = summary.memory_total_bytes,
            // Windows Installer starts GUI-subsystem custom actions without an
            // attached stdout handle. Reporting must not turn a successful engine
            // qualification into a package failure; callers that redirect stdout
            // still receive and validate the JSON object.
        }) catch {};
        return 0;
    }

    writeJsonLine(init.io, init.gpa, .{
        .schema = 1,
        .success = false,
        .product = "SD-300",
        .product_version = engine.expected_product_version,
        .abi_version = engine.expected_abi_version,
        .engine_schema_version = engine.expected_schema_version,
        .target_os = @tagName(builtin.os.tag),
        .target_arch = @tagName(builtin.cpu.arch),
        .error_code = "EngineSampleTimeout",
    }) catch {};
    return 2;
}

pub fn main(init: std.process.Init) !void {
    const args = try init.minimal.args.toSlice(init.arena.allocator());
    if (isSelfTest(args)) std.process.exit(runSelfTest(init));

    if (!window_visibility.claimSingleInstanceOrNotify()) return;
    defer window_visibility.releaseSingleInstance();

    try window_visibility.installOpenSignal();
    defer window_visibility.uninstallOpenSignal();
    try window_visibility.installQuitSignal();
    var engine_runtime = engine.Runtime.init(init.io, std.heap.page_allocator) catch null;
    if (engine_runtime) |*runtime| active_engine = runtime;
    defer {
        active_engine = null;
        if (engine_runtime) |*runtime| runtime.deinit();
    }

    const settings_document = if (engine_runtime) |*runtime|
        settings.load(runtime, std.heap.page_allocator) catch settings.Document{}
    else
        settings.Document{};
    const effective_tray = tray_supported and settings_document.gui.tray_enabled;
    const start_hidden = startsHidden(args) and effective_tray;
    startup_should_show = !start_hidden;
    defer startup_should_show = true;
    try window_visibility.installStartupHide(start_hidden);
    defer window_visibility.uninstallOpenMessageRoute();
    const executable_dir = try std.process.executableDirPathAlloc(init.io, std.heap.page_allocator);
    defer std.heap.page_allocator.free(executable_dir);
    const icon_path = try iconPath(std.heap.page_allocator, executable_dir);
    defer std.heap.page_allocator.free(icon_path);
    const dynamic_shell_windows = [_]native_sdk.ShellWindow{.{
        .label = "main",
        .title = "SD-300 System Diagnostics",
        .width = window_width,
        .height = window_height,
        .restore_state = true,
        // The host window close policy is fixed when app.zon creates the
        // scene-first window. Windows/macOS therefore always use `.hide` and
        // the model converts it to a graceful quit when tray is disabled.
        // Linux has no tray in Native SDK 0.5.4 and retains `.quit`.
        .close_policy = if (tray_supported) .hide else .quit,
        .views = &shell_views,
    }};
    const dynamic_shell_scene: native_sdk.ShellConfig = .{ .windows = &dynamic_shell_windows };

    const app_state = try NativeApp.create(std.heap.page_allocator, .{
        .name = "sd300-gui",
        .scene = dynamic_shell_scene,
        .canvas_label = canvas_label,
        .update_fx = update,
        .init_fx = initEffects,
        .view = CompiledAppView.build,
        // Runtime markup and file watching are development facilities. Release
        // binaries use only the comptime-compiled view and cannot depend on a
        // source-tree path existing on the user's machine.
        .markup = if (builtin.mode == .Debug)
            .{ .source = app_markup, .watch_path = "src/app.native", .io = init.io }
        else
            null,
        .on_command = onCommand,
        .status_item = if (effective_tray) .{
            .title = "SD",
            .icon_path = icon_path,
            .tooltip = "SD-300 System Diagnostics",
            .items = &initial_tray_items,
        } else null,
        .status_item_fn = if (effective_tray) statusItem else null,
        .tokens_fn = qubeTokens,
        .chrome = .{ .prefix_commands = chrome_command_count, .build = warmCarbonChrome },
        .fonts = &app_fonts,
    });
    defer app_state.destroy();
    app_state.model = initialModelWithSettings(settings_document);
    active_app_state = app_state;
    defer active_app_state = null;

    try runner.runWithOptions(app_state.app(), .{
        .app_name = "SD-300",
        .window_title = "SD-300 System Diagnostics",
        .bundle_id = "dev.qubetx.sd300",
        .icon_path = icon_path,
        .default_frame = geometry.RectF.init(0, 0, window_width, window_height),
        .restore_state = true,
        .js_window_api = false,
        .security = .{
            .permissions = &app_permissions,
            .navigation = .{ .allowed_origins = &.{ "zero://inline", "zero://app" } },
        },
    }, init);
}

test {
    _ = @import("tests.zig");
}
