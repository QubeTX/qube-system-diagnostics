const std = @import("std");
const native_sdk = @import("native_sdk");
const main = @import("main.zig");
const engine = @import("engine.zig");
const window_visibility = @import("platform/window_visibility.zig");

const canvas = native_sdk.canvas;
const testing = std.testing;

const AppMarkup = canvas.MarkupView(main.Model, main.Msg);

fn buildTree(arena: std.mem.Allocator, model: *const main.Model) !main.AppUi.Tree {
    var view = try AppMarkup.init(arena, main.app_markup);
    var ui = main.AppUi.init(arena);
    const node = view.build(&ui, model) catch |err| {
        if (err == error.MarkupBuild) {
            std.debug.print("app.native:{d}:{d}: {s}\n", .{ view.diagnostic.line, view.diagnostic.column, view.diagnostic.message });
        }
        return err;
    };
    return ui.finalize(node);
}

const render_bench_capacity: usize = 2048;
// The warmed-state scroll benchmark below drives fully populated Network,
// Processes, and Drivers sections, which emit more draw commands and layout
// nodes than the overview steady-state case. Give it a generous, heap-backed
// scratch so a large scrolled display list cannot silently overflow.
const warm_bench_capacity: usize = 6144;

/// Generic frame-plan scratch keyed by command capacity. The overview
/// steady-state benchmark keeps its 2048-slot buffers; the warmed-state
/// attribution benchmark below uses a 6144-slot instance. Test-only.
fn FrameScratchStorage(comptime cap: usize) type {
    return struct {
        const Self = @This();
        render_commands: [cap]canvas.RenderCommand = undefined,
        render_batches: [cap]canvas.RenderBatch = undefined,
        resources: [512]canvas.RenderResource = undefined,
        resource_cache_entries: [512]canvas.RenderResourceCacheEntry = undefined,
        resource_cache_actions: [1024]canvas.RenderResourceCacheAction = undefined,
        glyph_atlas_entries: [cap]canvas.GlyphAtlasEntry = undefined,
        glyph_atlas_cache_entries: [cap]canvas.GlyphAtlasCacheEntry = undefined,
        glyph_atlas_cache_actions: [cap * 2]canvas.GlyphAtlasCacheAction = undefined,
        text_layout_plans: [cap]canvas.TextLayoutPlan = undefined,
        text_layout_lines: [cap * 2]canvas.TextLine = undefined,
        text_layout_cache_entries: [cap]canvas.TextLayoutCacheEntry = undefined,
        text_layout_cache_actions: [cap * 2]canvas.TextLayoutCacheAction = undefined,
        changes: [cap]canvas.DiffChange = undefined,

        fn storage(self: *Self) canvas.CanvasFrameStorage {
            return .{
                .render_commands = &self.render_commands,
                .render_batches = &self.render_batches,
                .resources = &self.resources,
                .resource_cache_entries = &self.resource_cache_entries,
                .resource_cache_actions = &self.resource_cache_actions,
                .glyph_atlas_entries = &self.glyph_atlas_entries,
                .glyph_atlas_cache_entries = &self.glyph_atlas_cache_entries,
                .glyph_atlas_cache_actions = &self.glyph_atlas_cache_actions,
                .text_layout_plans = &self.text_layout_plans,
                .text_layout_lines = &self.text_layout_lines,
                .text_layout_cache_entries = &self.text_layout_cache_entries,
                .text_layout_cache_actions = &self.text_layout_cache_actions,
                .changes = &self.changes,
            };
        }
    };
}
const WarmRenderBenchFrameScratch = FrameScratchStorage(warm_bench_capacity);

const RenderBenchFrameScratch = struct {
    render_commands: [render_bench_capacity]canvas.RenderCommand = undefined,
    render_batches: [render_bench_capacity]canvas.RenderBatch = undefined,
    resources: [256]canvas.RenderResource = undefined,
    resource_cache_entries: [256]canvas.RenderResourceCacheEntry = undefined,
    resource_cache_actions: [512]canvas.RenderResourceCacheAction = undefined,
    glyph_atlas_entries: [render_bench_capacity]canvas.GlyphAtlasEntry = undefined,
    glyph_atlas_cache_entries: [render_bench_capacity]canvas.GlyphAtlasCacheEntry = undefined,
    glyph_atlas_cache_actions: [render_bench_capacity * 2]canvas.GlyphAtlasCacheAction = undefined,
    text_layout_plans: [render_bench_capacity]canvas.TextLayoutPlan = undefined,
    text_layout_lines: [render_bench_capacity * 2]canvas.TextLine = undefined,
    text_layout_cache_entries: [render_bench_capacity]canvas.TextLayoutCacheEntry = undefined,
    text_layout_cache_actions: [render_bench_capacity * 2]canvas.TextLayoutCacheAction = undefined,
    changes: [render_bench_capacity]canvas.DiffChange = undefined,

    fn storage(self: *RenderBenchFrameScratch) canvas.CanvasFrameStorage {
        return .{
            .render_commands = &self.render_commands,
            .render_batches = &self.render_batches,
            .resources = &self.resources,
            .resource_cache_entries = &self.resource_cache_entries,
            .resource_cache_actions = &self.resource_cache_actions,
            .glyph_atlas_entries = &self.glyph_atlas_entries,
            .glyph_atlas_cache_entries = &self.glyph_atlas_cache_entries,
            .glyph_atlas_cache_actions = &self.glyph_atlas_cache_actions,
            .text_layout_plans = &self.text_layout_plans,
            .text_layout_lines = &self.text_layout_lines,
            .text_layout_cache_entries = &self.text_layout_cache_entries,
            .text_layout_cache_actions = &self.text_layout_cache_actions,
            .changes = &self.changes,
        };
    }
};

fn buildRenderBenchDisplayList(
    arena: std.mem.Allocator,
    model: *const main.Model,
    builder: *canvas.Builder,
    nodes: []canvas.WidgetLayoutNode,
) !canvas.DisplayList {
    const size = native_sdk.geometry.SizeF.init(1180, 760);
    const tokens = main.qubeTokens(model);
    const tree = try buildTree(arena, model);
    const layout = try canvas.layoutWidgetTreeWithTokens(
        tree.root,
        native_sdk.geometry.RectF.init(0, 0, size.width, size.height),
        tokens,
        nodes,
    );
    try main.warmCarbonChrome(model, builder, size, tokens);
    try layout.emitDisplayList(builder, tokens);
    return builder.displayList();
}

fn renderBenchLessThan(_: void, a: u64, b: u64) bool {
    return a < b;
}

const RenderBenchDamage = struct {
    rects: [canvas.max_canvas_frame_dirty_rects]native_sdk.geometry.RectF = undefined,
    count: usize = 0,
    bounds: ?native_sdk.geometry.RectF = null,

    fn add(self: *RenderBenchDamage, rect: native_sdk.geometry.RectF) void {
        const normalized = rect.normalized();
        if (normalized.isEmpty()) return;
        self.bounds = if (self.bounds) |bounds|
            native_sdk.geometry.RectF.unionWith(bounds, normalized)
        else
            normalized;
        var merged = normalized;
        self.absorbIntersecting(&merged);
        if (self.count >= self.rects.len) {
            var best_index: usize = 0;
            var best_cost: f32 = std.math.floatMax(f32);
            for (self.rects[0..self.count], 0..) |cluster, index| {
                const candidate = native_sdk.geometry.RectF.unionWith(cluster, merged);
                const cost = candidate.width * candidate.height - cluster.width * cluster.height;
                if (cost < best_cost) {
                    best_cost = cost;
                    best_index = index;
                }
            }
            merged = native_sdk.geometry.RectF.unionWith(self.rects[best_index], merged);
            self.remove(best_index);
            self.absorbIntersecting(&merged);
        }
        self.rects[self.count] = merged;
        self.count += 1;
    }

    fn absorbIntersecting(self: *RenderBenchDamage, merged: *native_sdk.geometry.RectF) void {
        var index: usize = 0;
        while (index < self.count) {
            if (!self.rects[index].intersects(merged.*)) {
                index += 1;
                continue;
            }
            merged.* = native_sdk.geometry.RectF.unionWith(self.rects[index], merged.*);
            self.remove(index);
            index = 0;
        }
    }

    fn remove(self: *RenderBenchDamage, index: usize) void {
        var shift = index;
        while (shift + 1 < self.count) : (shift += 1) self.rects[shift] = self.rects[shift + 1];
        self.count -= 1;
    }

    fn slice(self: *const RenderBenchDamage) []const native_sdk.geometry.RectF {
        return self.rects[0..self.count];
    }
};

