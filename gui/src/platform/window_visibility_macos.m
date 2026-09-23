#import <AppKit/AppKit.h>

extern void sd300_model_open(void);
extern void sd300_model_shutdown(void);

static id sd300_termination_observer = nil;

void sd300_install_termination_cleanup(void) {
    if (sd300_termination_observer) return;
    // AppKit terminate: exits without returning to Zig main. Its synchronous
    // notification is the last opportunity to join owned collector processes.
    sd300_termination_observer = [[NSNotificationCenter defaultCenter]
        addObserverForName:NSApplicationWillTerminateNotification
                    object:nil
                     queue:nil
                usingBlock:^(NSNotification *note) {
                    (void)note;
                    sd300_model_shutdown();
                }];
}

void sd300_uninstall_termination_cleanup(void) {
    if (!sd300_termination_observer) return;
    [[NSNotificationCenter defaultCenter] removeObserver:sd300_termination_observer];
    sd300_termination_observer = nil;
}

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
