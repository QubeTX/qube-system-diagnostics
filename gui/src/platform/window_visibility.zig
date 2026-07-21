const std = @import("std");
const builtin = @import("builtin");

const windows = std.os.windows;

const windows_style_index: c_int = -16;
const windows_overlapped_style: c_long = 0x00CF0000;
const windows_infinite: windows.DWORD = 0xFFFF_FFFF;
const windows_wait_object_0: windows.DWORD = 0;
const windows_wm_quit: windows.UINT = 0x0012;
const windows_error_already_exists: windows.DWORD = 183;
const windows_event_modify_state: windows.DWORD = 0x0002;
const windows_sw_hide: c_int = 0;
const windows_sw_show: c_int = 5;
const windows_sw_restore: c_int = 9;
const quit_event_name = std.unicode.utf8ToUtf16LeStringLiteral("Local\\SD300.Gui.Quit.v1");
const open_event_name = std.unicode.utf8ToUtf16LeStringLiteral("Local\\SD300.Gui.Open.v1");
const instance_mutex_name = std.unicode.utf8ToUtf16LeStringLiteral("Local\\SD300.Gui.Instance.v1");

var instance_mutex: ?windows.HANDLE = null;

const WindowsProbe = struct {
    process_id: windows.DWORD,
    found: bool = false,
    visible: bool = false,
    window: ?windows.HWND = null,
};

extern "kernel32" fn GetCurrentProcessId() callconv(.winapi) windows.DWORD;
extern "kernel32" fn GetCurrentThreadId() callconv(.winapi) windows.DWORD;
extern "kernel32" fn GetLastError() callconv(.winapi) windows.DWORD;
extern "kernel32" fn Sleep(milliseconds: windows.DWORD) callconv(.winapi) void;
extern "kernel32" fn CreateMutexW(attributes: ?*anyopaque, initial_owner: windows.BOOL, name: [*:0]const u16) callconv(.winapi) ?windows.HANDLE;
extern "kernel32" fn ReleaseMutex(handle: windows.HANDLE) callconv(.winapi) windows.BOOL;
extern "kernel32" fn OpenEventW(desired_access: windows.DWORD, inherit_handle: windows.BOOL, name: [*:0]const u16) callconv(.winapi) ?windows.HANDLE;
extern "kernel32" fn SetEvent(handle: windows.HANDLE) callconv(.winapi) windows.BOOL;
extern "kernel32" fn CreateEventW(
    attributes: ?*anyopaque,
    manual_reset: windows.BOOL,
    initial_state: windows.BOOL,
    name: [*:0]const u16,
) callconv(.winapi) ?windows.HANDLE;
extern "kernel32" fn WaitForSingleObject(handle: windows.HANDLE, milliseconds: windows.DWORD) callconv(.winapi) windows.DWORD;
extern "kernel32" fn CloseHandle(handle: windows.HANDLE) callconv(.winapi) windows.BOOL;
extern "user32" fn EnumWindows(
    callback: *const fn (windows.HWND, windows.LPARAM) callconv(.winapi) windows.BOOL,
    parameter: windows.LPARAM,
) callconv(.winapi) windows.BOOL;
extern "user32" fn GetWindowThreadProcessId(
    window: windows.HWND,
    process_id: *windows.DWORD,
) callconv(.winapi) windows.DWORD;
extern "user32" fn GetWindowLongW(window: windows.HWND, index: c_int) callconv(.winapi) c_long;
extern "user32" fn IsWindowVisible(window: windows.HWND) callconv(.winapi) windows.BOOL;
extern "user32" fn IsIconic(window: windows.HWND) callconv(.winapi) windows.BOOL;
extern "user32" fn ShowWindow(window: windows.HWND, command: c_int) callconv(.winapi) windows.BOOL;
extern "user32" fn SetForegroundWindow(window: windows.HWND) callconv(.winapi) windows.BOOL;
extern "user32" fn PostThreadMessageW(
    thread_id: windows.DWORD,
    message: windows.UINT,
    wparam: usize,
    lparam: isize,
) callconv(.winapi) windows.BOOL;

extern fn sd300_main_window_visible() callconv(.c) c_int;
extern fn sd300_claim_unix_instance() callconv(.c) c_int;

/// Windows uses an explicit per-logon mutex because launching an `.exe`
/// directly has no OS application-identity arbitration. macOS LaunchServices
/// and Linux GtkApplication already route an ordinary second launch to the
/// existing bundle/application id.
pub fn claimSingleInstanceOrNotify() bool {
    if (comptime builtin.os.tag != .windows) {
        return sd300_claim_unix_instance() > 0;
    }
    const mutex = CreateMutexW(null, .TRUE, instance_mutex_name) orelse return true;
    if (GetLastError() != windows_error_already_exists) {
        instance_mutex = mutex;
        return true;
    }
    _ = CloseHandle(mutex);
    var attempt: usize = 0;
    while (attempt < 100) : (attempt += 1) {
        if (OpenEventW(windows_event_modify_state, .FALSE, open_event_name)) |event| {
            _ = SetEvent(event);
            _ = CloseHandle(event);
            break;
        }
        Sleep(50);
    }
    return false;
}