fn renderBenchDamage(changes: []const canvas.DiffChange) RenderBenchDamage {
    var damage = RenderBenchDamage{};
    for (changes) |change| {
        if (change.dirty_bounds) |bounds| damage.add(bounds);
    }
    return damage;
}

fn findByText(widget: canvas.Widget, kind: canvas.WidgetKind, text: []const u8) ?canvas.Widget {
    if (widget.kind == kind and std.mem.eql(u8, widget.text, text)) return widget;
    for (widget.children) |child| {
        if (findByText(child, kind, text)) |found| return found;
    }
    return null;
}

fn expectByText(widget: canvas.Widget, kind: canvas.WidgetKind, text: []const u8) !canvas.Widget {
    return findByText(widget, kind, text) orelse {
        std.debug.print("no {t} with text \"{s}\" in the SD-300 view\n", .{ kind, text });
        return error.WidgetNotFound;
    };
}

test "a fast summary updates the native overview projection" {
    var model = main.initialModel();
    main.applySummary(&model, engine.FastSummary{
        .sequence = 42,
        .cpu_percent = 18.25,
        .memory_percent = 62.5,
        .memory_used_bytes = 16 * 1024 * 1024 * 1024,
        .memory_total_bytes = 32 * 1024 * 1024 * 1024,
        .logical_processors = 22,
        .warning_count = 3,
    });

    try testing.expect(model.engine_ready);
    try testing.expectEqual(@as(u64, 42), model.sequence);
    try testing.expectEqual(@as(f64, 18.25), model.cpu_percent);
    try testing.expectEqual(@as(f64, 16), model.memory_used_gib);
    try testing.expectEqualStrings("CPU · 18.3%", model.tray_cpu_buffer.text());
    try testing.expectEqualStrings("Memory · 62.5%", model.tray_memory_buffer.text());
    try testing.expectEqual(@as(f64, 18.25), model.cpu_history[model.cpu_history.len - 1]);
    try testing.expect(model.overview_topic_meta.ready);
    try testing.expectEqualStrings("fast-summary", model.overview_topic_meta.topic());
    try testing.expectEqualStrings("SD-300 platform CPU and memory collectors", model.overview_topic_meta.provenance());
    try testing.expectEqual(@as(u64, 42), model.overview_topic_meta.sequence);

    var arena_state = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena_state.deinit();
    const tree = try buildTree(arena_state.allocator(), &model);
    _ = try expectByText(tree.root, .badge, "LIVE · SAMPLE 42");
    _ = try expectByText(tree.root, .text, "18.3%");
    _ = try expectByText(tree.root, .text, "16.0 GiB used of 32.0 GiB");
    _ = try expectByText(tree.root, .badge, "3 WARNINGS");
}

test "re-reading one fast sequence does not invent another history sample" {
    var model = main.initialModel();
    const summary = engine.FastSummary{
        .sequence = 9,
        .captured_unix_ms = 1_777_777_777_000,
        .cpu_percent = 37.5,
        .memory_percent = 61.25,
        .memory_used_bytes = 10 * 1024 * 1024 * 1024,
        .memory_total_bytes = 16 * 1024 * 1024 * 1024,
        .logical_processors = 8,
        .warning_count = 0,
    };
    main.applySummary(&model, summary);
    const cpu_history = model.cpu_history;
    const memory_history = model.memory_history;

    main.applySummary(&model, summary);

    try testing.expectEqualSlices(f64, &cpu_history, &model.cpu_history);
    try testing.expectEqualSlices(f64, &memory_history, &model.memory_history);
    try testing.expectEqual(@as(u64, 1_777_777_777_000), model.overview_topic_meta.captured_unix_ms);
}

test "top sample state becomes stale until sequence and capture advance" {
    var test_clock = native_sdk.TestClock{};
    test_clock.setWallMs(1_777_777_777_000);
    var model = main.initialModel();
    model.clock = test_clock.clock();
    const summary = engine.FastSummary{
        .sequence = 9,
        .captured_unix_ms = 1_777_777_777_000,
        .cpu_percent = 37.5,
        .memory_percent = 61.25,
        .memory_used_bytes = 10 * 1024 * 1024 * 1024,
        .memory_total_bytes = 16 * 1024 * 1024 * 1024,
        .logical_processors = 8,
        .warning_count = 0,
    };
    main.applySummary(&model, summary);
    try testing.expect(model.summaryLive());

    test_clock.advanceMs(2501);
    main.applySummary(&model, summary);
    try testing.expect(model.summaryStale());
    try testing.expect(!model.summaryLive());

    var arena_stale = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena_stale.deinit();
    const stale_tree = try buildTree(arena_stale.allocator(), &model);
    _ = try expectByText(stale_tree.root, .badge, "STALE · SAMPLE 9");
    try testing.expect(findByText(stale_tree.root, .badge, "LIVE · SAMPLE 9") == null);

    var next = summary;
    next.sequence = 10;
    next.captured_unix_ms += 2501;
    main.applySummary(&model, next);
    try testing.expect(model.summaryLive());
    try testing.expect(!model.summaryStale());
}

test "top sample state exposes collector failure without claiming live" {
    var model = main.initialModel();
    main.applySummary(&model, .{
        .sequence = 7,
        .cpu_percent = 10,
        .memory_percent = 20,
        .memory_total_bytes = 16 * 1024 * 1024 * 1024,
        .logical_processors = 8,
    });
    main.markFastSummaryFailed(&model);
    try testing.expect(model.summaryFailed());
    try testing.expect(!model.summaryLive());

    var arena_state = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena_state.deinit();
    const tree = try buildTree(arena_state.allocator(), &model);
    _ = try expectByText(tree.root, .badge, "COLLECTOR FAILED · LAST SAMPLE 7");
    try testing.expect(findByText(tree.root, .badge, "LIVE · SAMPLE 7") == null);
}

test "instrument trace remains a bounded real CPU history" {
    var model = main.initialModel();
    var sequence: u64 = 1;
    while (sequence <= 75) : (sequence += 1) {
        main.applySummary(&model, engine.FastSummary{
            .sequence = sequence,
            .cpu_percent = @floatFromInt(sequence),
            .memory_percent = 50,
            .memory_used_bytes = 8 * 1024 * 1024 * 1024,
            .memory_total_bytes = 16 * 1024 * 1024 * 1024,
            .logical_processors = 8,
            .warning_count = 0,
        });
    }

    try testing.expectEqual(@as(usize, 60), model.cpu_history.len);
    try testing.expectEqual(@as(f64, 16), model.cpu_history[0]);
    try testing.expectEqual(@as(f64, 75), model.cpu_history[model.cpu_history.len - 1]);
}

test "all nine navigation messages select exactly one real section" {
    const messages = [_]main.Msg{
        .select_overview,
        .select_cpu,
        .select_memory,
        .select_disk,
        .select_gpu,
        .select_network,
        .select_processes,
        .select_thermals,
        .select_drivers,
    };
    var model = main.initialModel();
    var fx = main.Effects.init(testing.allocator);
    defer fx.deinit();
    fx.executor = .fake;

    for (messages, 0..) |message, section| {
        main.update(&model, .{ .content_scrolled = .{ .offset = 320, .viewport_extent = 600, .content_extent = 1800 } }, &fx);
        try testing.expectEqual(@as(f64, 320), model.scroll_top);
        main.update(&model, message, &fx);
        try testing.expectEqual(@as(u8, @intCast(section)), model.active_section);
        try testing.expectEqual(@as(f64, 0), model.scroll_top);
    }
}

