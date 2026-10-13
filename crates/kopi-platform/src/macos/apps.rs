//! `NSWorkspace` and `NSRunningApplication` behind [`ForegroundApp`], and the CoreGraphics
//! idle timer behind [`UserIdle`].

use std::time::Duration;

use objc2_app_kit::{NSApplicationActivationPolicy, NSRunningApplication, NSWorkspace};
use objc2_foundation::NSString;

use super::on_main;
use crate::apps::{AppRef, ForegroundApp, UserIdle};

/// The frontmost app, from `NSWorkspace`. Calls run on the main thread, because
/// `frontmostApplication` is only kept up to date by the main run loop.
#[derive(Debug, Clone, Default)]
pub struct MacForegroundApp;

impl ForegroundApp for MacForegroundApp {
    fn current(&self) -> Option<AppRef> {
        on_main(|| {
            NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .map(|app| app_ref(&app))
        })
        .flatten()
    }

    fn running(&self, bundle_id: &str) -> Option<AppRef> {
        let bundle_id = bundle_id.to_owned();
        on_main(move || {
            NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(
                &bundle_id,
            ))
            .firstObject()
            .map(|app| app_ref(&app))
        })
        .flatten()
    }
}

/// The apps running now that have a Dock icon, for choosing apps to exclude. Kopi itself and
/// background agents aren't in it.
pub fn running_apps() -> Vec<AppRef> {
    on_main(|| {
        NSWorkspace::sharedWorkspace()
            .runningApplications()
            .iter()
            .filter(|app| app.activationPolicy() == NSApplicationActivationPolicy::Regular)
            .map(|app| app_ref(&app))
            .filter(|app| app.bundle_id.is_some())
            .collect()
    })
    .unwrap_or_default()
}

fn app_ref(app: &NSRunningApplication) -> AppRef {
    AppRef {
        bundle_id: app.bundleIdentifier().map(|s| s.to_string()),
        name: app.localizedName().map(|s| s.to_string()),
    }
}

/// Idle time from `CGEventSourceSecondsSinceLastEventType`. It needs no permission and can be
/// called from any thread.
#[derive(Debug, Clone, Default)]
pub struct MacUserIdle;

impl UserIdle for MacUserIdle {
    fn idle_time(&self) -> Option<Duration> {
        // SAFETY: a plain C function that takes two integers and returns a double.
        let seconds = unsafe {
            CGEventSourceSecondsSinceLastEventType(COMBINED_SESSION_STATE, ANY_INPUT_EVENT)
        };
        Duration::try_from_secs_f64(seconds).ok()
    }
}

/// `kCGEventSourceStateCombinedSessionState`
const COMBINED_SESSION_STATE: i32 = 0;
/// `kCGAnyInputEventType`
const ANY_INPUT_EVENT: u32 = !0;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
}
