const std = @import("std");
const builtin = @import("builtin");

const windows = std.os.windows;

pub const expected_abi_version: u32 = 1;
pub const expected_schema_version: u32 = 1;
pub const expected_product_version = "3.0.0";

pub const status_ok: i32 = 0;
pub const status_unchanged: i32 = 1;
pub const status_buffer_too_small: i32 = 2;

pub const Topic = enum(u32) {
    static = 0,
    fast = 1,
    medium = 2,
    slow = 3,
    diagnostics = 4,
    health = 5,
    drivers = 6,
    warnings = 7,
    capabilities = 8,
};

pub const ExportKind = enum(u32) {
    redacted_snapshot = 1,
    capabilities = 2,
};

pub const TopicPayload = struct {
    bytes: []u8,
    sequence: u64,

    pub fn deinit(self: TopicPayload, allocator: std.mem.Allocator) void {
        allocator.free(self.bytes);
    }
};

pub const FastSummary = extern struct {
    sequence: u64 = 0,
    captured_unix_ms: u64 = 0,
    cpu_percent: f32 = 0,
    memory_percent: f32 = 0,
    memory_used_bytes: u64 = 0,
    memory_total_bytes: u64 = 0,
    logical_processors: u32 = 0,
    warning_count: u32 = 0,
};

pub const TraySummary = extern struct {
    sequence: u64 = 0,
    gpu_percent: f32 = 0,
    storage_free_percent: f32 = 0,
    gpu_available: u32 = 0,
    storage_available: u32 = 0,
    disk_health: u32 = 0,
    reserved: u32 = 0,
};

comptime {
    if (@sizeOf(FastSummary) != 48 or @alignOf(FastSummary) != 8) {
        @compileError("FastSummary no longer matches the SD-300 Rust ABI");
    }
    if (@sizeOf(TraySummary) != 32 or @alignOf(TraySummary) != 8) {
        @compileError("TraySummary no longer matches the SD-300 Rust ABI");
    }
}

const AbiVersionFn = *const fn () callconv(.c) u32;
const MetadataFn = *const fn (?[*]u8, usize, *usize) callconv(.c) i32;
const WriteSettingsFn = *const fn ([*]const u8, usize) callconv(.c) i32;
const SetLaunchAtLoginFn = *const fn (u32, u32) callconv(.c) i32;
const CreateFn = *const fn (*?*anyopaque) callconv(.c) i32;
const HandleFn = *const fn (?*anyopaque) callconv(.c) i32;
const RequestExportFn = *const fn (?*anyopaque, u32) callconv(.c) i32;
const ReadExportStatusFn = *const fn (?*anyopaque, ?[*]u8, usize, *usize) callconv(.c) i32;
const SetProfileFn = *const fn (?*anyopaque, u32) callconv(.c) i32;
const SetProcessSortFn = *const fn (?*anyopaque, u32) callconv(.c) i32;
const ReadFastSummaryFn = *const fn (?*anyopaque, *FastSummary) callconv(.c) i32;
const ReadTraySummaryFn = *const fn (?*anyopaque, *TraySummary) callconv(.c) i32;
const ReadTopicFn = *const fn (?*anyopaque, u32, u64, ?[*]u8, usize, *usize, *u64) callconv(.c) i32;

const profile_overview: u32 = 2;
const profile_hidden: u32 = 1;
const profile_foreground: u32 = 0;
const profile_processes: u32 = 3;

const Metadata = struct {
    abi_version: u32,
    schema_version: u32,
    product: []const u8,
    product_version: []const u8,
    target_os: []const u8,
    target_arch: []const u8,
};