test "drivers view exposes a real asynchronous rescan action" {
    var arena_state = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena_state.deinit();

    var model = main.initialModel();
    model.active_section = 8;
    const tree = try buildTree(arena_state.allocator(), &model);
    _ = try expectByText(tree.root, .button, "Scan again");
}

test "process table defaults to a focused primary set and expands on demand" {
    var model = main.initialModel();
    model.active_section = 6;
    model.detail.process_count = 16;
    for (0..model.detail.process_count) |index| {
        model.detail.process_rows[index].id = @intCast(index + 1);
        model.detail.process_rows[index].pid = @intCast(index + 1);
    }
    try testing.expectEqual(@as(usize, 8), model.visibleProcessCount());
    try testing.expectEqualStrings("Show all matches", model.processToggleLabel());

    var fx = main.Effects.init(testing.allocator);
    defer fx.deinit();
    fx.executor = .fake;
    main.update(&model, .toggle_process_rows, &fx);
    try testing.expectEqual(@as(usize, 16), model.visibleProcessCount());
    try testing.expectEqualStrings("Show primary 8", model.processToggleLabel());
}

test "process consumers sort immediately by CPU memory PID and name" {
    var model = main.initialModel();
    model.active_section = 6;
    model.detail.process_count = 3;
    const names = [_][]const u8{ "Zulu", "Alpha", "Bravo" };
    const cpu = [_]f64{ 4, 20, 10 };
    const memory = [_]f64{ 900, 100, 500 };
    for (0..3) |index| {
        model.detail.process_rows[index].id = @intCast(index + 1);
        model.detail.process_rows[index].pid = @intCast(index + 1);
        model.detail.process_rows[index].friendly_buffer.set(names[index]);
        model.detail.process_rows[index].cpu_percent = cpu[index];
        model.detail.process_rows[index].memory_mib = memory[index];
    }
    var fx = main.Effects.init(testing.allocator);
    defer fx.deinit();
    fx.executor = .fake;

    main.update(&model, .sort_process_cpu, &fx);
    try testing.expectEqualStrings("Alpha", model.processes()[0].friendlyName());
    main.update(&model, .sort_process_memory, &fx);
    try testing.expectEqualStrings("Zulu", model.processes()[0].friendlyName());
    main.update(&model, .sort_process_pid, &fx);
    try testing.expectEqualStrings("Zulu", model.processes()[0].friendlyName());
    main.update(&model, .sort_process_name, &fx);
    try testing.expectEqualStrings("Alpha", model.processes()[0].friendlyName());

    try testing.expectEqual(@as(u32, 0), @intFromEnum(main.ProcessSort.cpu));
    try testing.expectEqual(@as(u32, 1), @intFromEnum(main.ProcessSort.memory));
    try testing.expectEqual(@as(u32, 2), @intFromEnum(main.ProcessSort.pid));
    try testing.expectEqual(@as(u32, 3), @intFromEnum(main.ProcessSort.name));

    model.detail.fast_ready = true;
    var arena_state = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena_state.deinit();
    const tree = try buildTree(arena_state.allocator(), &model);
    _ = try expectByText(tree.root, .button, "PID");
}

test "process filtering is bounded case insensitive and reports matched rows" {
    var model = main.initialModel();
    model.active_section = 6;
    model.detail.process_count = 3;
    const names = [_][]const u8{ "SD-300 Native Monitor", "Explorer", "Audio Service" };
    const states = [_][]const u8{ "Run", "Sleep", "Stopped" };
    for (0..3) |index| {
        model.detail.process_rows[index].id = @intCast(index + 1);
        model.detail.process_rows[index].pid = @intCast(index + 1);
        model.detail.process_rows[index].friendly_buffer.set(names[index]);
        model.detail.process_rows[index].name_buffer.set(names[index]);
        model.detail.process_rows[index].status_buffer.set(states[index]);
    }

    model.process_filter_buffer.set("service");
    main.rebuildProcessFilter(&model);
    try testing.expectEqual(@as(usize, 1), model.processMatchCount());
    try testing.expectEqualStrings("Audio Service", model.processes()[0].friendlyName());

    model.process_filter_buffer.set("RUN");
    main.rebuildProcessFilter(&model);
    try testing.expectEqual(@as(usize, 1), model.processMatchCount());
    try testing.expectEqualStrings("SD-300 Native Monitor", model.processes()[0].friendlyName());
}

test "connection and driver filters operate on bounded projections" {
    var model = main.initialModel();
    model.detail.connection_count = 2;
    model.detail.connection_rows[0].id = 1;
    model.detail.connection_rows[0].pid = 42;
    model.detail.connection_rows[0].remote_buffer.set("203.0.113.10");
    model.detail.connection_rows[0].process_buffer.set("browser.exe");
    model.detail.connection_rows[1].id = 2;
    model.detail.connection_rows[1].pid = 77;
    model.detail.connection_rows[1].state_buffer.set("listening");
    model.connection_filter_buffer.set("browser");
    main.rebuildConnectionFilter(&model);
    try testing.expectEqual(@as(usize, 1), model.connectionMatchCount());
    try testing.expectEqual(@as(u32, 42), model.connections()[0].pid);

    model.detail.driver_count = 3;
    const categories = [_][]const u8{ "network", "storage", "system" };
    for (0..3) |index| {
        model.detail.driver_rows[index].id = @intCast(index + 1);
        model.detail.driver_rows[index].category_buffer.set(categories[index]);
        model.detail.driver_rows[index].name_buffer.set(categories[index]);
    }
    model.detail.driver_rows[1].attention = true;
    model.driver_attention_only = true;
    main.rebuildDriverFilter(&model);
    try testing.expectEqual(@as(usize, 1), model.driverMatchCount());
    try testing.expectEqualStrings("storage", model.drivers()[0].category());

    model.driver_attention_only = false;
    model.driver_filter_buffer.set("SYSTEM");
    main.rebuildDriverFilter(&model);
    try testing.expectEqual(@as(usize, 1), model.driverMatchCount());
    try testing.expectEqualStrings("system", model.drivers()[0].category());
}

test "audience mode changes interpretation without touching terminal defaults" {
    var model = main.initialModel();
    model.cpu_percent = 92;
    model.memory_percent = 80;
    try testing.expect(model.userMode());
    try testing.expect(std.mem.indexOf(u8, model.cpuAssessment(), "critical") != null);
    try testing.expect(std.mem.indexOf(u8, model.memoryAssessment(), "elevated") != null);

    var fx = main.Effects.init(testing.allocator);
    defer fx.deinit();
    fx.executor = .fake;
    main.update(&model, .toggle_audience_mode, &fx);
    try testing.expect(model.technicianMode());
}

test "tray commands map to real show and graceful quit effects" {
    try testing.expect(main.onCommand("app.open").? == .open_window);
    try testing.expect(main.onCommand("app.quit").? == .quit_app);
    try testing.expect(main.onCommand("app.unknown") == null);

    var model = main.initialModel();
    var fx = main.Effects.init(testing.allocator);
    defer fx.deinit();
    fx.executor = .fake;

    main.update(&model, .open_window, &fx);
    var actions = fx.windowActionState();
    try testing.expectEqual(@as(u32, 1), actions.show_count);
    try testing.expectEqualStrings("main", actions.lastLabel());

    main.update(&model, .quit_app, &fx);
    actions = fx.windowActionState();
    try testing.expectEqual(@as(u32, 1), actions.quit_count);
}

test "close-policy hide quits only when tray is disabled" {
    try testing.expect(main.shouldQuitForHiddenWindow(false, true));
    try testing.expect(!main.shouldQuitForHiddenWindow(true, true));
    try testing.expect(!main.shouldQuitForHiddenWindow(false, false));
}

