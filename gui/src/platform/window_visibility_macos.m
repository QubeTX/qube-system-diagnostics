#import <AppKit/AppKit.h>

extern void sd300_model_open(void);

int sd300_main_window_visible(void) {
    BOOL found = NO;
    for (NSWindow *window in NSApp.windows) {
        if (!window.canBecomeMainWindow) continue;
        found = YES;
        if (window.isVisible && !window.isMiniaturized) return 1;
    }
    return found ? 0 : 1;
}

int sd300_main_window_policy_hidden(void) {
    for (NSWindow *window in NSApp.windows) {
        if (!window.canBecomeMainWindow) continue;
        return !window.isVisible && !window.isMiniaturized ? 1 : 0;
    }
    return 0;
}

void sd300_main_window_show(void) {
    [NSApp activateIgnoringOtherApps:YES];
    for (NSWindow *window in NSApp.windows) {
        if (!window.canBecomeMainWindow) continue;
        if (window.isMiniaturized) [window deminiaturize:nil];
        [window makeKeyAndOrderFront:nil];
        return;
    }
}

void sd300_main_window_hide(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        for (NSWindow *window in NSApp.windows) {
            if (!window.canBecomeMainWindow) continue;
            [window orderOut:nil];
            return;
        }
    });
}

void sd300_platform_open(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        sd300_model_open();
    });
}

void sd300_platform_quit(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [NSApp terminate:nil];
    });
}
