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
    running: bool = false,
    setup_message: canvas.TextBuffer(2048) = .{},
    line_count: usize = 0,
    detail_count: usize = 0,
    detail_rows: [255]DetailLine = [_]DetailLine{.{}} ** 255,
    rows: [16]Line = [_]Line{.{}} ** 16,
    pub fn details(self: *const Projection) []const DetailLine { return self.detail_rows[0..self.detail_count]; }
    pub fn lines(self: *const Projection) []const Line { return self.rows[0..self.line_count]; }
    pub fn apply(self: *Projection, allocator: std.mem.Allocator, bytes: []const u8) !void {
        const Envelope = struct { data: struct {
            companion: struct { running: bool = false, result: ?struct { detail_lines: []const []const u8 = &.{} } = null } = .{},
            companion_lines: []const []const u8 = &.{},
            optional_setup: struct { running: bool = false, message: []const u8 = "" } = .{},
        } };
        const parsed = try std.json.parseFromSlice(Envelope, allocator, bytes, .{ .ignore_unknown_fields = true });
        defer parsed.deinit();
        self.running = parsed.value.data.companion.running or parsed.value.data.optional_setup.running;
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