test "close-policy quit consults startup tray presence, not the live setting" {
    // Session-effective tray presence is captured once from persisted settings,
    // exactly like the effective_tray decision that gates the status item.
    var tray_off = main.initialModelWithSettings(.{ .gui = .{ .tray_enabled = false } });
    try testing.expect(!tray_off.tray_session_active);

    const tray_supported = tray_off.trayAvailable();

    var tray_on = main.initialModelWithSettings(.{ .gui = .{ .tray_enabled = true } });
    // A tray icon exists this session only where the platform supports it.
    try testing.expectEqual(tray_supported, tray_on.tray_session_active);

    var fx = main.Effects.init(testing.allocator);
    defer fx.deinit();
    fx.executor = .fake;

    // BUG 1 forward case: launch tray-OFF, then enable tray in settings. The
    // pending preference flips on and RESTART REQUIRED is armed, but this
    // session still has NO icon. A policy-hidden close must therefore still
    // quit — reading the live setting would strand a hidden, icon-less ghost.
    main.update(&tray_off, .toggle_tray, &fx);
    if (tray_supported) {
        try testing.expect(tray_off.tray_enabled);
        try testing.expect(!tray_off.tray_session_active);
        try testing.expect(tray_off.settings_restart_required);
        try testing.expect(main.shouldQuitForHiddenWindow(tray_off.tray_session_active, true));
        // The pre-fix input would have wrongly refused to quit.
        try testing.expect(!main.shouldQuitForHiddenWindow(tray_off.tray_enabled, true));
    }

    // BUG 1 symmetric case: launch tray-ON, then disable tray in settings. The
    // icon is still live this session, so a policy-hidden close must NOT quit
    // (the tray can still reopen it) even though the preference is now off.
    main.update(&tray_on, .toggle_tray, &fx);
    if (tray_supported) {
        try testing.expect(!tray_on.tray_enabled);
        try testing.expect(tray_on.tray_session_active);
        try testing.expect(!main.shouldQuitForHiddenWindow(tray_on.tray_session_active, true));
        // The pre-fix input would have wrongly quit and killed a live icon.
        try testing.expect(main.shouldQuitForHiddenWindow(tray_on.tray_enabled, true));
    }
}

test "minimized window keeps foreground cadence via policy-hidden not visibility" {
    const Probe = struct { visible: bool, policy_hidden: bool };
    // Platform window-state truth table for the states the refresh tick sees,
    // as (raw_visible, policy_hidden) reported by the window_visibility probes.
    const on_screen = Probe{ .visible = true, .policy_hidden = false };
    const minimized = Probe{ .visible = false, .policy_hidden = false };
    const tray_hidden = Probe{ .visible = false, .policy_hidden = true };

    // The fix derives presentation-active = !policy_hidden and feeds THAT to the
    // cadence choice, so a minimized window (not policy-hidden) holds the 1 s
    // foreground cadence and restore from the taskbar is instantly fresh.
    try testing.expectEqual(@as(u64, 1000), main.refreshIntervalMs(!on_screen.policy_hidden));
    try testing.expectEqual(@as(u64, 1000), main.refreshIntervalMs(!minimized.policy_hidden));
    try testing.expectEqual(@as(u64, 30000), main.refreshIntervalMs(!tray_hidden.policy_hidden));

    // Regression guard: the old raw-visibility derivation dropped a minimized
    // window (raw visible = false) to the 30 s hidden cadence — the stale-on-
    // restore bug this fix removes.
    try testing.expectEqual(@as(u64, 30000), main.refreshIntervalMs(minimized.visible));

    // The quit path is unaffected: a minimized window is never policy-hidden and
    // so can never satisfy the quit predicate, while a real tray-hidden window
    // still can when no session tray icon exists.
    try testing.expect(!main.shouldQuitForHiddenWindow(false, minimized.policy_hidden));
    try testing.expect(main.shouldQuitForHiddenWindow(false, tray_hidden.policy_hidden));
}

test "external singleton open enters the typed model update path" {
    var model = main.initialModel();
    model.window_visible = false;
    var fx = main.Effects.init(testing.allocator);
    defer fx.deinit();
    fx.executor = .fake;

    main.sd300_model_open();
    main.initEffects(&model, &fx);

    try testing.expect(model.window_visible);
    const actions = fx.windowActionState();
    try testing.expectEqual(@as(u32, 1), actions.show_count);
    try testing.expectEqualStrings("main", actions.lastLabel());
}

test "singleton open remains pending beyond the former startup retry budget" {
    var pending = true;
    for (0..1000) |_| pending = window_visibility.openRequestPending(false, false);
    try testing.expect(pending);
    try testing.expect(!window_visibility.openRequestPending(true, false));
    try testing.expect(!window_visibility.openRequestPending(false, true));
}

test "tray state exposes live summaries and explicit Open and Quit" {
    var model = main.initialModel();
    main.applySummary(&model, engine.FastSummary{
        .sequence = 8,
        .cpu_percent = 7.25,
        .memory_percent = 44.5,
        .memory_used_bytes = 8 * 1024 * 1024 * 1024,
        .memory_total_bytes = 16 * 1024 * 1024 * 1024,
        .logical_processors = 8,
        .warning_count = 0,
    });
    main.applyTraySummary(&model, engine.TraySummary{
        .sequence = 1,
        .gpu_percent = 12.5,
        .storage_free_percent = 43.25,
        .gpu_available = 1,
        .storage_available = 1,
        .disk_health = 1,
    });
    var scratch: main.NativeApp.StatusItemScratch = .{};
    const state = main.statusItem(&model, &scratch);
    try testing.expectEqualStrings("SD", state.title);
    try testing.expectEqual(@as(usize, 9), state.items.len);
    try testing.expectEqualStrings("CPU · 7.3%", state.items[0].label);
    try testing.expect(!state.items[0].enabled);
    try testing.expectEqualStrings("Memory · 44.5%", state.items[1].label);
    try testing.expectEqualStrings("GPU · 12.5%", state.items[2].label);
    try testing.expectEqualStrings("Storage free · 43.3%", state.items[3].label);
    try testing.expectEqualStrings("Disk health · good", state.items[4].label);
    try testing.expectEqualStrings("app.open", state.items[6].command);
    try testing.expectEqualStrings("app.quit", state.items[8].command);
}

test "boot arms exactly one live foreground render timer" {
    var model = main.initialModel();
    var fx = main.Effects.init(testing.allocator);
    defer fx.deinit();
    fx.executor = .fake;

    main.initEffects(&model, &fx);
    try testing.expectEqual(@as(usize, 1), fx.pendingTimerCount());
    const timer = fx.pendingTimerAt(0).?;
    try testing.expectEqual(@as(u64, 1000), timer.interval_ms);
    try testing.expectEqual(native_sdk.TimerMode.repeating, timer.mode);
    try testing.expect(!model.engine_ready);
    try testing.expect(std.mem.indexOf(u8, model.status(), "Engine unavailable") != null);
}

test "hidden tray cadence is cheaper than the foreground visual cadence" {
    try testing.expectEqual(@as(u64, 1000), main.refreshIntervalMs(true));
    try testing.expectEqual(@as(u64, 30000), main.refreshIntervalMs(false));
}

test "package self-test is an explicit internal command" {
    try testing.expect(main.isSelfTest(&.{ "sd300-gui", "--self-test", "--json" }));
    try testing.expect(!main.isSelfTest(&.{"sd300-gui"}));
    try testing.expect(!main.isSelfTest(&.{ "sd300-gui", "--json" }));
}

test "hidden startup requires both lifecycle arguments" {
    try testing.expect(main.startsHidden(&.{ "sd300-gui", "--startup", "--hidden" }));
    try testing.expect(!main.startsHidden(&.{ "sd300-gui", "--hidden" }));
    try testing.expect(!main.startsHidden(&.{ "sd300-gui", "--startup" }));
}

test "the overview lays out at the production window size" {
    var arena_state = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena_state.deinit();

    var model = main.initialModel();
    const tree = try buildTree(arena_state.allocator(), &model);
    var nodes: [192]canvas.WidgetLayoutNode = undefined;
    const layout = try canvas.layoutWidgetTree(
        tree.root,
        native_sdk.geometry.RectF.init(0, 0, 1180, 760),
        &nodes,
    );
    try testing.expect(layout.nodes.len > 20);
    _ = try expectByText(tree.root, .text, "System overview");
    _ = try expectByText(tree.root, .button, "Refresh");
}

