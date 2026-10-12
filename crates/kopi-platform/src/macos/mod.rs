//! macOS: NSPasteboard, NSWorkspace, CGEvent and the Accessibility API.

mod apps;
mod pasteboard;

use std::sync::mpsc;
use std::time::Duration;

use dispatch2::DispatchQueue;
use objc2::MainThreadMarker;

pub use apps::{MacForegroundApp, MacUserIdle};
pub use pasteboard::MacPasteboard;

/// How long a background thread waits for the main thread to run a call. Pasteboard calls
/// take milliseconds, so hitting this means the main thread is stuck or not running its loop.
const MAIN_THREAD_TIMEOUT: Duration = Duration::from_secs(5);

/// Runs `f` on the main thread and returns its result, or `None` if the main thread didn't
/// get to it in time.
///
/// AppKit's pasteboard and workspace calls have to run there. On macOS 27.0.1,
/// `accessBehavior` called from another thread aborts the app. From the main thread, `f` runs
/// straight away. From any other thread it's queued on the main queue, so the main run loop
/// must be running, as it is in the Tauri app. The wait has a limit, so a thread that the main
/// thread is joining can't deadlock with it.
pub(crate) fn on_main<R, F>(f: F) -> Option<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    if MainThreadMarker::new().is_some() {
        return Some(f());
    }
    let (tx, rx) = mpsc::sync_channel(1);
    DispatchQueue::main().exec_async(move || {
        // The caller may have stopped waiting. Then the result is just dropped.
        let _ = tx.send(f());
    });
    rx.recv_timeout(MAIN_THREAD_TIMEOUT).ok()
}