const Library = struct {
    handle: if (builtin.os.tag == .windows) windows.HMODULE else *anyopaque,

    fn open(path: []const u8, allocator: std.mem.Allocator) !Library {
        if (builtin.os.tag == .windows) {
            const path_w = try std.unicode.utf8ToUtf16LeAllocZ(allocator, path);
            defer allocator.free(path_w);
            const flags = load_library_search_dll_load_dir | load_library_search_system32;
            const handle = LoadLibraryExW(path_w.ptr, null, flags) orelse
                return error.EngineLibraryOpenFailed;
            return .{ .handle = handle };
        }

        const path_z = try allocator.dupeZ(u8, path);
        defer allocator.free(path_z);
        const mode: std.c.RTLD = switch (builtin.os.tag) {
            .macos => .{ .NOW = true, .LOCAL = true },
            // POSIX RTLD_LOCAL is the zero/default state on Linux. Zig's
            // typed Linux RTLD flags therefore expose GLOBAL but no LOCAL
            // field; requesting NOW without GLOBAL retains local scope.
            .linux => .{ .NOW = true },
            else => @compileError("SD-300 GUI supports only Windows, macOS, and Linux"),
        };
        const handle = std.c.dlopen(path_z.ptr, mode) orelse
            return error.EngineLibraryOpenFailed;
        return .{ .handle = handle };
    }

    fn close(self: *Library) void {
        if (builtin.os.tag == .windows) {
            _ = FreeLibrary(self.handle);
        } else {
            _ = std.c.dlclose(self.handle);
        }
        self.* = undefined;
    }

    fn lookup(self: *Library, comptime T: type, name: [:0]const u8) !T {
        const address = if (builtin.os.tag == .windows)
            GetProcAddress(self.handle, name.ptr)
        else
            std.c.dlsym(self.handle, name.ptr);
        const symbol = address orelse return error.EngineSymbolMissing;
        return @as(T, @ptrCast(@alignCast(symbol)));
    }
};

