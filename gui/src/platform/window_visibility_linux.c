#if defined(__linux__)

#include <gtk/gtk.h>

extern void sd300_model_open(void);

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

static gboolean sd300_destroy_windows(gpointer unused) {
    (void)unused;
    GListModel *windows = gtk_window_get_toplevels();
    guint count = g_list_model_get_n_items(windows);
    for (guint index = 0; index < count; ++index) {
        GtkWindow *window = GTK_WINDOW(g_list_model_get_item(windows, index));
        gtk_window_destroy(window);
        g_object_unref(window);
    }
    return G_SOURCE_REMOVE;
}

void sd300_platform_open(void) {
    g_main_context_invoke(NULL, sd300_open_model, NULL);
}

void sd300_platform_quit(void) {
    g_main_context_invoke(NULL, sd300_destroy_windows, NULL);
}

#endif