pub fn releaseSingleInstance() void {
    if (comptime builtin.os.tag != .windows) return;
    if (instance_mutex) |mutex| {
        _ = ReleaseMutex(mutex);
        _ = CloseHandle(mutex);
        instance_mutex = null;
    }
}

pub fn installOpenSignal() !void {
    if (comptime builtin.os.tag != .windows) return;
    const thread = try std.Thread.spawn(.{}, waitForWindowsOpenSignal, .{});
    thread.detach();
}

pub fn installStartupHide(start_hidden: bool) !void {
    if (!start_hidden) return;
    if (comptime builtin.os.tag == .windows) {
        const thread = try std.Thread.spawn(.{}, hideWindowsMainWindowWhenReady, .{});
        thread.detach();
    }
}

/// Returns whether at least one SD-300 top-level app window currently reaches
/// the glass. A platform probe that cannot find its window fails open so a
/// transient startup condition never strands collection in background mode.
pub fn mainWindowVisible() bool {
    return switch (builtin.os.tag) {
        .windows => windowsMainWindowVisible(),
        .macos => sd300_main_window_visible() != 0,
        .linux => true,
        else => @compileError("SD-300 GUI supports only Windows, macOS, and Linux"),
    };
}

/// Install a per-logon-session quit signal used by the proven owner before an
/// update or uninstall. The wait occurs off the UI thread and posts WM_QUIT to
/// the Native SDK message loop, allowing normal runtime/tray teardown without
/// a polling timer or renderer work.
pub fn installQuitSignal() !void {
    if (comptime builtin.os.tag != .windows) return;
    const main_thread_id = GetCurrentThreadId();
    const thread = try std.Thread.spawn(.{}, waitForWindowsQuitSignal, .{main_thread_id});
    thread.detach();
}

fn waitForWindowsQuitSignal(main_thread_id: windows.DWORD) void {
    // Manual reset releases every legacy duplicate process as well as the
    // normal single instance. The lifecycle caller keeps the event alive only
    // until it has proved that every GUI process has drained.
    const handle = CreateEventW(null, .TRUE, .FALSE, quit_event_name) orelse return;
    defer _ = CloseHandle(handle);
    if (WaitForSingleObject(handle, windows_infinite) == windows_wait_object_0) {
        _ = PostThreadMessageW(main_thread_id, windows_wm_quit, 0, 0);
    }
}

fn waitForWindowsOpenSignal() void {
    const handle = CreateEventW(null, .FALSE, .FALSE, open_event_name) orelse return;
    defer _ = CloseHandle(handle);
    while (WaitForSingleObject(handle, windows_infinite) == windows_wait_object_0) {
        showWindowsMainWindow();
    }
}

fn hideWindowsMainWindowWhenReady() void {
    var attempt: usize = 0;
    while (attempt < 200) : (attempt += 1) {
        if (findWindowsMainWindow()) |window| {
            _ = ShowWindow(window, windows_sw_hide);
            return;
        }
        Sleep(25);
    }
}

fn showWindowsMainWindow() void {
    const window = findWindowsMainWindow() orelse return;
    _ = ShowWindow(window, windows_sw_restore);
    _ = ShowWindow(window, windows_sw_show);
    _ = SetForegroundWindow(window);
}

fn findWindowsMainWindow() ?windows.HWND {
    var probe = WindowsProbe{ .process_id = GetCurrentProcessId() };
    _ = EnumWindows(probeWindowsWindow, @bitCast(@as(usize, @intFromPtr(&probe))));
    return probe.window;
}

fn windowsMainWindowVisible() bool {
    var probe = WindowsProbe{ .process_id = GetCurrentProcessId() };
    _ = EnumWindows(probeWindowsWindow, @bitCast(@as(usize, @intFromPtr(&probe))));
    return !probe.found or probe.visible;
}

fn probeWindowsWindow(window: windows.HWND, parameter: windows.LPARAM) callconv(.winapi) windows.BOOL {
    const address: usize = @bitCast(parameter);
    const probe: *WindowsProbe = @ptrFromInt(address);
    var owner_process_id: windows.DWORD = 0;
    _ = GetWindowThreadProcessId(window, &owner_process_id);
    if (owner_process_id != probe.process_id) return .TRUE;

    const style = GetWindowLongW(window, windows_style_index);
    if ((style & windows_overlapped_style) == 0) return .TRUE;

    probe.found = true;
    probe.window = window;
    if (IsWindowVisible(window).toBool() and !IsIconic(window).toBool()) {
        probe.visible = true;
        return .FALSE;
    }
    return .TRUE;
}
