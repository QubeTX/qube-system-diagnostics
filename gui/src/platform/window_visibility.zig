const std = @import("std");
const builtin = @import("builtin");

const windows = std.os.windows;

const windows_style_index: c_int = -16;
const windows_overlapped_style: c_long = 0x00CF0000;
const windows_infinite: windows.DWORD = 0xFFFF_FFFF;
const windows_wait_object_0: windows.DWORD = 0;
const windows_wm_quit: windows.UINT = 0x0012;
const windows_wm_ncdestroy: windows.UINT = 0x0082;
const windows_wm_sd300_open: windows.UINT = 0x8000 + 0x300;
const windows_open_subclass_id: usize = 0x5344_3330;
const windows_error_already_exists: windows.DWORD = 183;
const windows_event_modify_state: windows.DWORD = 0x0002;
const windows_sw_hide: c_int = 0;
const windows_sw_restore: c_int = 9;
const quit_event_name = std.unicode.utf8ToUtf16LeStringLiteral("Local\\SD300.Gui.Quit.v1");
const open_event_name = std.unicode.utf8ToUtf16LeStringLiteral("Local\\SD300.Gui.Open.v1");
const instance_mutex_name = std.unicode.utf8ToUtf16LeStringLiteral("Local\\SD300.Gui.Instance.v1");

var instance_mutex: ?windows.HANDLE = null;
var open_signal_event: ?windows.HANDLE = null;
var open_signal_thread: ?std.Thread = null;
var open_signal_stopping = std.atomic.Value(bool).init(false);
var open_message_route_ready = std.atomic.Value(bool).init(false);

const WindowsProbe = struct {
    process_id: windows.DWORD,
    found: bool = false,
    visible: bool = false,
    window: ?windows.HWND = null,
};

const WindowsSubclassProc = *const fn (windows.HWND, windows.UINT, usize, isize, usize, usize) callconv(.winapi) isize;

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
extern "user32" fn PostMessageW(window: windows.HWND, message: windows.UINT, wparam: usize, lparam: isize) callconv(.winapi) windows.BOOL;
extern "comctl32" fn SetWindowSubclass(window: windows.HWND, callback: WindowsSubclassProc, subclass_id: usize, reference_data: usize) callconv(.winapi) windows.BOOL;
extern "comctl32" fn RemoveWindowSubclass(window: windows.HWND, callback: WindowsSubclassProc, subclass_id: usize) callconv(.winapi) windows.BOOL;
extern "comctl32" fn DefSubclassProc(window: windows.HWND, message: windows.UINT, wparam: usize, lparam: isize) callconv(.winapi) isize;
extern "user32" fn PostThreadMessageW(
    thread_id: windows.DWORD,
    message: windows.UINT,
    wparam: usize,
    lparam: isize,
) callconv(.winapi) windows.BOOL;

extern fn sd300_main_window_visible() callconv(.c) c_int;
extern fn sd300_main_window_policy_hidden() callconv(.c) c_int;
extern fn sd300_main_window_show() callconv(.c) void;
extern fn sd300_main_window_hide() callconv(.c) void;
extern fn sd300_claim_unix_instance() callconv(.c) c_int;
extern fn sd300_model_open() callconv(.c) void;

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
    if (open_signal_thread != null) return;
    const event = CreateEventW(null, .FALSE, .FALSE, open_event_name) orelse return error.OpenSignalUnavailable;
    errdefer _ = CloseHandle(event);
    open_signal_stopping.store(false, .release);
    open_signal_event = event;
    open_signal_thread = try std.Thread.spawn(.{}, waitForWindowsOpenSignal, .{event});
}

/// Stop and join the singleton-open listener before app state is destroyed.
/// Waking its event also interrupts a listener that is waiting for the next
/// launch, while the atomic stops an in-flight UI-queue retry safely.
pub fn uninstallOpenSignal() void {
    if (comptime builtin.os.tag != .windows) return;
    open_signal_stopping.store(true, .release);
    if (open_signal_event) |event| _ = SetEvent(event);
    if (open_signal_thread) |thread| thread.join();
    open_signal_thread = null;
    if (open_signal_event) |event| _ = CloseHandle(event);
    open_signal_event = null;
}

