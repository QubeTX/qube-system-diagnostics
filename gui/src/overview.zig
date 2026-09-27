//! Bounded, prepared overview copy. No inventory work runs in the view builder.
const std = @import("std");
const canvas = @import("native_sdk").canvas;
const projection = @import("projection.zig");
const Text = canvas.TextBuffer(192);

pub const Reading = struct {
    value_buffer: Text = .init("Reading…"),
    context_buffer: Text = .{},
    state_buffer: Text = .{},
    secondary_buffer: Text = .init("Reading…"),
    pub fn value(self: *const Reading) []const u8 { return self.value_buffer.text(); }
    pub fn context(self: *const Reading) []const u8 { return self.context_buffer.text(); }
    pub fn state(self: *const Reading) []const u8 { return self.state_buffer.text(); }
    pub fn secondary(self: *const Reading) []const u8 { return self.secondary_buffer.text(); }
};

pub const Adapter = struct {
    id: u64 = 0,
    load_available: bool = false,
    name_buffer: Text = .{},
    load_buffer: Text = .{},
    memory_buffer: Text = .{},
    temperature_buffer: Text = .{},
    pub fn name(self: *const Adapter) []const u8 { return self.name_buffer.text(); }
    pub fn load(self: *const Adapter) []const u8 { return self.load_buffer.text(); }
    pub fn memory(self: *const Adapter) []const u8 { return self.memory_buffer.text(); }
    pub fn temperature(self: *const Adapter) []const u8 { return self.temperature_buffer.text(); }
};

pub fn columnsForWidth(content_width: f32) u32 {
    return if (content_width >= 840) 3 else if (content_width >= 560) 2 else 1;
}

fn format(buffer: *Text, comptime fmt: []const u8, args: anytype) void {
    var bytes: [192]u8 = undefined;
    buffer.set(std.fmt.bufPrint(&bytes, fmt, args) catch "Not available");
}

// Omit trademark boilerplate, retaining the actual model and its qualifiers.
pub fn shortHardwareName(buffer: *Text, source: []const u8) void {
    var bytes: [192]u8 = undefined;
    var used: usize = 0;
    var at: usize = if (std.mem.startsWith(u8, source, "NVIDIA GeForce ")) 7 else 0;
    while (at < source.len and used < bytes.len) {
        if (std.mem.startsWith(u8, source[at..], "(R)")) { at += 3; continue; }
        if (std.mem.startsWith(u8, source[at..], "(TM)")) { at += 4; continue; }
        if (std.mem.eql(u8, source[at..], " GPU")) break;
        bytes[used] = source[at]; used += 1; at += 1;
    }
    buffer.set(bytes[0..used]);
}

pub fn topicState(meta: *const projection.TopicMeta, now: u64) []const u8 {
    if (!meta.ready) return "Reading…";
    const status = meta.availability();
    if (std.mem.eql(u8, status, "error") or std.mem.eql(u8, status, "contradictory")) return if (meta.captured_unix_ms > 0) "Couldn’t read · previous values" else "Couldn’t read";
    if (!std.mem.eql(u8, status, "available")) return if (meta.captured_unix_ms > 0) "Not available · previous values" else "Not available";
    if (meta.captured_unix_ms == 0 or now < meta.captured_unix_ms or now - meta.captured_unix_ms > meta.expected_interval_ms *| 3) return "Reading delayed · previous values";
    return "";
}

fn temperature(buffer: *Text, value: f64, fahrenheit: bool) void {
    format(buffer, "{d:.0} °{s}", .{ if (fahrenheit) value * 9 / 5 + 32 else value, if (fahrenheit) "F" else "C" });
}