test "headless SD-300 1 Hz renderer benchmark" {
    if (comptime !@import("builtin").link_libc) return error.SkipZigTest;
    if (std.c.getenv("SD300_RENDER_BENCH") == null) return error.SkipZigTest;

    const allocator = testing.allocator;
    const width: usize = 1180;
    const height: usize = 760;
    const direct_sample_count: usize = 6;
    const cached_sample_count: usize = 30;

    var model_a = main.initialModel();
    main.applySummary(&model_a, .{
        .sequence = 62,
        .cpu_percent = 37.4,
        .memory_percent = 68.2,
        .memory_used_bytes = 21 * 1024 * 1024 * 1024,
        .memory_total_bytes = 32 * 1024 * 1024 * 1024,
        .logical_processors = 22,
        .warning_count = 0,
    });
    var model_b = model_a;
    main.applySummary(&model_b, .{
        .sequence = 63,
        .cpu_percent = 78.0,
        .memory_percent = 69.6,
        .memory_used_bytes = 22 * 1024 * 1024 * 1024,
        .memory_total_bytes = 32 * 1024 * 1024 * 1024,
        .logical_processors = 22,
        .warning_count = 0,
    });

    var arena_a = std.heap.ArenaAllocator.init(allocator);
    defer arena_a.deinit();
    var arena_b = std.heap.ArenaAllocator.init(allocator);
    defer arena_b.deinit();
    const commands_a = try allocator.alloc(canvas.CanvasCommand, render_bench_capacity);
    defer allocator.free(commands_a);
    const commands_b = try allocator.alloc(canvas.CanvasCommand, render_bench_capacity);
    defer allocator.free(commands_b);
    const builder_a = try allocator.create(canvas.Builder);
    defer allocator.destroy(builder_a);
    builder_a.* = canvas.Builder.init(commands_a);
    const builder_b = try allocator.create(canvas.Builder);
    defer allocator.destroy(builder_b);
    builder_b.* = canvas.Builder.init(commands_b);
    const nodes_a = try allocator.alloc(canvas.WidgetLayoutNode, 512);
    defer allocator.free(nodes_a);
    const nodes_b = try allocator.alloc(canvas.WidgetLayoutNode, 512);
    defer allocator.free(nodes_b);

    const build_start = std.Io.Timestamp.now(testing.io, .real).nanoseconds;
    const list_a = try buildRenderBenchDisplayList(arena_a.allocator(), &model_a, builder_a, nodes_a);
    const list_b = try buildRenderBenchDisplayList(arena_b.allocator(), &model_b, builder_b, nodes_b);
    const build_ns = std.Io.Timestamp.now(testing.io, .real).nanoseconds - build_start;

    const initial_scratch = try allocator.create(RenderBenchFrameScratch);
    defer allocator.destroy(initial_scratch);
    const ab_scratch = try allocator.create(RenderBenchFrameScratch);
    defer allocator.destroy(ab_scratch);
    const ba_scratch = try allocator.create(RenderBenchFrameScratch);
    defer allocator.destroy(ba_scratch);
    const frame_options = canvas.CanvasFrameOptions{
        .surface_size = native_sdk.geometry.SizeF.init(width, height),
        .scale = 1,
    };
    const plan_start = std.Io.Timestamp.now(testing.io, .real).nanoseconds;
    const initial_frame = try list_a.framePlan(null, frame_options, initial_scratch.storage());
    const frame_ab = try list_b.framePlan(list_a, frame_options, ab_scratch.storage());
    const frame_ba = try list_a.framePlan(list_b, frame_options, ba_scratch.storage());
    const plan_ns = std.Io.Timestamp.now(testing.io, .real).nanoseconds - plan_start;
    var diff_ab_storage: [render_bench_capacity]canvas.DiffChange = undefined;
    var diff_ba_storage: [render_bench_capacity]canvas.DiffChange = undefined;
    const damage_ab = renderBenchDamage(try canvas.DisplayList.diff(list_a, list_b, &diff_ab_storage));
    const damage_ba = renderBenchDamage(try canvas.DisplayList.diff(list_b, list_a, &diff_ba_storage));
    try testing.expect(damage_ab.count >= 2);
    try testing.expect(damage_ba.count >= 2);

    var makira_face = try canvas.font_ttf.Face.parse(@embedFile("fonts/Makira-Regular.ttf"));
    var plex_face = try canvas.font_ttf.Face.parse(@embedFile("fonts/IBMPlexMono-Regular.ttf"));
    const tokens = main.qubeTokens(&model_a);
    const fonts = [_]canvas.ReferenceFont{
        .{ .id = tokens.typography.font_id, .face = &makira_face },
        .{ .id = tokens.typography.mono_font_id, .face = &plex_face },
    };
    const pixel_len = width * height * 4;
    const clear = tokens.colors.background;

    const direct_pixels = try allocator.alloc(u8, pixel_len);
    defer allocator.free(direct_pixels);
    const direct_surface = (try canvas.ReferenceRenderSurface.init(width, height, direct_pixels)).withFonts(&fonts);
    try direct_surface.renderPass(initial_frame.renderPass(), clear);
    var direct_times: [direct_sample_count]u64 = undefined;
    for (&direct_times, 0..) |*elapsed, index| {
        const frame = if (index % 2 == 0) frame_ab else frame_ba;
        const damage = if (index % 2 == 0) &damage_ab else &damage_ba;
        const start = std.Io.Timestamp.now(testing.io, .real).nanoseconds;
        _ = try direct_surface.renderPassDamage(frame.renderPass(), clear, damage.slice());
        elapsed.* = @intCast(std.Io.Timestamp.now(testing.io, .real).nanoseconds - start);
    }

    const cached_pixels = try allocator.alloc(u8, pixel_len);
    defer allocator.free(cached_pixels);
    var memo = canvas.ReferenceRenderMemo.init(allocator);
    defer memo.deinit();
    const cached_surface = (try canvas.ReferenceRenderSurface.init(width, height, cached_pixels)).withFonts(&fonts).withRenderMemo(&memo);
    try cached_surface.renderPass(initial_frame.renderPass(), clear);
    // Warm the finite glyph alphabet and both alternating layouts before the
    // measured 1 Hz steady-state run. Cold-start cost is reported separately
    // by the full-frame test/build path; it must not distort a 15-minute gate.
    for (0..8) |index| {
        const frame = if (index % 2 == 0) frame_ab else frame_ba;
        const damage = if (index % 2 == 0) &damage_ab else &damage_ba;
        _ = try cached_surface.renderPassDamage(frame.renderPass(), clear, damage.slice());
    }
    const mask_hits_before = memo.glyph_mask_hits;
    const mask_misses_before = memo.glyph_mask_misses;
    const command_hits_before = memo.hits;
    const command_misses_before = memo.misses;
    var cached_times: [cached_sample_count]u64 = undefined;
    for (&cached_times, 0..) |*elapsed, index| {
        const frame = if (index % 2 == 0) frame_ab else frame_ba;
        const damage = if (index % 2 == 0) &damage_ab else &damage_ba;
        const start = std.Io.Timestamp.now(testing.io, .real).nanoseconds;
        _ = try cached_surface.renderPassDamage(frame.renderPass(), clear, damage.slice());
        elapsed.* = @intCast(std.Io.Timestamp.now(testing.io, .real).nanoseconds - start);
    }
    try testing.expectEqualSlices(u8, direct_pixels, cached_pixels);

    var direct_total: u64 = 0;
    var cached_total: u64 = 0;
    for (direct_times) |value| direct_total += value;
    for (cached_times) |value| cached_total += value;
    std.sort.pdq(u64, &direct_times, {}, renderBenchLessThan);
    std.sort.pdq(u64, &cached_times, {}, renderBenchLessThan);
    const direct_p95_index = (direct_sample_count * 95 + 99) / 100 - 1;
    const cached_p95_index = (cached_sample_count * 95 + 99) / 100 - 1;
    var dirty_area: f64 = 0;
    for (damage_ab.slice()) |rect| dirty_area += rect.width * rect.height;
    const profile = frame_ab.profile();
    const direct_avg_ms = @as(f64, @floatFromInt(direct_total)) / direct_sample_count / 1_000_000.0;
    const cached_avg_ms = @as(f64, @floatFromInt(cached_total)) / cached_sample_count / 1_000_000.0;
    std.debug.print(
        "SD300_RENDER_BENCH commands={d} build_pair_ms={d:.3} plan_three_ms={d:.3} dirty_rects={d} dirty_area={d:.0} dirty_ratio={d:.4} direct_avg_ms={d:.3} direct_p95_ms={d:.3} cached_avg_ms={d:.3} cached_p95_ms={d:.3} renderer_core_at_1hz={d:.3}% command_memo_hits={d} command_memo_misses={d} glyph_mask_hits={d} glyph_mask_misses={d}\n",
        .{
            list_b.commandCount(),
            @as(f64, @floatFromInt(build_ns)) / 1_000_000.0,
            @as(f64, @floatFromInt(plan_ns)) / 1_000_000.0,
            damage_ab.count,
            dirty_area,
            profile.dirty_ratio,
            direct_avg_ms,
            @as(f64, @floatFromInt(direct_times[direct_p95_index])) / 1_000_000.0,
            cached_avg_ms,
            @as(f64, @floatFromInt(cached_times[cached_p95_index])) / 1_000_000.0,
            cached_avg_ms / 10.0,
            memo.hits - command_hits_before,
            memo.misses - command_misses_before,
            memo.glyph_mask_hits - mask_hits_before,
            memo.glyph_mask_misses - mask_misses_before,
        },
    );
}

