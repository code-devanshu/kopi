//! Which app the user is in, and how long since they last touched the keyboard or mouse.

use std::time::Duration;

/// An app, as far as the OS could tell us.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppRef {
    /// A macOS bundle ID such as `com.apple.Safari`.
    pub bundle_id: Option<String>,
    /// The name to show, such as "Safari".
    pub name: Option<String>,
}

/// The app in front, used as the best guess for where a copy came from.
pub trait ForegroundApp: Send {
    /// The frontmost app, or `None` if there isn't one or the OS didn't answer.
    fn current(&self) -> Option<AppRef>;

    /// A running app with this bundle ID, for a copy whose writer named itself.
    fn running(&self, bundle_id: &str) -> Option<AppRef>;
}

/// How long the user has been away. The watcher polls less often when they are.
pub trait UserIdle: Send {
    /// Time since the last key press, click or mouse move in this login session.
    fn idle_time(&self) -> Option<Duration>;
}
