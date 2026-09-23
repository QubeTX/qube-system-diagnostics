// Compile the actual prepared host so this detects regressions in publication,
// not a parallel copy of the accessibility setter. No SDK cache is modified.
#import "../gui/node_modules/@native-sdk/cli/src/platform/macos/appkit_host.m"
#include <assert.h>

@interface FocusRecordingSurface : NativeSdkMetalSurfaceView
@property(nonatomic, assign) NSUInteger focusActions;
@property(nonatomic, assign) uint64_t lastFocusId;
@end

@implementation FocusRecordingSurface
- (BOOL)emitWidgetAccessibilityActionWithId:(uint64_t)widgetId action:(NSInteger)action {
    assert(action == NATIVE_SDK_APPKIT_WIDGET_ACCESSIBILITY_ACTION_FOCUS);
    self.focusActions += 1;
    self.lastFocusId = widgetId;
    return YES;
}
@end

int main(void) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        FocusRecordingSurface *surface = [[FocusRecordingSurface alloc] initWithFrame:NSZeroRect];
        [surface stopDisplayTimer];
        native_sdk_appkit_widget_accessibility_node_t node = {
            .id = 42,
            .role = NATIVE_SDK_APPKIT_WIDGET_ROLE_BUTTON,
            .label = "Focus fixture",
            .label_len = 13,
            .state_flags = NATIVE_SDK_APPKIT_WIDGET_STATE_ENABLED | NATIVE_SDK_APPKIT_WIDGET_STATE_FOCUSED,
            .action_flags = NATIVE_SDK_APPKIT_WIDGET_ACTION_FOCUS,
        };
        for (NSUInteger index = 0; index < 20; index++) {
            [surface updateWidgetAccessibilityWithNodes:&node count:1];
            assert(surface.widgetAccessibilityElements.count == 1);
            assert([surface.widgetAccessibilityElements[0] isAccessibilityFocused]);
            assert(surface.focusActions == 0);
        }
        NativeSdkWidgetAccessibilityElement *element = (id)surface.widgetAccessibilityElements[0];
        // A real assistive-client request must still reach the runtime once.
        [element setAccessibilityFocused:YES];
        assert(surface.focusActions == 1 && surface.lastFocusId == 42);
        [element setAccessibilityFocused:NO];
        assert(surface.focusActions == 1 && !element.accessibilityFocused);
        element.accessibilityEnabled = NO;
        [element setAccessibilityFocused:YES];
        assert(surface.focusActions == 1);
        element.accessibilityEnabled = YES;
        element.actionFlags = 0;
        [element setAccessibilityFocused:YES];
        assert(surface.focusActions == 1);
        node.state_flags = NATIVE_SDK_APPKIT_WIDGET_STATE_ENABLED;
        [surface updateWidgetAccessibilityWithNodes:&node count:1];
        assert(![surface.widgetAccessibilityElements[0] isAccessibilityFocused]);
        assert(surface.focusActions == 1);
        [surface updateWidgetAccessibilityWithNodes:NULL count:0];
        assert(surface.widgetAccessibilityElements.count == 0);
        puts("PASS: semantic publication is passive; assistive focus remains actionable");
    }
    return 0;
}