pub const Runtime = struct {
    library: Library,
    handle: ?*anyopaque,
    stop_fn: HandleFn,
    destroy_fn: HandleFn,
    set_profile_fn: SetProfileFn,
    set_process_sort_fn: SetProcessSortFn,
    request_driver_scan_fn: HandleFn,
    read_fast_summary_fn: ReadFastSummaryFn,
    read_tray_summary_fn: ReadTraySummaryFn,
    read_topic_fn: ReadTopicFn,
    read_settings_fn: MetadataFn,
    write_settings_fn: WriteSettingsFn,
    set_launch_at_login_fn: SetLaunchAtLoginFn,
    request_export_fn: RequestExportFn,
    read_export_status_fn: ReadExportStatusFn,
    topic_sequences: [9]u64 = [_]u64{0} ** 9,

    pub fn init(io: std.Io, allocator: std.mem.Allocator) !Runtime {
        const exe_dir = try std.process.executableDirPathAlloc(io, allocator);
        defer allocator.free(exe_dir);
        const engine_path = try std.fs.path.join(allocator, &.{ exe_dir, engineFileName() });
        defer allocator.free(engine_path);
        if (!std.fs.path.isAbsolute(engine_path)) return error.EnginePathNotAbsolute;

        var library = try Library.open(engine_path, allocator);
        errdefer library.close();

        const abi_version_fn = try library.lookup(AbiVersionFn, "sd300_engine_abi_version");
        const schema_version_fn = try library.lookup(AbiVersionFn, "sd300_engine_schema_version");
        const metadata_fn = try library.lookup(MetadataFn, "sd300_engine_metadata");
        const create_fn = try library.lookup(CreateFn, "sd300_engine_create");
        const start_fn = try library.lookup(HandleFn, "sd300_engine_start");
        const set_profile_fn = try library.lookup(SetProfileFn, "sd300_engine_set_profile");
        const set_process_sort_fn = try library.lookup(SetProcessSortFn, "sd300_engine_set_process_sort");
        const request_driver_scan_fn = try library.lookup(HandleFn, "sd300_engine_request_driver_scan");
        const stop_fn = try library.lookup(HandleFn, "sd300_engine_stop");
        const destroy_fn = try library.lookup(HandleFn, "sd300_engine_destroy");
        const read_fast_summary_fn = try library.lookup(ReadFastSummaryFn, "sd300_engine_read_fast_summary");
        const read_tray_summary_fn = try library.lookup(ReadTraySummaryFn, "sd300_engine_read_tray_summary");
        const read_topic_fn = try library.lookup(ReadTopicFn, "sd300_engine_read_topic");
        const read_settings_fn = try library.lookup(MetadataFn, "sd300_engine_read_settings");
        const write_settings_fn = try library.lookup(WriteSettingsFn, "sd300_engine_write_settings");
        const set_launch_at_login_fn = try library.lookup(SetLaunchAtLoginFn, "sd300_engine_set_launch_at_login");
        const request_export_fn = try library.lookup(RequestExportFn, "sd300_engine_request_export");
        const read_export_status_fn = try library.lookup(ReadExportStatusFn, "sd300_engine_read_export_status");

        if (abi_version_fn() != expected_abi_version or
            schema_version_fn() != expected_schema_version)
        {
            return error.EngineVersionMismatch;
        }
        try validateMetadata(allocator, metadata_fn);

        var handle: ?*anyopaque = null;
        if (create_fn(&handle) != status_ok or handle == null) {
            return error.EngineCreateFailed;
        }
        errdefer _ = destroy_fn(handle);
        if (set_profile_fn(handle, profile_overview) != status_ok) {
            return error.EngineProfileFailed;
        }
        if (start_fn(handle) != status_ok) {
            return error.EngineStartFailed;
        }

        return .{
            .library = library,
            .handle = handle,
            .stop_fn = stop_fn,
            .destroy_fn = destroy_fn,
            .set_profile_fn = set_profile_fn,
            .set_process_sort_fn = set_process_sort_fn,
            .request_driver_scan_fn = request_driver_scan_fn,
            .read_fast_summary_fn = read_fast_summary_fn,
            .read_tray_summary_fn = read_tray_summary_fn,
            .read_topic_fn = read_topic_fn,
            .read_settings_fn = read_settings_fn,
            .write_settings_fn = write_settings_fn,
            .set_launch_at_login_fn = set_launch_at_login_fn,
            .request_export_fn = request_export_fn,
            .read_export_status_fn = read_export_status_fn,
        };
    }

    pub fn deinit(self: *Runtime) void {
        if (self.handle) |handle| {
            _ = self.stop_fn(handle);
            _ = self.destroy_fn(handle);
            self.handle = null;
        }
        self.library.close();
    }

    pub fn readFastSummary(self: *Runtime) !FastSummary {
        var summary = FastSummary{};
        const status = self.read_fast_summary_fn(self.handle, &summary);
        if (status == status_ok) return summary;
        if (status == status_unchanged) return error.EngineDataPending;
        return error.EngineReadFailed;
    }

    pub fn readTraySummary(self: *Runtime) !TraySummary {
        var summary = TraySummary{};
        const status = self.read_tray_summary_fn(self.handle, &summary);
        if (status == status_ok) return summary;
        if (status == status_unchanged) return error.EngineDataPending;
        return error.EngineReadFailed;
    }

    pub fn readTopicAlloc(self: *Runtime, allocator: std.mem.Allocator, topic: Topic) !?TopicPayload {
        const index: usize = @intFromEnum(topic);
        var required: usize = 0;
        var sequence: u64 = 0;
        const probe_status = self.read_topic_fn(
            self.handle,
            @intFromEnum(topic),
            self.topic_sequences[index],
            null,
            0,
            &required,
            &sequence,
        );
        if (probe_status == status_unchanged) return null;
        if (probe_status != status_buffer_too_small or required < 2 or required > 2 * 1024 * 1024) {
            return error.EngineTopicReadFailed;
        }

        const buffer = try allocator.alloc(u8, required);
        errdefer allocator.free(buffer);
        var filled_required = required;
        const read_status = self.read_topic_fn(
            self.handle,
            @intFromEnum(topic),
            self.topic_sequences[index],
            buffer.ptr,
            buffer.len,
            &filled_required,
            &sequence,
        );
        if (read_status == status_unchanged) {
            allocator.free(buffer);
            return null;
        }
        if (read_status != status_ok or filled_required < 2 or filled_required > buffer.len) {
            return error.EngineTopicReadFailed;
        }
        self.topic_sequences[index] = sequence;
        return .{ .bytes = buffer[0 .. filled_required - 1], .sequence = sequence };
    }

    pub fn setWindowVisible(self: *Runtime, visible: bool) !void {
        const profile = if (visible) profile_overview else profile_hidden;
        if (self.set_profile_fn(self.handle, profile) != status_ok) {
            return error.EngineProfileFailed;
        }
    }

    pub fn setView(self: *Runtime, visible: bool, section: u8) !void {
        const profile = if (!visible)
            profile_hidden
        else if (section == 0)
            profile_overview
        else if (section == 6)
            profile_processes
        else
            profile_foreground
        ;
        if (self.set_profile_fn(self.handle, profile) != status_ok) {
            return error.EngineProfileFailed;
        }
    }

    pub fn setProcessSort(self: *Runtime, sort: u32) !void {
        if (self.set_process_sort_fn(self.handle, sort) != status_ok) {
            return error.EngineProcessSortFailed;
        }
    }

    pub fn requestDriverScan(self: *Runtime) !void {
        if (self.request_driver_scan_fn(self.handle) != status_ok) {
            return error.EngineDriverScanFailed;
        }
    }

    pub fn readSettingsAlloc(self: *Runtime, allocator: std.mem.Allocator) ![]u8 {
        var required: usize = 0;
        const probe_status = self.read_settings_fn(null, 0, &required);
        if (probe_status != status_buffer_too_small or required < 2 or required > 256 * 1024) {
            return error.EngineSettingsReadFailed;
        }
        const buffer = try allocator.alloc(u8, required);
        errdefer allocator.free(buffer);
        var filled_required = required;
        if (self.read_settings_fn(buffer.ptr, buffer.len, &filled_required) != status_ok or
            filled_required < 2 or filled_required > buffer.len)
        {
            return error.EngineSettingsReadFailed;
        }
        return buffer[0 .. filled_required - 1];
    }

    pub fn writeSettings(self: *Runtime, json: []const u8) !void {
        if (json.len == 0 or json.len > 256 * 1024) return error.EngineSettingsWriteFailed;
        if (self.write_settings_fn(json.ptr, json.len) != status_ok) {
            return error.EngineSettingsWriteFailed;
        }
    }

    pub fn setLaunchAtLogin(self: *Runtime, enabled: bool, start_hidden: bool) !void {
        if (self.set_launch_at_login_fn(@intFromBool(enabled), @intFromBool(start_hidden)) != status_ok) {
            return error.EngineLaunchAtLoginFailed;
        }
    }

    pub fn requestExport(self: *Runtime, kind: ExportKind) !void {
        if (self.request_export_fn(self.handle, @intFromEnum(kind)) != status_ok) {
            return error.EngineExportRequestFailed;
        }
    }

    pub fn readExportStatusAlloc(self: *Runtime, allocator: std.mem.Allocator) ![]u8 {
        var required: usize = 0;
        const probe_status = self.read_export_status_fn(self.handle, null, 0, &required);
        if (probe_status != status_buffer_too_small or required < 2 or required > 16 * 1024) {
            return error.EngineExportStatusFailed;
        }
        const buffer = try allocator.alloc(u8, required);
        errdefer allocator.free(buffer);
        var filled_required = required;
        if (self.read_export_status_fn(self.handle, buffer.ptr, buffer.len, &filled_required) != status_ok or
            filled_required < 2 or filled_required > buffer.len)
        {
            return error.EngineExportStatusFailed;
        }
        return buffer[0 .. filled_required - 1];
    }
};

