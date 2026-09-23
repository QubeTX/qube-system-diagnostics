#if defined(__linux__)

#include <gtk/gtk.h>

extern void sd300_model_open(void);
extern void sd300_model_visibility_changed(void);

void sd300_configure_renderer(void) {
    // SD-300 submits a CPU-rasterized Cairo image. Avoid a second GPU pipeline
    // (and software-GL driver allocations on machines without an active GPU).
    // Keep explicit overrides intact. This is private process setup against
    // the distribution-qualified GTK closure, before any GTK or worker init.
    g_setenv("GSK_RENDERER", "cairo", FALSE);
}

static void sd300_visibility_changed(GObject *object, GParamSpec *property, gpointer unused) {
    (void)object;
    (void)property;
    (void)unused;
    sd300_model_visibility_changed();
}

static void observe_surface(GdkSurface *surface) {
    if (g_object_get_data(G_OBJECT(surface), "sd300-visibility-observed")) return;
    g_object_set_data(G_OBJECT(surface), "sd300-visibility-observed", GINT_TO_POINTER(1));
    g_signal_connect(surface, "notify::mapped", G_CALLBACK(sd300_visibility_changed), NULL);
    if (GDK_IS_TOPLEVEL(surface))
        g_signal_connect(surface, "notify::state", G_CALLBACK(sd300_visibility_changed), NULL);
}

// Called from the model's UI-thread refresh. GTK owns every borrowed surface;
// release only the toplevel references returned by GListModel. An unrealized
// startup window remains foreground until its state can be observed.
int sd300_main_window_visible(void) {
    GListModel *windows = gtk_window_get_toplevels();
    guint count = g_list_model_get_n_items(windows);
    int observed = 0;
    for (guint index = 0; index < count; ++index) {
        GtkWindow *window = GTK_WINDOW(g_list_model_get_item(windows, index));
        GdkSurface *surface = gtk_native_get_surface(GTK_NATIVE(window));
        if (surface != NULL) {
            observe_surface(surface);
            observed = 1;
            gboolean visible = gtk_widget_get_visible(GTK_WIDGET(window)) &&
                               gdk_surface_get_mapped(surface);
            if (visible && GDK_IS_TOPLEVEL(surface)) {
                visible = !(gdk_toplevel_get_state(GDK_TOPLEVEL(surface)) &
                            GDK_TOPLEVEL_STATE_MINIMIZED);
            }
            g_object_unref(window);
            if (visible) return 1;
        } else {
            g_object_unref(window);
        }
    }
    return !observed;
}

static gboolean sd300_open_model(gpointer unused) {
    (void)unused;
    sd300_model_open();
    return G_SOURCE_REMOVE;
}

static gboolean sd300_close_windows(gpointer unused) {
    (void)unused;
    GListModel *windows = gtk_window_get_toplevels();
    guint count = g_list_model_get_n_items(windows);
    // Follow close-request so the SDK releases its borrowed drawing-area
    // pointers before GTK destroys the widget tree. Destroying the window
    // directly bypasses that cleanup and leaves teardown with stale pointers.
    // Walk backwards because an accepted close removes a toplevel immediately.
    while (count > 0) {
        GtkWindow *window = GTK_WINDOW(g_list_model_get_item(windows, --count));
        gtk_window_close(window);
        g_object_unref(window);
    }
    return G_SOURCE_REMOVE;
}

void sd300_platform_open(void) {
    g_main_context_invoke(NULL, sd300_open_model, NULL);
}

void sd300_platform_quit(void) {
    g_main_context_invoke(NULL, sd300_close_windows, NULL);
}

#endif