// ============================================================================
// Warmed-state scroll damage attribution benchmark
//
// The steady-state overview benchmark above proves the 1 Hz tick is cheap on a
// warmed surface. This benchmark reproduces the reported "goes laggy after ~60s
// once you scroll" behaviour and attributes where the cost actually lands. It
// drives Network (5), Processes (6), and Drivers (8) — each fully populated so
// the content overflows the viewport — in three shapes:
//
//   * tick    : one data-only frame transition (chart shift + scalar/table
//               deltas) at a fixed scroll offset, cold vs warmed histories.
//   * scroll  : the same tick at a non-zero scroll offset that overflows.
//   * burst   : N successive frames that only advance the scroll offset on a
//               fully warmed model, i.e. a wheel-scroll drag.
//
// For every frame it records render duration, the damage mode the patched
// reference renderer actually took (sparse `.damage` vs the conservative
// union/full `.fallback`), the damage-rect count, the frame dirty ratio, and
// the render-memo hit/miss deltas. Test-only; gated behind SD300_RENDER_BENCH.
// ============================================================================

const WarmSample = struct {
    ns: u64,
    fallback: bool,
    rects: usize,
    ratio: f32,
};

const WarmPool = struct {
    allocator: std.mem.Allocator,
    frame_options: canvas.CanvasFrameOptions,
    clear: canvas.Color,
    fonts: []const canvas.ReferenceFont,
    width: usize,
    height: usize,
    pixels: []u8,
    verify: []u8,
    arena0: *std.heap.ArenaAllocator,
    arena1: *std.heap.ArenaAllocator,
    cmds0: []canvas.CanvasCommand,
    cmds1: []canvas.CanvasCommand,
    nodes0: []canvas.WidgetLayoutNode,
    nodes1: []canvas.WidgetLayoutNode,
    builder0: *canvas.Builder,
    builder1: *canvas.Builder,
    diff: []canvas.DiffChange,

    fn build(pool: *WarmPool, slot: u1, model: *const main.Model) !canvas.DisplayList {
        const arena = if (slot == 0) pool.arena0 else pool.arena1;
        const cmds = if (slot == 0) pool.cmds0 else pool.cmds1;
        const nodes = if (slot == 0) pool.nodes0 else pool.nodes1;
        const builder = if (slot == 0) pool.builder0 else pool.builder1;
        _ = arena.reset(.retain_capacity);
        builder.* = canvas.Builder.init(cmds);
        return buildRenderBenchDisplayList(arena.allocator(), model, builder, nodes);
    }
};

inline fn warmClock() i128 {
    return @intCast(std.Io.Timestamp.now(testing.io, .real).nanoseconds);
}

fn warmSetName(buf: anytype, comptime fmt: []const u8, args: anytype) void {
    var tmp: [128]u8 = undefined;
    const text = std.fmt.bufPrint(&tmp, fmt, args) catch return;
    buf.set(text);
}

fn warmFill(history: []f64, tick: u32, seed: u32) void {
    for (history, 0..) |*sample, index| {
        const t = @as(f64, @floatFromInt(index)) + @as(f64, @floatFromInt(tick));
        const s = @as(f64, @floatFromInt(seed));
        const value = 50.0 + 40.0 * @sin(t * 0.30 + s) + 8.0 * @sin(t * 0.13 + s * 1.7);
        sample.* = std.math.clamp(value, 5.0, 95.0);
    }
}

fn warmHistories(model: *main.Model, tick: u32) void {
    warmFill(&model.cpu_history, tick, 1);
    warmFill(&model.memory_history, tick, 2);
    warmFill(&model.swap_history, tick, 3);
    warmFill(&model.network_download_history, tick, 4);
    warmFill(&model.network_upload_history, tick, 5);
    warmFill(&model.gpu_history, tick, 6);
    warmFill(&model.temperature_history_celsius, tick, 7);
    warmFill(&model.temperature_history_fahrenheit, tick, 8);
    warmFill(&model.disk_read_history, tick, 9);
    warmFill(&model.disk_write_history, tick, 10);
}

fn warmPopulate(model: *main.Model, section: u8) void {
    const d = &model.detail;
    switch (section) {
        5 => {
            d.static_ready = true;
            d.fast_ready = true;
            d.medium_ready = true;
            d.diagnostics_ready = true;
            d.interface_count = 4;
            d.interface_total_count = 6;
            for (0..4) |i| {
                const r = &d.interface_rows[i];
                r.id = @intCast(i);
                r.is_up = true;
                r.download_kib_s = 100 + @as(f64, @floatFromInt(i)) * 40;
                r.upload_kib_s = 20 + @as(f64, @floatFromInt(i)) * 8;
                r.received_gib = 12.5;
                r.transmitted_gib = 3.2;
                warmSetName(&r.name_buffer, "eth{d}", .{i});
                r.state_buffer.set("up");
                warmSetName(&r.address_buffer, "10.0.0.{d}", .{i + 2});
                r.mac_buffer.set("00:11:22:33:44:55");
            }
            d.connection_count = 20;
            d.connection_total_count = 44;
            d.listening_count = 7;
            for (0..20) |i| {
                const r = &d.connection_rows[i];
                r.id = @intCast(i);
                r.local_port = @intCast(1024 + i);
                r.remote_port = 443;
                r.pid = @intCast(2000 + i);
                r.protocol_buffer.set("tcp");
                r.local_buffer.set("10.0.0.5");
                warmSetName(&r.remote_buffer, "93.184.216.{d}", .{i + 1});
                r.state_buffer.set("ESTABLISHED");
                warmSetName(&r.process_buffer, "svc{d}", .{i});
            }
            d.gateway_reachable = true;
            d.gateway_latency_ms = 1.2;
            d.gateway_latency_available = true;
            d.dns_resolved = true;
            d.dns_latency_ms = 8.3;
            d.dns_latency_available = true;
            d.internet_reachable = true;
            d.internet_latency_ms = 15.0;
            d.internet_latency_available = true;
        },
        6 => {
            d.fast_ready = true;
            d.process_values_warmed = true;
            d.process_total_count = 486;
            d.process_total_threads = 3277;
            d.process_count = 16;
            model.show_all_processes = true;
            for (0..16) |i| {
                const r = &d.process_rows[i];
                r.id = @intCast(i);
                r.pid = @intCast(1000 + i * 7);
                warmSetName(&r.friendly_buffer, "Process {d}", .{i});
                warmSetName(&r.name_buffer, "proc{d}.exe", .{i});
                r.status_buffer.set("running");
            }
        },
        8 => {
            d.drivers_ready = true;
            d.driver_count = 32;
            d.driver_total_count = 71;
            d.driver_attention_count = 5;
            d.driver_scan_buffer.set("completed");
            for (0..32) |i| {
                const r = &d.driver_rows[i];
                r.id = @intCast(i);
                r.attention = (i % 7 == 0);
                warmSetName(&r.name_buffer, "Device Controller {d}", .{i});
                r.category_buffer.set("system");
                r.status_buffer.set("ok");
                warmSetName(&r.version_buffer, "10.0.{d}.3", .{i});
                r.date_buffer.set("2025-01-01");
            }
        },
        else => {},
    }
}

