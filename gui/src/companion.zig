const std = @import("std");
const canvas = @import("native_sdk").canvas;

pub const Line = struct {
    id: usize = 0,
    buffer: canvas.TextBuffer(2048) = .{},
    pub fn text(self: *const Line) []const u8 { return self.buffer.text(); }
};
pub const DetailLine = struct {
    id: usize = 0,
    buffer: canvas.TextBuffer(1024) = .{},
    pub fn text(self: *const DetailLine) []const u8 { return self.buffer.text(); }
};
pub const Projection = struct {
    storage_running: bool = false,
    storage_confirming: bool = false,
    storage_notice: canvas.TextBuffer(4096) = .{},
    storage_count: usize = 0,
    storage_rows: [10]Line = [_]Line{.{}} ** 10,
    running: bool = false,
    setup_network_notice: canvas.TextBuffer(3072) = .{},
    setup_smart_notice: canvas.TextBuffer(3072) = .{},
    setup_message: canvas.TextBuffer(2048) = .{},
    line_count: usize = 0,
    detail_count: usize = 0,
    detail_rows: [255]DetailLine = [_]DetailLine{.{}} ** 255,
    rows: [16]Line = [_]Line{.{}} ** 16,
    pub fn storageLines(self: *const Projection) []const Line { return self.storage_rows[0..self.storage_count]; }
    pub fn details(self: *const Projection) []const DetailLine { return self.detail_rows[0..self.detail_count]; }
    pub fn lines(self: *const Projection) []const Line { return self.rows[0..self.line_count]; }
    pub fn apply(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const Envelope = struct { data: struct {
            companion: struct { running: bool = false, result: ?struct { detail_lines: []const []const u8 = &.{} } = null } = .{},
            companion_lines: []const []const u8 = &.{},
            setup_network_notice: []const u8 = "",
            setup_smart_notice: []const u8 = "",
            storage_probe: struct { running: bool = false, awaiting_consent: bool = false, notice: []const u8 = "" } = .{},
            storage_probe_lines: []const []const u8 = &.{},
            optional_setup: struct { running: bool = false, message: []const u8 = "" } = .{},
        } };
        const parsed = try std.json.parseFromSlice(Envelope, allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.storage_running = parsed.value.data.storage_probe.running;
        self.storage_confirming = parsed.value.data.storage_probe.awaiting_consent;
        self.storage_notice.set(parsed.value.data.storage_probe.notice);
        self.storage_count = @min(parsed.value.data.storage_probe_lines.len, self.storage_rows.len);
        for (parsed.value.data.storage_probe_lines[0..self.storage_count], 0..) |line, index| {
            self.storage_rows[index].id = index;
            self.storage_rows[index].buffer.set(line);
        }
        self.running = parsed.value.data.companion.running or parsed.value.data.optional_setup.running;
        self.setup_network_notice.set(parsed.value.data.setup_network_notice);
        self.setup_smart_notice.set(parsed.value.data.setup_smart_notice);
        self.setup_message.set(parsed.value.data.optional_setup.message);
        const captured_details = if (parsed.value.data.companion.result) |result| result.detail_lines else &.{};
        self.detail_count = @min(captured_details.len, self.detail_rows.len);
        for (captured_details[0..self.detail_count], 0..) |line, index| {
            self.detail_rows[index].id = index;
            self.detail_rows[index].buffer.set(line);
        }
        self.line_count = @min(parsed.value.data.companion_lines.len, self.rows.len);
        for (parsed.value.data.companion_lines[0..self.line_count], 0..) |line, index| {
            self.rows[index].id = index;
            self.rows[index].buffer.set(line);
        }
    }
};