fn validateMetadata(allocator: std.mem.Allocator, metadata_fn: MetadataFn) !void {
    var required: usize = 0;
    _ = metadata_fn(null, 0, &required);
    if (required < 2 or required > 4096) return error.EngineMetadataInvalid;
    const buffer = try allocator.alloc(u8, required);
    defer allocator.free(buffer);
    if (metadata_fn(buffer.ptr, buffer.len, &required) != status_ok or required < 2) {
        return error.EngineMetadataInvalid;
    }
    const parsed = std.json.parseFromSlice(Metadata, allocator, buffer[0 .. required - 1], .{}) catch
        return error.EngineMetadataInvalid;
    defer parsed.deinit();
    const metadata = parsed.value;
    if (metadata.abi_version != expected_abi_version or
        metadata.schema_version != expected_schema_version or
        !std.mem.eql(u8, metadata.product, "SD-300") or
        !std.mem.eql(u8, metadata.product_version, expected_product_version) or
        !std.mem.eql(u8, metadata.target_os, @tagName(builtin.os.tag)) or
        !std.mem.eql(u8, metadata.target_arch, @tagName(builtin.cpu.arch)))
    {
        return error.EngineMetadataMismatch;
    }
}

fn engineFileName() []const u8 {
    return switch (builtin.os.tag) {
        .windows => "sd300_engine.dll",
        .macos => "libsd300_engine.dylib",
        .linux => "libsd300_engine.so",
        else => unreachable,
    };
}

const load_library_search_dll_load_dir: windows.DWORD = 0x00000100;
const load_library_search_system32: windows.DWORD = 0x00000800;

extern "kernel32" fn LoadLibraryExW(
    file_name: [*:0]const u16,
    file: ?windows.HANDLE,
    flags: windows.DWORD,
) callconv(.winapi) ?windows.HMODULE;
extern "kernel32" fn GetProcAddress(
    module: windows.HMODULE,
    name: [*:0]const u8,
) callconv(.winapi) ?windows.FARPROC;
extern "kernel32" fn FreeLibrary(module: windows.HMODULE) callconv(.winapi) windows.BOOL;

test "fast summary ABI is stable" {
    try std.testing.expectEqual(@as(usize, 48), @sizeOf(FastSummary));
    try std.testing.expectEqual(@as(usize, 8), @alignOf(FastSummary));
}
