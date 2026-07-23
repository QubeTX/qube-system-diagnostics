const std = @import("std");
const builtin = @import("builtin");
const native_sdk = @import("native_sdk");

pub fn build(b: *std.Build) void {
    const native_sdk_dependency = b.dependency("native_sdk", .{});
    const icon_geometry_mod = b.createModule(.{
        .root_source_file = native_sdk_dependency.path("src/primitives/geometry/root.zig"),
        .target = b.graph.host,
        .optimize = .ReleaseSafe,
    });
    const app_icon_mod = b.createModule(.{
        .root_source_file = native_sdk_dependency.path("src/primitives/canvas/app_icon.zig"),
        .target = b.graph.host,
        .optimize = .ReleaseSafe,
    });
    app_icon_mod.addImport("geometry", icon_geometry_mod);
    const icon_export_mod = b.createModule(.{
        .root_source_file = b.path("tools/export_icons.zig"),
        .target = b.graph.host,
        .optimize = .ReleaseSafe,
    });
    icon_export_mod.addImport("app_icon", app_icon_mod);
    const icon_export_exe = b.addExecutable(.{
        .name = "sd300-export-icons",
        .root_module = icon_export_mod,
    });
    const generate_icons_run = b.addRunArtifact(icon_export_exe);
    generate_icons_run.setCwd(b.path("."));
    generate_icons_run.addArgs(&.{
        "generate",
        "assets/icon-source/app-icon.png",
        "assets/icon-source/tray-icon.svg",
        "assets/generated",
    });
    generate_icons_run.has_side_effects = true;
    const generate_icons_step = b.step("generate-icons", "Regenerate app and tray assets from the selected masters");
    generate_icons_step.dependOn(&generate_icons_run.step);

    const check_icons_run = b.addRunArtifact(icon_export_exe);
    check_icons_run.setCwd(b.path("."));
    check_icons_run.addArgs(&.{
        "check",
        "assets/icon-source/app-icon.png",
        "assets/icon-source/tray-icon.svg",
        "assets/generated",
    });
    const check_icons_step = b.step("check-icons", "Verify generated app and tray assets match the selected masters");
    check_icons_step.dependOn(&check_icons_run.step);

    // SD-300 release lanes build on the target OS (cross-architecture only).
    // Using the host here avoids redeclaring the SDK-owned `target` build
    // option and makes unsupported cross-OS builds fail closed later.
    const app_options: native_sdk.AppOptions = if (builtin.os.tag == .linux)
        .{
            .name = "sd300-gui",
            .main = "../../src/main.zig",
            .app_root = "platform/linux",
        }
    else
        .{ .name = "sd300-gui" };
    const app = native_sdk.addAppArtifacts(b, native_sdk_dependency, app_options);
    const os = app.exe.root_module.resolved_target.?.result.os.tag;
    // Do not set `app.exe.root_module.strip` here. Native SDK reuses the app
    // module while producing its strict analysis-only object; Zig 0.16.0 can
    // crash in that path when the shared module is stripped. The owned build
    // wrappers strip only the finished release executable after strict tests,
    // leaving the analysis graph and Windows PDB-producing install step intact.
    if (os == .linux) {
        // Zig's explicit Linux target mode does not infer the native multiarch
        // library directory even though Native SDK's pkg-config integration
        // resolves GTK's headers and -l names. The owned build wrapper passes
        // gtk4's target-native libdir so x86_64, ARM64, and musl stay portable.
        if (b.option([]const u8, "system-lib-dir", "Target-native GTK/system library directory")) |dir| {
            const library_path: std.Build.LazyPath = .{ .cwd_relative = dir };
            app.exe.root_module.addLibraryPath(library_path);
            if (app.tests.root_module != app.exe.root_module) {
                app.tests.root_module.addLibraryPath(library_path);
            }
        }
    } else if (os == .macos) {
        // Explicit architecture targets do not inherit every Xcode SDK search
        // path. In particular, newer SDKs expose libDER as a nested/private
        // framework even though Security.framework publicly includes it.
        if (b.option([]const u8, "system-include-dir", "Target macOS SDK system include directory")) |dir| {
            const include_path: std.Build.LazyPath = .{ .cwd_relative = dir };
            app.exe.root_module.addSystemIncludePath(include_path);
            if (app.tests.root_module != app.exe.root_module) {
                app.tests.root_module.addSystemIncludePath(include_path);
            }
        }
        if (b.option([]const u8, "system-framework-dir", "Target macOS SDK framework directory")) |dir| {
            const framework_path: std.Build.LazyPath = .{ .cwd_relative = dir };
            app.exe.root_module.addSystemFrameworkPath(framework_path);
            if (app.tests.root_module != app.exe.root_module) {
                app.tests.root_module.addSystemFrameworkPath(framework_path);
            }
        }
    }
    const engine_name = switch (os) {
        .windows => "sd300_engine.dll",
        .macos => "libsd300_engine.dylib",
        .linux => "libsd300_engine.so",
        else => @panic("SD-300 GUI engine is supported only on Windows, macOS, and Linux"),
    };
    const install_engine = b.addInstallBinFile(b.path(engine_name), engine_name);
    const install_icon = b.addInstallBinFile(b.path("assets/generated/app-icon-512.png"), "assets/app-icon.png");
    const install_app_ico = b.addInstallBinFile(b.path("assets/generated/app-icon.ico"), "assets/app-icon.ico");
    const install_tray_ico = b.addInstallBinFile(b.path("assets/generated/tray-icon.ico"), "assets/tray-icon.ico");
    const install_tray_dark_ico = b.addInstallBinFile(b.path("assets/generated/tray-icon-dark.ico"), "assets/tray-icon-dark.ico");
    const install_tray_light_ico = b.addInstallBinFile(b.path("assets/generated/tray-icon-light.ico"), "assets/tray-icon-light.ico");
    const install_tray_template = b.addInstallBinFile(b.path("assets/generated/tray-icon-template.png"), "assets/tray-icon-template.png");
    const install_app_icns = b.addInstallBinFile(b.path("assets/generated/app-icon.icns"), "assets/app-icon.icns");
    if (os == .windows) {
        app.exe.root_module.addWin32ResourceFile(.{
            .file = b.path("platform/windows/app-icon.rc"),
            .include_paths = &.{b.path("assets/generated")},
        });
    }
    if (builtin.os.tag == .macos) {
        const visibility_source = b.path("src/platform/window_visibility_macos.m");
        const lifecycle_source = b.path("src/platform/lifecycle_unix.c");
        app.exe.root_module.addCSourceFile(.{ .file = visibility_source, .flags = &.{ "-fobjc-arc", "-fblocks" } });
        app.exe.root_module.addCSourceFile(.{ .file = lifecycle_source, .flags = &.{} });
        if (app.tests.root_module != app.exe.root_module) {
            app.tests.root_module.addCSourceFile(.{ .file = visibility_source, .flags = &.{ "-fobjc-arc", "-fblocks" } });
            app.tests.root_module.addCSourceFile(.{ .file = lifecycle_source, .flags = &.{} });
        }
    } else if (builtin.os.tag == .linux) {
        const visibility_source = b.path("src/platform/window_visibility_linux.c");
        const lifecycle_source = b.path("src/platform/lifecycle_unix.c");
        app.exe.root_module.addCSourceFile(.{ .file = visibility_source, .flags = &.{} });
        app.exe.root_module.addCSourceFile(.{ .file = lifecycle_source, .flags = &.{} });
        if (app.tests.root_module != app.exe.root_module) {
            app.tests.root_module.addCSourceFile(.{ .file = visibility_source, .flags = &.{} });
            app.tests.root_module.addCSourceFile(.{ .file = lifecycle_source, .flags = &.{} });
        }
    }
    app.install.step.dependOn(&install_engine.step);
    app.install.step.dependOn(&install_icon.step);
    app.install.step.dependOn(&install_app_ico.step);
    app.install.step.dependOn(&install_tray_ico.step);
    app.install.step.dependOn(&install_tray_dark_ico.step);
    app.install.step.dependOn(&install_tray_light_ico.step);
    app.install.step.dependOn(&install_tray_template.step);
    app.install.step.dependOn(&install_app_icns.step);
    app.install.step.dependOn(&check_icons_run.step);
    app.run.step.dependOn(&install_engine.step);
    app.run.step.dependOn(&install_icon.step);
    app.run.step.dependOn(&install_app_ico.step);
    app.run.step.dependOn(&install_tray_ico.step);
    app.run.step.dependOn(&install_tray_dark_ico.step);
    app.run.step.dependOn(&install_tray_light_ico.step);
    app.run.step.dependOn(&install_tray_template.step);
    app.run.step.dependOn(&install_app_icns.step);
    app.run.step.dependOn(&check_icons_run.step);
    app.tests.step.dependOn(&check_icons_run.step);
}
