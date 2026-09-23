const std = @import("std");
const settings = @import("settings.zig");

/// One owned writer, one replaceable pending document. Disk I/O never holds
/// the queue lock; shutdown flushes the newest request before engine unload.
pub const Writer = struct {
    io: std.Io,
    context: *anyopaque,
    save_fn: *const fn (*anyopaque, settings.Document) anyerror!void,
    mutex: std.Io.Mutex = .init,
    changed: std.Io.Condition = .init,
    thread: ?std.Thread = null,
    pending: ?settings.Document = null,
    requested: u64 = 0,
    result: Result = .{},
    stopping: bool = false,

    pub const Result = struct { sequence: u64 = 0, failed: bool = false };

    pub fn start(self: *Writer) !void {
        self.thread = try std.Thread.spawn(.{}, run, .{self});
    }

    pub fn request(self: *Writer, document: settings.Document) u64 {
        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);
        std.debug.assert(!self.stopping);
        self.requested +|= 1;
        self.pending = document;
        self.changed.signal(self.io);
        return self.requested;
    }

    pub fn status(self: *Writer) Result {
        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);
        return self.result;
    }

    pub fn stop(self: *Writer) void {
        const thread = self.thread orelse return;
        self.mutex.lockUncancelable(self.io);
        self.stopping = true;
        self.changed.signal(self.io);
        self.mutex.unlock(self.io);
        thread.join();
        self.thread = null;
    }

    fn run(self: *Writer) void {
        self.mutex.lockUncancelable(self.io);
        defer self.mutex.unlock(self.io);
        while (true) {
            while (self.pending == null and !self.stopping) self.changed.waitUncancelable(self.io, &self.mutex);
            const document = self.pending orelse return;
            const sequence = self.requested;
            self.pending = null;
            self.mutex.unlock(self.io);
            const failed = if (self.save_fn(self.context, document)) |_| false else |_| true;
            self.mutex.lockUncancelable(self.io);
            self.result = .{ .sequence = sequence, .failed = failed };
        }
    }
};

test "pending preferences coalesce and shutdown flushes the newest document" {
    const Sink = struct {
        count: usize = 0,
        document: settings.Document = .{},
        fn save(context: *anyopaque, document: settings.Document) !void {
            const self: *@This() = @ptrCast(@alignCast(context));
            self.count += 1;
            self.document = document;
        }
    };
    var sink: Sink = .{};
    var writer: Writer = .{ .io = std.testing.io, .context = &sink, .save_fn = Sink.save };
    for (0..100) |index| _ = writer.request(.{ .gui = .{ .last_section = @intCast(index % 9) } });
    try writer.start();
    writer.stop();
    writer.stop();
    try std.testing.expectEqual(@as(usize, 1), sink.count);
    try std.testing.expectEqual(@as(u8, 0), sink.document.gui.last_section);
    try std.testing.expectEqual(@as(u64, 100), writer.status().sequence);
    try std.testing.expect(!writer.status().failed);
}

test "failed preference commit is observable after joined shutdown" {
    const Sink = struct {
        fn save(_: *anyopaque, _: settings.Document) !void { return error.AccessDenied; }
    };
    var context: u8 = 0;
    var writer: Writer = .{ .io = std.testing.io, .context = &context, .save_fn = Sink.save };
    _ = writer.request(.{});
    try writer.start();
    writer.stop();
    try std.testing.expect(writer.status().failed);
    try std.testing.expectEqual(@as(u64, 1), writer.status().sequence);
}

test "slow commit does not lock requests and latest preferences recover after failure" {
    const Sink = struct {
        entered: std.Io.Event = .unset,
        release: std.Io.Event = .unset,
        count: usize = 0,
        last_section: u8 = 0,
        fn save(context: *anyopaque, document: settings.Document) !void {
            const self: *@This() = @ptrCast(@alignCast(context));
            self.count += 1;
            if (self.count == 1) {
                self.entered.set(std.testing.io);
                self.release.waitUncancelable(std.testing.io);
                return error.AccessDenied;
            }
            self.last_section = document.gui.last_section;
        }
    };
    var sink: Sink = .{};
    var writer: Writer = .{ .io = std.testing.io, .context = &sink, .save_fn = Sink.save };
    _ = writer.request(.{});
    try writer.start();
    defer writer.stop();
    defer sink.release.set(std.testing.io);
    sink.entered.waitUncancelable(std.testing.io);
    for (1..108) |index| _ = writer.request(.{ .gui = .{ .last_section = @intCast(index % 9) } });
    sink.release.set(std.testing.io);
    writer.stop();
    try std.testing.expectEqual(@as(usize, 2), sink.count);
    try std.testing.expectEqual(@as(u8, 8), sink.last_section);
    try std.testing.expectEqual(@as(u64, 108), writer.status().sequence);
    try std.testing.expect(!writer.status().failed);
}
