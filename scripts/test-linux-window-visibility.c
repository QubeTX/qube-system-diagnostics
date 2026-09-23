// Native GTK mapping/recovery fixture; run under a private Xvfb session.
#include <gtk/gtk.h>
#include <stdio.h>
extern int sd300_main_window_visible(void);
void sd300_model_open(void) {}
static int changes = 0;
void sd300_model_visibility_changed(void) { ++changes; }

static int expect(int wanted) {
    gint64 deadline = g_get_monotonic_time() + 2000000;
    do {
        while (g_main_context_iteration(NULL, FALSE)) {}
        if (sd300_main_window_visible() == wanted) return 1;
        g_usleep(1000);
    } while (g_get_monotonic_time() < deadline);
    fprintf(stderr, "GTK visibility expected %d, received %d\n",
            wanted, sd300_main_window_visible());
    return 0;
}

int main(void) {
    gtk_init();
    if (!expect(1)) return 1; // No surface yet: keep foreground sampling.
    GtkWidget *window = gtk_window_new();
    gtk_window_set_default_size(GTK_WINDOW(window), 320, 200);
    gtk_window_present(GTK_WINDOW(window));
    if (!expect(1)) return 2;
    changes = 0;
    gtk_widget_set_visible(window, FALSE);
    if (!expect(0)) return 3;
    if (changes == 0) return 6;
    changes = 0;
    gtk_window_present(GTK_WINDOW(window));
    if (!expect(1)) return 4;
    if (changes == 0) return 7;
    gtk_window_destroy(GTK_WINDOW(window));
    if (!expect(1)) return 5;
    puts("GTK visible, hidden, recovery and startup states pass");
    return 0;
}
