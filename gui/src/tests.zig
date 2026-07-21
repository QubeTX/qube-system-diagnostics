const std = @import("std");
const native_sdk = @import("native_sdk");
const main = @import("main.zig");
const engine = @import("engine.zig");

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

    var arena_state = std.heap.ArenaAllocator.init(testing.allocator);
    defer arena_state.deinit();
    const tree = try buildTree(arena_state.allocator(), &model);
    _ = try expectByText(tree.root, .badge, "LIVE · SAMPLE 42");
    _ = try expectByText(tree.root, .text, "18.3%");
    _ = try expectByText(tree.root, .text, "16.0 GiB used of 32.0 GiB");
    _ = try expectByText(tree.root, .badge, "3 WARNINGS");
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