fn warmNudge(model: *main.Model, section: u8, tick: u32) void {
    const d = &model.detail;
    const ft = @as(f64, @floatFromInt(tick));
    switch (section) {
        5 => {
            d.total_download_kib_s = 800 + ft * 17.0;
            d.total_upload_kib_s = 120 + ft * 5.0;
            d.interface_rows[0].download_kib_s = 100 + ft * 3.0;
        },
        6 => {
            for (0..d.process_count) |i| {
                const fi = @as(f64, @floatFromInt(i));
                d.process_rows[i].cpu_percent = std.math.clamp(5 + 30 * @abs(@sin(fi * 0.5 + ft * 0.4)), 0, 400);
                d.process_rows[i].memory_mib = 50 + fi * 20 + ft * 2;
                d.process_rows[i].memory_percent = std.math.clamp(1 + fi * 0.3 + ft * 0.05, 0, 100);
            }
        },
        8 => {
            // Two far-apart rows change per tick: a small, well-separated
            // damage pattern that can exercise the sparse `.damage` path.
            const label: []const u8 = if (tick % 2 == 0) "ok" else "degraded";
            d.driver_rows[0].status_buffer.set(label);
            d.driver_rows[16].status_buffer.set(label);
        },
        else => {},
    }
}

fn warmConfigure(model: *main.Model, section: u8, tick: u32, warm: bool, scroll: f64) void {
    model.* = main.initialModel();
    model.engine_ready = true;
    model.fast_summary_seen = true;
    model.sequence = 100;
    model.cpu_percent = 42;
    model.memory_percent = 61;
    model.active_section = section;
    model.scroll_top = scroll;
    warmPopulate(model, section);
    warmNudge(model, section, tick);
    if (warm) warmHistories(model, tick);
}

fn warmApply(
    pool: *WarmPool,
    surface: canvas.ReferenceRenderSurface,
    prev: canvas.DisplayList,
    cur: canvas.DisplayList,
    scratch: *WarmRenderBenchFrameScratch,
) !WarmSample {
    const frame = try cur.framePlan(prev, pool.frame_options, scratch.storage());
    const damage = renderBenchDamage(try canvas.DisplayList.diff(prev, cur, pool.diff));
    const start = warmClock();
    const mode = try surface.renderPassDamage(frame.renderPass(), pool.clear, damage.slice());
    const ns: u64 = @intCast(warmClock() - start);
    return .{ .ns = ns, .fallback = mode == .fallback, .rects = damage.count, .ratio = frame.profile().dirty_ratio };
}

fn warmSummarize(
    name: []const u8,
    samples: []const WarmSample,
    dhits: u64,
    dmiss: u64,
    dmaskhit: u64,
    dmaskmiss: u64,
) void {
    var times: [64]u64 = undefined;
    var total: u64 = 0;
    var damage_n: usize = 0;
    var fallback_n: usize = 0;
    var rects_min: usize = std.math.maxInt(usize);
    var rects_max: usize = 0;
    var ratio_sum: f64 = 0;
    const n = samples.len;
    for (samples, 0..) |s, i| {
        times[i] = s.ns;
        total += s.ns;
        if (s.fallback) {
            fallback_n += 1;
        } else {
            damage_n += 1;
        }
        if (s.rects < rects_min) rects_min = s.rects;
        if (s.rects > rects_max) rects_max = s.rects;
        ratio_sum += s.ratio;
    }
    std.sort.pdq(u64, times[0..n], {}, renderBenchLessThan);
    const p50 = times[n / 2];
    const p95 = times[@min(n - 1, (n * 95 + 99) / 100 - 1)];
    const maxv = times[n - 1];
    const denom = @as(f64, @floatFromInt(n));
    std.debug.print(
        "WARM_BENCH {s} | n={d:>2} avg={d:>7.3} p50={d:>7.3} p95={d:>7.3} max={d:>7.3} ms | damage={d:>2} fallback={d:>2} | rects={d}-{d} ratio={d:.3} | memo hit+{d} miss+{d} maskhit+{d} maskmiss+{d}\n",
        .{
            name,
            n,
            @as(f64, @floatFromInt(total)) / denom / 1_000_000.0,
            @as(f64, @floatFromInt(p50)) / 1_000_000.0,
            @as(f64, @floatFromInt(p95)) / 1_000_000.0,
            @as(f64, @floatFromInt(maxv)) / 1_000_000.0,
            damage_n,
            fallback_n,
            rects_min,
            rects_max,
            ratio_sum / denom,
            dhits,
            dmiss,
            dmaskhit,
            dmaskmiss,
        },
    );
}

fn warmRunTick(
    pool: *WarmPool,
    name: []const u8,
    model_a: *const main.Model,
    model_b: *const main.Model,
    s0: *WarmRenderBenchFrameScratch,
    s1: *WarmRenderBenchFrameScratch,
    s2: *WarmRenderBenchFrameScratch,
) !void {
    var memo = canvas.ReferenceRenderMemo.init(pool.allocator);
    defer memo.deinit();
    const surface = (try canvas.ReferenceRenderSurface.init(pool.width, pool.height, pool.pixels)).withFonts(pool.fonts).withRenderMemo(&memo);

    const list_a = try pool.build(0, model_a);
    const list_b = try pool.build(1, model_b);

    // Seed the retained surface with frame A.
    const seed_frame = try list_a.framePlan(null, pool.frame_options, s0.storage());
    try surface.renderPass(seed_frame.renderPass(), pool.clear);

    // Correctness gate: applying the A->B damage to the retained surface must
    // reproduce a full render of B byte-for-byte. Guards against synthesizing
    // an incomplete damage set that would make the timings meaningless.
    const frame_ab = try list_b.framePlan(list_a, pool.frame_options, s1.storage());
    const damage_ab = renderBenchDamage(try canvas.DisplayList.diff(list_a, list_b, pool.diff));
    _ = try surface.renderPassDamage(frame_ab.renderPass(), pool.clear, damage_ab.slice());
    const full_b = try list_b.framePlan(null, pool.frame_options, s2.storage());
    const verify_surface = (try canvas.ReferenceRenderSurface.init(pool.width, pool.height, pool.verify)).withFonts(pool.fonts);
    try verify_surface.renderPass(full_b.renderPass(), pool.clear);
    try testing.expectEqualSlices(u8, pool.verify, pool.pixels);

    // Re-seed to A, then precompute both directions with distinct scratch so
    // the alternating steady-state loop can reuse them.
    try surface.renderPass(seed_frame.renderPass(), pool.clear);
    const frame_ba = try list_a.framePlan(list_b, pool.frame_options, s2.storage());
    const damage_ba = renderBenchDamage(try canvas.DisplayList.diff(list_b, list_a, pool.diff));

    // Warm the memo/caches (even count returns the surface to A).
    for (0..4) |i| {
        if (i % 2 == 0) {
            _ = try surface.renderPassDamage(frame_ab.renderPass(), pool.clear, damage_ab.slice());
        } else {
            _ = try surface.renderPassDamage(frame_ba.renderPass(), pool.clear, damage_ba.slice());
        }
    }

    const base_hits = memo.hits;
    const base_misses = memo.misses;
    const base_mask_hits = memo.glyph_mask_hits;
    const base_mask_misses = memo.glyph_mask_misses;

    const sample_count: usize = 24;
    var samples: [64]WarmSample = undefined;
    for (0..sample_count) |i| {
        const use_ab = i % 2 == 0;
        const frame = if (use_ab) frame_ab else frame_ba;
        const damage = if (use_ab) &damage_ab else &damage_ba;
        const start = warmClock();
        const mode = try surface.renderPassDamage(frame.renderPass(), pool.clear, damage.slice());
        const ns: u64 = @intCast(warmClock() - start);
        samples[i] = .{ .ns = ns, .fallback = mode == .fallback, .rects = damage.count, .ratio = frame.profile().dirty_ratio };
    }

    const dhits: u64 = @intCast(memo.hits - base_hits);
    const dmiss: u64 = @intCast(memo.misses - base_misses);
    const dmaskhit: u64 = @intCast(memo.glyph_mask_hits - base_mask_hits);
    const dmaskmiss: u64 = @intCast(memo.glyph_mask_misses - base_mask_misses);
    warmSummarize(name, samples[0..sample_count], dhits, dmiss, dmaskhit, dmaskmiss);
}