pub const View = struct {
    cpu: Reading = .{},
    memory: Reading = .{},
    disk: Reading = .{},
    network: Reading = .{},
    thermals: Reading = .{},
    gpu_state_buffer: Text = .init("Reading…"),
    gpu_more_buffer: Text = .{},
    gpu_rows: [2]Adapter = .{ .{}, .{} },
    gpu_count: usize = 0,
    process_buffer: Text = .init("Reading processes…"),
    process_cpu_buffer: Text = .init("CPU · reading…"),
    process_memory_buffer: Text = .init("Memory · reading…"),
    process_state_buffer: Text = .{},
    finding_rows: [16]projection.FindingRow = [_]projection.FindingRow{.{}} ** 16,
    finding_count: usize = 0,
    collection_rows: [16]projection.FindingRow = [_]projection.FindingRow{.{}} ** 16,
    collection_count: usize = 0,
    pub fn gpus(self: *const View) []const Adapter { return self.gpu_rows[0..self.gpu_count]; }
    pub fn gpuState(self: *const View) []const u8 { return self.gpu_state_buffer.text(); }
    pub fn gpuMore(self: *const View) []const u8 { return self.gpu_more_buffer.text(); }
    pub fn processes(self: *const View) []const u8 { return self.process_buffer.text(); }
    pub fn processCpu(self: *const View) []const u8 { return self.process_cpu_buffer.text(); }
    pub fn processMemory(self: *const View) []const u8 { return self.process_memory_buffer.text(); }
    pub fn processState(self: *const View) []const u8 { return self.process_state_buffer.text(); }
    pub fn findings(self: *const View) []const projection.FindingRow { return self.finding_rows[0..@min(2, self.finding_count)]; }
    pub fn allFindings(self: *const View) []const projection.FindingRow { return self.finding_rows[0..self.finding_count]; }
    pub fn collectionFindings(self: *const View) []const projection.FindingRow { return self.collection_rows[0..self.collection_count]; }

    pub fn prepare(self: *View, model: anytype) void {
        const d = &model.detail;
        const now: u64 = @intCast(@max(0, model.clock.wallMs()));
        const fast = topicState(d.topicMeta(1), now);
        const slow = topicState(d.topicMeta(3), now);
        const summary_state = if (model.summaryFailed()) (if (model.fast_summary_seen) "Couldn’t read · previous values" else "Couldn’t read") else if (model.summaryStale()) "Reading delayed · previous values" else if (!model.fast_summary_seen) "Reading…" else "";
        self.cpu.state_buffer.set(summary_state);
        self.memory.state_buffer.set(summary_state);
        if (model.fast_summary_seen) {
            format(&self.cpu.value_buffer, "{d:.1}%", .{model.cpu_percent});
            if (model.memory_total_gib > 0) {
                format(&self.memory.value_buffer, "{d:.1}%", .{model.memory_percent});
                format(&self.memory.context_buffer, "{d:.1} / {d:.1} GiB used", .{model.memory_used_gib, model.memory_total_gib});
            } else self.memory.value_buffer.set("Not available");
        } else if (model.summaryFailed()) {
            self.cpu.value_buffer.set("Couldn’t read");
            self.memory.value_buffer.set("Couldn’t read");
        }
        shortHardwareName(&self.cpu.context_buffer, if (d.cpuModel().len > 0) d.cpuModel() else model.systemCpu());
        self.disk.state_buffer.set(if (model.diskIoAvailable()) "" else if (d.activity_captured_unix_ms == 0) "Reading…" else if (d.activity_observation.available) "Reading delayed · previous values" else "Couldn’t read");
        if (d.activity_captured_unix_ms > 0 and d.disk_io_available) {
            format(&self.disk.value_buffer, "{d:.1}", .{d.disk_read_mib_s});
            format(&self.disk.secondary_buffer, "{d:.1}", .{d.disk_write_mib_s});
        } else { self.disk.value_buffer.set(if (d.fast_ready) "Not available" else "Reading…"); self.disk.secondary_buffer.set(self.disk.value()); }
        if (d.overview_disk_available) {
            format(&self.disk.context_buffer, "{s} · {d:.1} GiB free{s}", .{d.overview_disk.mount(), d.overview_disk.available_gib, if (slow.len > 0) " · previous" else ""});
        } else self.disk.context_buffer.set("Storage capacity not available");
        self.network.state_buffer.set(fast);
        if (d.network_rate_available) {
            format(&self.network.value_buffer, "{d:.1}", .{d.total_download_kib_s});
            format(&self.network.secondary_buffer, "{d:.1}", .{d.total_upload_kib_s});
        } else { self.network.value_buffer.set(if (d.fast_ready) "Not available" else "Reading…"); self.network.secondary_buffer.set(self.network.value()); }
        self.network.context_buffer.set("Download / Upload · KiB/s");
        self.gpu_state_buffer.set(slow);
        self.gpu_count = @min(2, d.gpu_count);
        if (d.slow_ready and self.gpu_count == 0 and slow.len == 0) self.gpu_state_buffer.set("Not available");
        // Stable identity ordering, never utilization order, avoids jumping cards.
        var indices: [projection.max_gpus]usize = undefined;
        for (0..d.gpu_count) |i| indices[i] = i;
        std.mem.sort(usize, indices[0..d.gpu_count], d, struct {
            fn less(detail: *const projection.Projection, a: usize, b: usize) bool {
                const aa = &detail.gpu_rows[a]; const bb = &detail.gpu_rows[b];
                const order = std.mem.order(u8, aa.identity(), bb.identity());
                return if (order == .eq) std.mem.lessThan(u8, aa.name(), bb.name()) else order == .lt;
            }
        }.less);
        const fahrenheit = model.temperature_unit == .fahrenheit;
        for (indices[0..self.gpu_count], 0..) |index, i| {
            const gpu = &d.gpu_rows[index];
            var row = Adapter{ .id = gpu.id };
            shortHardwareName(&row.name_buffer, gpu.name());
            row.load_available = gpu.utilization_available and gpu.utilization_observation.available;
            if (row.load_available) format(&row.load_buffer, "{d:.0}%", .{gpu.utilization_percent}) else row.load_buffer.set("Not available");
            if (gpu.unified) row.memory_buffer.set("Unified memory") else if (gpu.memory_used_available and gpu.memory_total_available and gpu.memory_observation.available) {
                format(&row.memory_buffer, "{d:.1} / {d:.1} GiB video memory", .{gpu.memory_used_mib / 1024, gpu.memory_total_mib / 1024});
            } else row.memory_buffer.set("Video memory not reported");
            if (gpu.temperature_available and gpu.temperature_observation.available) temperature(&row.temperature_buffer, gpu.temperature_celsius, fahrenheit) else row.temperature_buffer.set("Not available");
            self.gpu_rows[i] = row;
        }
        if (d.gpu_total_count > 2) format(&self.gpu_more_buffer, "+{d} more", .{d.gpu_total_count - 2}) else self.gpu_more_buffer.set("");
        self.thermals.state_buffer.set(slow);
        if (d.cpu_temperature_available and d.cpu_temperature_observation.available) temperature(&self.thermals.value_buffer, d.cpu_temperature_celsius, fahrenheit) else self.thermals.value_buffer.set(if (d.slow_ready) "Not available" else "Reading…");
        self.process_state_buffer.set(fast);
        if (d.fast_ready and d.process_observation.available) format(&self.process_buffer, "{d} processes", .{d.process_total_count}) else self.process_buffer.set(if (d.fast_ready) "Processes not available" else "Reading processes…");
        if (d.top_cpu_available and d.top_cpu.cpu_available) format(&self.process_cpu_buffer, "{s} · {d:.1}% of one core", .{d.top_cpu.friendlyName(), d.top_cpu.cpu_percent}) else self.process_cpu_buffer.set("CPU · reading not available");
        if (d.top_memory_available and d.top_memory.memory_available) format(&self.process_memory_buffer, "{s} · {d:.0} MiB", .{d.top_memory.friendlyName(), d.top_memory.memory_mib}) else self.process_memory_buffer.set("Memory · reading not available");
        self.finding_count = 0;
        self.collection_count = 0;
        for (d.findings()) |finding| {
            if (std.mem.eql(u8, finding.kind(), "incomplete_observation")) {
                self.collection_rows[self.collection_count] = finding;
                self.collection_count += 1;
            } else {
                self.finding_rows[self.finding_count] = finding;
                self.finding_count += 1;
            }
        }
    }
};