/// Subclass the SDK window on its owning UI thread so singleton launches can
/// enter the typed model path with a normal window-bound message. The SDK pump
/// filters out thread-only messages while the window is policy-hidden.
pub fn installOpenMessageRoute() void {
    if (comptime builtin.os.tag != .windows) return;
    if (open_message_route_ready.load(.acquire)) return;
    const window = findWindowsMainWindow() orelse return;
    if (SetWindowSubclass(window, &sd300WindowSubclassProc, windows_open_subclass_id, 0).toBool()) {
        open_message_route_ready.store(true, .release);
    }
}

pub fn uninstallOpenMessageRoute() void {
    if (comptime builtin.os.tag != .windows) return;
    open_message_route_ready.store(false, .release);
    const window = findWindowsMainWindow() orelse return;
    _ = RemoveWindowSubclass(window, &sd300WindowSubclassProc, windows_open_subclass_id);
}

/// Pure retry policy seam: a requested open remains owned until the UI queue
/// accepts it, unless application teardown explicitly cancels the listener.
pub fn openRequestPending(queue_accepted: bool, stopping: bool) bool {
    return !queue_accepted and !stopping;
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

/// Whether the main window is alive but hidden by the close policy. This is
/// intentionally narrower than `mainWindowVisible() == false`: minimizing is
/// a background presentation state, not a request to quit.
pub fn mainWindowPolicyHidden() bool {
    return switch (builtin.os.tag) {
        .windows => windowsMainWindowPolicyHidden(),
        .macos => sd300_main_window_policy_hidden() != 0,
        .linux => false,
        else => @compileError("SD-300 GUI supports only Windows, macOS, and Linux"),
    };
}

/// Idempotently reveal and activate the retained main window. Native SDK also
/// receives the typed `showWindow` effect; this direct platform nudge closes a
/// race where a host-level policy hide has happened but the asynchronous frame
/// state has not yet marked the runtime window hidden.
pub fn showMainWindow() void {
    switch (builtin.os.tag) {
        .windows => {
            const window = findWindowsMainWindow() orelse return;
            _ = ShowWindow(window, windows_sw_restore);
            _ = SetForegroundWindow(window);
        },
        .macos => sd300_main_window_show(),
        .linux => {},
        else => @compileError("SD-300 GUI supports only Windows, macOS, and Linux"),
    }
}

/// Hide the retained window for the explicit managed startup route. The
/// Windows ready-thread remains installed as well so the first-present path
/// cannot race this immediate model-thread request.
pub fn hideMainWindow() void {
    switch (builtin.os.tag) {
        .windows => {
            const window = findWindowsMainWindow() orelse return;
            _ = ShowWindow(window, windows_sw_hide);
        },
        .macos => sd300_main_window_hide(),
        .linux => {},
        else => @compileError("SD-300 GUI supports only Windows, macOS, and Linux"),
    }
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

fn waitForWindowsOpenSignal(handle: windows.HANDLE) void {
    while (WaitForSingleObject(handle, windows_infinite) == windows_wait_object_0) {
        if (open_signal_stopping.load(.acquire)) return;
        // The SDK pump filters for its native window, so post the private Open
        // message only after initEffects has installed the UI-thread subclass.
        // Retain the request until that route exists or orderly shutdown wins.
        while (!open_signal_stopping.load(.acquire)) {
            if (!open_message_route_ready.load(.acquire)) {
                Sleep(10);
                continue;
            }
            const window = findWindowsMainWindow() orelse {
                Sleep(10);
                continue;
            };
            const accepted = PostMessageW(window, windows_wm_sd300_open, 0, 0).toBool();
            if (!openRequestPending(accepted, open_signal_stopping.load(.acquire))) break;
            Sleep(10);
        }
    }
}

fn sd300WindowSubclassProc(
    window: windows.HWND,
    message: windows.UINT,
    wparam: usize,
    lparam: isize,
    _: usize,
    _: usize,
) callconv(.winapi) isize {
    if (message == windows_wm_sd300_open) {
        sd300_model_open();
        return 0;
    }
    if (message == windows_wm_ncdestroy) {
        open_message_route_ready.store(false, .release);
        _ = RemoveWindowSubclass(window, &sd300WindowSubclassProc, windows_open_subclass_id);
    }
    return DefSubclassProc(window, message, wparam, lparam);
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

fn windowsMainWindowPolicyHidden() bool {
    const window = findWindowsMainWindow() orelse return false;
    return !IsWindowVisible(window).toBool() and !IsIconic(window).toBool();
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
