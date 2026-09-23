// Native notification contract; full GUI smoke verifies actual AppKit quit.
#import <AppKit/AppKit.h>
#include <stdio.h>
extern void sd300_install_termination_cleanup(void);
extern void sd300_uninstall_termination_cleanup(void);
static int stopped = 0;
void sd300_model_open(void) {}
void sd300_model_shutdown(void) { ++stopped; }

int main(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        sd300_install_termination_cleanup();
        sd300_install_termination_cleanup();
        [[NSNotificationCenter defaultCenter]
            postNotificationName:NSApplicationWillTerminateNotification object:NSApp];
        if (stopped != 1) {
            fprintf(stderr, "AppKit cleanup was not synchronous or was installed twice: %d\n", stopped);
            return 1;
        }
        sd300_uninstall_termination_cleanup();
        sd300_uninstall_termination_cleanup();
        [[NSNotificationCenter defaultCenter]
            postNotificationName:NSApplicationWillTerminateNotification object:NSApp];
        if (stopped != 1) return 2;
        sd300_install_termination_cleanup();
        [[NSNotificationCenter defaultCenter]
            postNotificationName:NSApplicationWillTerminateNotification object:NSApp];
        if (stopped != 2) return 3;
        sd300_uninstall_termination_cleanup();
        puts("AppKit cleanup is synchronous, idempotently installed and removable");
    }
    return 0;
}
