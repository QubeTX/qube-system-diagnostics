const std = @import("std");
const engine = @import("engine.zig");

pub const schema_version: u32 = 1;

pub const AudienceMode = enum {
    user,
    technician,
};

pub const TemperatureUnit = enum {
    celsius,
    fahrenheit,
};

pub const ChartDensity = enum {
    compact,
    balanced,
    comfortable,
};

pub const Shared = struct {};

pub const Gui = struct {
    audience_mode: AudienceMode = .user,
    temperature_unit: TemperatureUnit = .celsius,
    tray_enabled: bool = false,
    launch_at_login: bool = false,
    reduced_motion: bool = true,
    chart_density: ChartDensity = .balanced,
    last_section: u8 = 0,
};

pub const Document = struct {
    schema_version: u32 = schema_version,
    shared: Shared = .{},
    gui: Gui = .{},
};

pub fn load(runtime: *engine.Runtime, allocator: std.mem.Allocator) !Document {
    const bytes = try runtime.readSettingsAlloc(allocator);
    defer allocator.free(bytes);
    const parsed = try std.json.parseFromSlice(Document, allocator, bytes, .{ .ignore_unknown_fields = true });
    defer parsed.deinit();
    if (parsed.value.schema_version != schema_version or parsed.value.gui.last_section > 8) {
        return error.SettingsSchemaMismatch;
    }
    return parsed.value;
}

pub fn save(runtime: *engine.Runtime, allocator: std.mem.Allocator, document: Document) !void {
    if (document.schema_version != schema_version or document.gui.last_section > 8) {
        return error.SettingsSchemaMismatch;
    }
    const bytes = try std.json.Stringify.valueAlloc(allocator, document, .{});
    defer allocator.free(bytes);
    try runtime.writeSettings(bytes);
}

test "GUI defaults are isolated from terminal behavior" {
    const document: Document = .{};
    try std.testing.expectEqual(AudienceMode.user, document.gui.audience_mode);
    try std.testing.expectEqual(TemperatureUnit.celsius, document.gui.temperature_unit);
    try std.testing.expect(!document.gui.tray_enabled);
    try std.testing.expect(!document.gui.launch_at_login);
    try std.testing.expect(document.gui.reduced_motion);
}
