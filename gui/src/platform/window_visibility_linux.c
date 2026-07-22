#if defined(__linux__)

#include <gtk/gtk.h>

extern void sd300_model_open(void);

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
