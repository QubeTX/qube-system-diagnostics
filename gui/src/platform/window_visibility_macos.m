#import <AppKit/AppKit.h>

int sd300_main_window_visible(void) {
    BOOL found = NO;
    for (NSWindow *window in NSApp.windows) {
        if (!window.canBecomeMainWindow) continue;
        found = YES;
        if (window.isVisible && !window.isMiniaturized) return 1;
    }
    return found ? 0 : 1;
}

void sd300_platform_open(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [NSApp activateIgnoringOtherApps:YES];
        for (NSWindow *window in NSApp.windows) {
            if (!window.canBecomeMainWindow) continue;
            [window makeKeyAndOrderFront:nil];
        }
    });
}

void sd300_platform_quit(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        [NSApp terminate:nil];
    });
}