fn warmRunBurst(
    pool: *WarmPool,
    name: []const u8,
    base_model: *main.Model,
    scratch: *WarmRenderBenchFrameScratch,
) !void {
    var memo = canvas.ReferenceRenderMemo.init(pool.allocator);
    defer memo.deinit();
    const surface = (try canvas.ReferenceRenderSurface.init(pool.width, pool.height, pool.pixels)).withFonts(pool.fonts).withRenderMemo(&memo);

    const step: usize = 20;
    const preroll: usize = 4;
    const measured: usize = 16;

    base_model.scroll_top = 0;
    var prev = try pool.build(0, base_model);
    const seed_frame = try prev.framePlan(null, pool.frame_options, scratch.storage());
    try surface.renderPass(seed_frame.renderPass(), pool.clear);

    var slot: u1 = 1;
    var frame_index: usize = 1;

    for (0..preroll) |_| {
        base_model.scroll_top = @floatFromInt(frame_index * step);
        const cur = try pool.build(slot, base_model);
        _ = try warmApply(pool, surface, prev, cur, scratch);
        prev = cur;
        slot ^= 1;
        frame_index += 1;
    }

    const base_hits = memo.hits;
    const base_misses = memo.misses;
    const base_mask_hits = memo.glyph_mask_hits;
    const base_mask_misses = memo.glyph_mask_misses;

    var samples: [64]WarmSample = undefined;
    for (0..measured) |i| {
        base_model.scroll_top = @floatFromInt(frame_index * step);
        const cur = try pool.build(slot, base_model);
        samples[i] = try warmApply(pool, surface, prev, cur, scratch);
        prev = cur;
        slot ^= 1;
        frame_index += 1;
    }

    const dhits: u64 = @intCast(memo.hits - base_hits);
    const dmiss: u64 = @intCast(memo.misses - base_misses);
    const dmaskhit: u64 = @intCast(memo.glyph_mask_hits - base_mask_hits);
    const dmaskmiss: u64 = @intCast(memo.glyph_mask_misses - base_mask_misses);
    warmSummarize(name, samples[0..measured], dhits, dmiss, dmaskhit, dmaskmiss);
}

test "headless SD-300 warmed-state scroll damage attribution benchmark" {
    if (comptime !@import("builtin").link_libc) return error.SkipZigTest;
    if (std.c.getenv("SD300_RENDER_BENCH") == null) return error.SkipZigTest;

    const allocator = testing.allocator;
    const width: usize = 1180;
    const height: usize = 760;
    const pixel_len = width * height * 4;

    var makira_face = try canvas.font_ttf.Face.parse(@embedFile("fonts/Makira-Regular.ttf"));
    var plex_face = try canvas.font_ttf.Face.parse(@embedFile("fonts/IBMPlexMono-Regular.ttf"));
    var tokens_model = main.initialModel();
    const tokens = main.qubeTokens(&tokens_model);
    const fonts = [_]canvas.ReferenceFont{
        .{ .id = tokens.typography.font_id, .face = &makira_face },
        .{ .id = tokens.typography.mono_font_id, .face = &plex_face },
    };

    const pixels = try allocator.alloc(u8, pixel_len);
    defer allocator.free(pixels);
    const verify = try allocator.alloc(u8, pixel_len);
    defer allocator.free(verify);

    const arena0 = try allocator.create(std.heap.ArenaAllocator);
    defer allocator.destroy(arena0);
    arena0.* = std.heap.ArenaAllocator.init(allocator);
    defer arena0.deinit();
    const arena1 = try allocator.create(std.heap.ArenaAllocator);
    defer allocator.destroy(arena1);
    arena1.* = std.heap.ArenaAllocator.init(allocator);
    defer arena1.deinit();

    const cmds0 = try allocator.alloc(canvas.CanvasCommand, warm_bench_capacity);
    defer allocator.free(cmds0);
    const cmds1 = try allocator.alloc(canvas.CanvasCommand, warm_bench_capacity);
    defer allocator.free(cmds1);
    const nodes0 = try allocator.alloc(canvas.WidgetLayoutNode, warm_bench_capacity);
    defer allocator.free(nodes0);
    const nodes1 = try allocator.alloc(canvas.WidgetLayoutNode, warm_bench_capacity);
    defer allocator.free(nodes1);
    const builder0 = try allocator.create(canvas.Builder);
    defer allocator.destroy(builder0);
    const builder1 = try allocator.create(canvas.Builder);
    defer allocator.destroy(builder1);
    const diff = try allocator.alloc(canvas.DiffChange, warm_bench_capacity);
    defer allocator.free(diff);

    var pool = WarmPool{
        .allocator = allocator,
        .frame_options = .{ .surface_size = native_sdk.geometry.SizeF.init(width, height), .scale = 1 },
        .clear = tokens.colors.background,
        .fonts = &fonts,
        .width = width,
        .height = height,
        .pixels = pixels,
        .verify = verify,
        .arena0 = arena0,
        .arena1 = arena1,
        .cmds0 = cmds0,
        .cmds1 = cmds1,
        .nodes0 = nodes0,
        .nodes1 = nodes1,
        .builder0 = builder0,
        .builder1 = builder1,
        .diff = diff,
    };

    const s0 = try allocator.create(WarmRenderBenchFrameScratch);
    defer allocator.destroy(s0);
    const s1 = try allocator.create(WarmRenderBenchFrameScratch);
    defer allocator.destroy(s1);
    const s2 = try allocator.create(WarmRenderBenchFrameScratch);
    defer allocator.destroy(s2);

    const model_a = try allocator.create(main.Model);
    defer allocator.destroy(model_a);
    const model_b = try allocator.create(main.Model);
    defer allocator.destroy(model_b);

    std.debug.print("\nSD300_WARM_BENCH surface={d}x{d} sections=5,6,8 (Network/Processes/Drivers)\n", .{ width, height });

    const sections = [_]u8{ 5, 6, 8 };
    const scrolls = [_]f64{ 0, 420 };
    const warm_flags = [_]bool{ false, true };

    for (sections) |section| {
        for (scrolls) |scroll| {
            for (warm_flags) |warm| {
                warmConfigure(model_a, section, 10, warm, scroll);
                warmConfigure(model_b, section, 11, warm, scroll);
                const warm_label: []const u8 = if (warm) "warm" else "cold";
                var name_buf: [96]u8 = undefined;
                const name = try std.fmt.bufPrint(&name_buf, "sec{d} tick {s} scroll={d:>3.0}", .{ section, warm_label, scroll });
                try warmRunTick(&pool, name, model_a, model_b, s0, s1, s2);
            }
        }
        // Scroll burst on a fully warmed, populated model: only the scroll
        // offset advances per frame — a wheel-scroll drag.
        warmConfigure(model_a, section, 10, true, 0);
        var burst_name_buf: [96]u8 = undefined;
        const burst_name = try std.fmt.bufPrint(&burst_name_buf, "sec{d} scroll-burst  warm      ", .{section});
        try warmRunBurst(&pool, burst_name, model_a, s0);
    }
}
