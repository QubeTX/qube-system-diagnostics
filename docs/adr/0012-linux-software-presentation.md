# ADR 0012 — Linux software presentation and ordered window cleanup

Date: 2026-09-23
Status: Candidate; native qualification required
Related: ADR 0011, task #v4a

## Evidence

Run 35874747086 records the normal GNU GUI's foreground RSS at 247.0 MiB on
x86-64 and 235.8 MiB on ARM64. A separate unchanged-byte Cairo smoke reduces
these to 159.5 and 150.0 MiB. Post-window mappings identify graphics libraries
and anonymous allocations as the main difference. These short diagnostics do
not qualify CPU or memory acceptance. The shared panel change is measured
separately on Windows and also applies to Linux.

SD-300's reviewed SDK already rasterizes the view into CPU pixels and presents
it through GtkDrawingArea/Cairo. A second GL/Vulkan rendering pipeline provides
no application 3D feature. Use Cairo as the process-local default, before GTK
initialization or worker startup, while preserving any explicit GSK_RENDERER
override. Verify the actual GtkNative renderer in a native fixture and retain
an explicit GL smoke in both GNU CI lanes.

GTK documents [Cairo selection](https://docs.gtk.org/gtk4/running.html#GSK_RENDERER),
but warns that environment controls are not stable end-user APIs. This is an
internal default against the distribution-qualified GTK closure, not a new
settings contract. Revalidate it whenever GTK dependencies change. GtkNative
exposes a renderer getter but no supported window renderer setter; inventing
one or reaching into private window storage would be less maintainable.

## Shutdown correction

The musl GUI intermittently exits with SIGSEGV after the owned quit socket
request. An unchanged repeat may pass, and attaching GDB changes its timing.
Source inspection identifies a definite lifetime defect: the quit route calls
`gtk_window_destroy`, bypassing the SDK's close-request handler. Later SDK
teardown still dereferences borrowed drawing-area pointers.

Route the request through [gtk_window_close](https://docs.gtk.org/gtk4/method.Window.close.html),
which follows the normal window-manager close path. That handler clears the
SDK views before GTK releases the window. Walk the toplevel list backwards
because accepted closes remove entries. The native regression fixture asserts
that an owned quit actually invokes close-request; direct destruction fails it.
Retain repeated foreground/hidden shutdown tests and separate debugger-only
reports. The source defect is established; claiming it explains every observed
SIGSEGV requires the native acceptance result.

## Acceptance

Native GNU x86-64, GNU ARM64 and musl must pass renderer selection, preserved
override, visibility recovery, normal close and owned-quit checks. Complete
resource windows, package closure validation and installer lifecycle remain
required. Windows and macOS backend selection is unchanged.
