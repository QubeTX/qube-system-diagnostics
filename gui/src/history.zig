//! Bounded captures projected into time buckets. NaN means an unobserved bucket.
//! Chart series must use bars: the SDK line renderer bridges non-finite points.
const std = @import("std");
pub const count = 60;
pub const Timeline = struct {
    captured: [count]u64 = [_]u64{0} ** count,
    values: [count]f64 = [_]f64{std.math.nan(f64)} ** count,
    latest: u64 = 0,
    pub fn observe(self: *Timeline, at: u64, value: ?f64) void {
        if (at == 0 or at == self.latest) return;
        if (at < self.latest) self.* = .{};
        std.mem.copyForwards(u64, self.captured[0 .. count - 1], self.captured[1..]);
        std.mem.copyForwards(f64, self.values[0 .. count - 1], self.values[1..]);
        self.captured[count-1] = at;
        self.values[count-1] = if (value) |v| (if (std.math.isFinite(v)) v else std.math.nan(f64)) else std.math.nan(f64);
        self.latest = at;
    }
    pub fn project(self: *const Timeline, now: u64, step: u64, output: *[count]f64) void {
        @memset(output, std.math.nan(f64));
        if (step == 0) return;
        const end = now / step;
        for (self.captured, self.values) |at, value| {
            if (at == 0 or at / step > end) continue;
            const age = end - at / step;
            if (age < count) output[count - 1 - @as(usize, @intCast(age))] = value;
        }
    }
};
test "capture deduplication gaps zero and stale buckets" {
    var h=Timeline{};
    h.observe(1000,0);h.observe(2400,6);h.observe(2400,7);h.observe(7000,9);
    var bins:[count]f64=undefined;h.project(7000,1000,&bins);
    try std.testing.expectEqual(@as(f64,0),bins[53]);try std.testing.expectEqual(@as(f64,6),bins[54]);
    try std.testing.expect(std.math.isNan(bins[55]));try std.testing.expectEqual(@as(f64,9),bins[59]);
    h.project(10000,1000,&bins);try std.testing.expect(std.math.isNan(bins[59]));
}
