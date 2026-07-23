const std = @import("std");
const app_icon = @import("app_icon");

const Mode = enum {
    generate,
    check,
};

const AssetWriter = struct {
    allocator: std.mem.Allocator,
    io: std.Io,
    mode: Mode,

    fn emit(self: AssetWriter, path: []const u8, bytes: []const u8) !void {
        const cwd = std.Io.Dir.cwd();
        switch (self.mode) {
            .generate => {
                if (std.fs.path.dirname(path)) |parent| {
                    try cwd.createDirPath(self.io, parent);
                }
                try cwd.writeFile(self.io, .{ .sub_path = path, .data = bytes });
                std.debug.print("generated {s}\n", .{path});
            },
            .check => {
                const existing = cwd.readFileAlloc(
                    self.io,
                    path,
                    self.allocator,
                    .limited(bytes.len + 1),
                ) catch {
                    std.debug.print("missing generated icon asset: {s}\n", .{path});
                    return error.GeneratedIconDrift;
                };
                defer self.allocator.free(existing);
                if (!std.mem.eql(u8, existing, bytes)) {
                    std.debug.print("generated icon asset is stale: {s}\n", .{path});
                    return error.GeneratedIconDrift;
                }
            },
        }
    }
};

pub fn main(init: std.process.Init) !void {
    var args = try init.minimal.args.iterateAllocator(init.gpa);
    defer args.deinit();
    _ = args.next();

    const mode_text = args.next() orelse return usage();
    const app_source_path = args.next() orelse return usage();
    const tray_source_path = args.next() orelse return usage();
    const output_root = args.next() orelse return usage();
    if (args.next() != null) return usage();

    const mode: Mode = if (std.mem.eql(u8, mode_text, "generate"))
        .generate
    else if (std.mem.eql(u8, mode_text, "check"))
        .check
    else
        return usage();

    const allocator = init.gpa;
    const io = init.io;
    const cwd = std.Io.Dir.cwd();
    const writer: AssetWriter = .{ .allocator = allocator, .io = io, .mode = mode };

    const app_source_bytes = try cwd.readFileAlloc(io, app_source_path, allocator, .limited(64 * 1024 * 1024));
    defer allocator.free(app_source_bytes);
    var app_source = try loadSource(allocator, app_source_bytes, .png, app_source_path);
    defer app_source.deinit(allocator);

    try emitAppAssets(allocator, writer, output_root, &app_source);

    const tray_source_bytes = try cwd.readFileAlloc(io, tray_source_path, allocator, .limited(4 * 1024 * 1024));
    defer allocator.free(tray_source_bytes);
    var tray_source = try loadSource(allocator, tray_source_bytes, .svg, tray_source_path);
    defer tray_source.deinit(allocator);

    try emitTrayAssets(allocator, writer, output_root, &tray_source);

    if (mode == .check) {
        std.debug.print("generated icon assets match the selected masters\n", .{});
    }
}

fn usage() error{InvalidArguments} {
    std.debug.print(
        "usage: export-icons <generate|check> <app-source.png> <tray-source.svg> <output-root>\n",
        .{},
    );
    return error.InvalidArguments;
}

fn loadSource(
    allocator: std.mem.Allocator,
    bytes: []const u8,
    kind: app_icon.SourceKind,
    path: []const u8,
) !app_icon.Source {
    return switch (try app_icon.loadSource(allocator, bytes, kind)) {
        .ok => |source| source,
        .issue => {
            std.debug.print("selected icon source is unsupported or not square: {s}\n", .{path});
            return error.InvalidIconSource;
        },
    };
}

fn emitAppAssets(
    allocator: std.mem.Allocator,
    writer: AssetWriter,
    output_root: []const u8,
    source: *const app_icon.Source,
) !void {
    const ico = try app_icon.buildIco(allocator, source);
    defer allocator.free(ico);
    try emitAt(allocator, writer, output_root, "app-icon.ico", ico);

    const icns = try app_icon.buildIcns(allocator, source);
    defer allocator.free(icns);
    try emitAt(allocator, writer, output_root, "app-icon.icns", icns);

    const runtime_png = try app_icon.buildSquarePng(allocator, source, 512);
    defer allocator.free(runtime_png);
    try emitAt(allocator, writer, output_root, "app-icon-512.png", runtime_png);
    try writer.emit("assets/app-icon.png", runtime_png);
    try writer.emit("assets/icon.png", runtime_png);

    inline for (app_icon.linux_sizes) |size| {
        const png = try app_icon.buildSquarePng(allocator, source, size);
        defer allocator.free(png);
        const relative = try std.fmt.allocPrint(
            allocator,
            "linux/hicolor/{d}x{d}/apps/sd300.png",
            .{ size, size },
        );
        defer allocator.free(relative);
        try emitAt(allocator, writer, output_root, relative, png);
    }
}

fn emitTrayAssets(
    allocator: std.mem.Allocator,
    writer: AssetWriter,
    output_root: []const u8,
    dark_source: *const app_icon.Source,
) !void {
    const dark_ico = try app_icon.buildIco(allocator, dark_source);
    defer allocator.free(dark_ico);
    try emitAt(allocator, writer, output_root, "tray-icon.ico", dark_ico);
    try emitAt(allocator, writer, output_root, "tray-icon-dark.ico", dark_ico);

    const template_png = try app_icon.buildSquarePng(allocator, dark_source, 36);
    defer allocator.free(template_png);
    try emitAt(allocator, writer, output_root, "tray-icon-template.png", template_png);
    try writer.emit("assets/tray-icon-template.png", template_png);

    const dark_preview_png = try app_icon.buildSquarePng(allocator, dark_source, 512);
    defer allocator.free(dark_preview_png);
    try emitAt(allocator, writer, output_root, "tray-icon-512.png", dark_preview_png);

    const tray_master_png = try app_icon.buildSquarePng(allocator, dark_source, 1024);
    defer allocator.free(tray_master_png);
    var light_source = try loadSource(allocator, tray_master_png, .png, "generated tray master");
    defer light_source.deinit(allocator);
    const image = light_source.image orelse return error.InvalidIconSource;
    var offset: usize = 0;
    while (offset < image.pixels.len) : (offset += 4) {
        image.pixels[offset] = 255;
        image.pixels[offset + 1] = 255;
        image.pixels[offset + 2] = 255;
    }

    const light_ico = try app_icon.buildIco(allocator, &light_source);
    defer allocator.free(light_ico);
    try emitAt(allocator, writer, output_root, "tray-icon-light.ico", light_ico);

    const light_preview_png = try app_icon.buildSquarePng(allocator, &light_source, 512);
    defer allocator.free(light_preview_png);
    try emitAt(allocator, writer, output_root, "tray-icon-light-512.png", light_preview_png);
}

fn emitAt(
    allocator: std.mem.Allocator,
    writer: AssetWriter,
    output_root: []const u8,
    relative_path: []const u8,
    bytes: []const u8,
) !void {
    const path = try std.fs.path.join(allocator, &.{ output_root, relative_path });
    defer allocator.free(path);
    try writer.emit(path, bytes);
}
