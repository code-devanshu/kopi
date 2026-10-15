//! The panel the hotkey opens: history, search and preview.
//!
//! On macOS it's a non-activating `NSPanel` (tauri-nspanel). It's built once at launch and then
//! only shown and hidden. The app the user was in stays frontmost, so a paste lands there. See
//! `docs/spikes/0002-macos-nspanel.md`.
//!
//! On Windows it's a borderless window that takes focus while it shows, and gives it back to the
//! window the user was in when it hides.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_specta::Event;

use crate::events::{PanelHidden, PanelShown};

/// The panel window's label.
pub const PANEL: &str = "panel";

const WIDTH: f64 = 760.0;
const HEIGHT: f64 = 480.0;

/// How long after the panel loses focus to decide whether to hide it. The emoji picker takes
/// focus when it opens, so the panel shouldn't hide before seeing where focus went.
const BLUR_GRACE: Duration = Duration::from_millis(100);

/// How long [`hide_now`] waits for the main thread.
const HIDE_TIMEOUT: Duration = Duration::from_secs(1);

/// Counts shows, so a hide planned on blur can tell the panel was shown again since.
#[derive(Default)]
struct Shows(AtomicU64);

/// Builds the panel, hidden.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    app.manage(Shows::default());
    platform::create(app)
}

/// Shows the panel on the screen with the pointer and focuses it. Safe from any thread.
pub fn show(app: &AppHandle) {
    on_main(app, |app| {
        app.state::<Shows>().0.fetch_add(1, Ordering::SeqCst);
        platform::show(app);
        let _ = PanelShown.emit(app);
    });
}

/// Hides the panel, if it's showing. Safe from any thread.
pub fn hide(app: &AppHandle) {
    on_main(app, |app| {
        if platform::hide(app) {
            let _ = PanelHidden.emit(app);
        }
    });
}

/// Hides the panel and returns once it's hidden, so keys posted next go to the app in front.
/// Gives up after [`HIDE_TIMEOUT`] if the main thread is stuck.
pub fn hide_now(app: &AppHandle) {
    if platform::is_main_thread() {
        hide(app);
        return;
    }
    let (tx, rx) = mpsc::sync_channel(1);
    let handle = app.clone();
    let queued = app.run_on_main_thread(move || {
        hide(&handle);
        let _ = tx.send(());
    });
    if queued.is_ok() {
        let _ = rx.recv_timeout(HIDE_TIMEOUT);
    }
}

/// What the hotkey does: shows the panel, or hides it if it's showing.
pub fn toggle(app: &AppHandle) {
    on_main(app, |app| {
        if platform::is_visible(app) {
            hide(app);
        } else {
            show(app);
        }
    });
}

/// Hides the panel shortly after it loses focus, unless another window of Kopi's process has
/// focus by then (an input method's, say) or the panel was shown again. Settings and onboarding
/// don't count.
pub fn focus_changed(app: &AppHandle, focused: bool) {
    if focused {
        return;
    }
    let shows = app.state::<Shows>().0.load(Ordering::SeqCst);
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(BLUR_GRACE);
        on_main(&app, move |app| {
            let shown_again = app.state::<Shows>().0.load(Ordering::SeqCst) != shows;
            let kopi_focused = platform::has_key_window(app)
                && !crate::settings_window::is_focused(app)
                && !crate::onboarding::is_focused(app);
            if !shown_again && !kopi_focused {
                hide(app);
            }
        });
    });
}

/// Runs `f` on the main thread: now if this is it, otherwise queued there.
fn on_main(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
    if platform::is_main_thread() {
        f(app);
    } else {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || f(&handle));
    }
}

#[cfg(target_os = "macos")]
pub(crate) use platform::place;

#[cfg(target_os = "macos")]
mod platform {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSEvent, NSScreen, NSWindow};
    use objc2_foundation::NSPoint;
    use tauri::{AppHandle, LogicalSize, Size, WebviewUrl};
    use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask};

    use super::{HEIGHT, PANEL, WIDTH};

    // The macro brings its own imports, so it gets a module of its own.
    mod class {
        use tauri_nspanel::tauri_panel;

        tauri_panel! {
            panel!(KopiPanel {
                config: {
                    can_become_key_window: true,
                    can_become_main_window: false,
                    is_floating_panel: true
                }
            })
        }
    }

    pub fn create(app: &AppHandle) -> tauri::Result<()> {
        PanelBuilder::<_, class::KopiPanel>::new(app, PANEL)
            .url(WebviewUrl::App("index.html".into()))
            .title("Kopi")
            .size(Size::Logical(LogicalSize::new(WIDTH, HEIGHT)))
            .level(PanelLevel::Floating)
            .style_mask(StyleMask::empty().nonactivating_panel())
            // Over full-screen apps, on whichever Space is active.
            .collection_behavior(
                CollectionBehavior::new()
                    .can_join_all_spaces()
                    .full_screen_auxiliary(),
            )
            .hides_on_deactivate(false)
            .has_shadow(true)
            // The page draws the rounded corners.
            .transparent(true)
            .no_activate(true)
            .with_window(|w| {
                w.decorations(false)
                    .transparent(true)
                    .visible(false)
                    .resizable(false)
                    .skip_taskbar(true)
                    .accept_first_mouse(true)
            })
            .build()?;
        Ok(())
    }

    pub fn show(app: &AppHandle) {
        let (Some(mtm), Ok(panel)) = (MainThreadMarker::new(), app.get_webview_panel(PANEL)) else {
            return;
        };
        let window: &NSWindow = panel.as_panel();
        if !panel.is_visible() {
            place(mtm, window, 0.6);
        }
        // Not Tauri's set_focus(): that activates Kopi and takes the menu bar from the app the
        // user was in. This makes the panel key without activating, and focuses the webview.
        panel.show_and_make_key();
    }

    pub fn hide(app: &AppHandle) -> bool {
        match app.get_webview_panel(PANEL) {
            Ok(panel) if panel.is_visible() => {
                panel.hide();
                true
            }
            _ => false,
        }
    }

    pub fn is_visible(app: &AppHandle) -> bool {
        app.get_webview_panel(PANEL)
            .is_ok_and(|panel| panel.is_visible())
    }

    /// Whether one of Kopi's windows has keyboard focus. `NSApp.isActive` can't answer this:
    /// it's true whenever the panel is key, though Kopi isn't frontmost.
    pub fn has_key_window(_app: &AppHandle) -> bool {
        MainThreadMarker::new()
            .is_some_and(|mtm| NSApplication::sharedApplication(mtm).keyWindow().is_some())
    }

    pub fn is_main_thread() -> bool {
        MainThreadMarker::new().is_some()
    }

    /// Centers a window across the screen with the pointer. `rise` is how much of the spare
    /// height goes below it: 0.5 is the middle, 0.6 a little above.
    pub fn place(mtm: MainThreadMarker, window: &NSWindow, rise: f64) {
        let pointer = NSEvent::mouseLocation();
        let screens = NSScreen::screens(mtm);
        let screen = screens
            .iter()
            .find(|screen| {
                let f = screen.frame();
                pointer.x >= f.origin.x
                    && pointer.x < f.origin.x + f.size.width
                    && pointer.y >= f.origin.y
                    && pointer.y <= f.origin.y + f.size.height
            })
            .or_else(|| NSScreen::mainScreen(mtm));
        let Some(screen) = screen else {
            return;
        };
        let area = screen.visibleFrame();
        let size = window.frame().size;
        // Cocoa's y axis points up, so more than half the spare height puts it above the middle.
        window.setFrameOrigin(NSPoint::new(
            (area.origin.x + (area.size.width - size.width) / 2.0).round(),
            (area.origin.y + (area.size.height - size.height) * rise).round(),
        ));
    }
}

/// A borderless window over every other. Unlike the macOS panel, showing it takes focus from
/// the app the user was in, so hiding it while it has focus hands focus back, and a paste lands
/// there.
#[cfg(not(target_os = "macos"))]
mod platform {
    use std::sync::{Mutex, OnceLock, PoisonError};
    use std::thread::{self, ThreadId};

    use tauri::{
        AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    };

    use self::focus::{WindowRef, activate, foreground_window};
    use super::{HEIGHT, PANEL, WIDTH};

    #[cfg(target_os = "windows")]
    mod focus {
        pub use kopi_platform::windows::{WindowRef, activate, foreground_window};
    }

    #[cfg(not(target_os = "windows"))]
    mod focus {
        pub type WindowRef = ();

        pub fn foreground_window() -> Option<WindowRef> {
            None
        }

        pub fn activate(_: WindowRef) -> bool {
            false
        }
    }

    /// The thread that built the panel, which is the main thread.
    static MAIN: OnceLock<ThreadId> = OnceLock::new();

    /// The window that had focus when the panel opened.
    struct Previous(Mutex<Option<WindowRef>>);

    pub fn create(app: &AppHandle) -> tauri::Result<()> {
        let _ = MAIN.set(thread::current().id());
        app.manage(Previous(Mutex::new(None)));
        WebviewWindowBuilder::new(app, PANEL, WebviewUrl::App("index.html".into()))
            .title("Kopi")
            .inner_size(WIDTH, HEIGHT)
            .decorations(false)
            .transparent(true)
            // With a shadow, Windows draws a border around the page's rounded corners.
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(false)
            .center()
            .build()?;
        Ok(())
    }

    pub fn show(app: &AppHandle) {
        let Some(window) = app.get_webview_window(PANEL) else {
            return;
        };
        if !window.is_visible().unwrap_or(false) {
            *previous(app) = foreground_window();
            place(app, &window);
        }
        let _ = window.show();
        let _ = window.set_focus();
    }

    pub fn hide(app: &AppHandle) -> bool {
        let Some(window) = app.get_webview_window(PANEL) else {
            return false;
        };
        let visible = window.is_visible().unwrap_or(false);
        let before = previous(app).take();
        if visible {
            // While the panel has focus, Kopi may hand it on. Done before hiding, so Windows
            // doesn't pick another window first. Clicking another window took focus already.
            if window.is_focused().unwrap_or(false)
                && let Some(before) = before
            {
                activate(before);
            }
            let _ = window.hide();
        }
        visible
    }

    pub fn is_visible(app: &AppHandle) -> bool {
        app.get_webview_window(PANEL)
            .is_some_and(|window| window.is_visible().unwrap_or(false))
    }

    pub fn has_key_window(app: &AppHandle) -> bool {
        app.get_webview_window(PANEL)
            .is_some_and(|window| window.is_focused().unwrap_or(false))
    }

    pub fn is_main_thread() -> bool {
        MAIN.get() == Some(&thread::current().id())
    }

    fn previous(app: &AppHandle) -> std::sync::MutexGuard<'_, Option<WindowRef>> {
        app.state::<Previous>()
            .inner()
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Centers the panel across the screen with the pointer, a little above the middle.
    fn place(app: &AppHandle, window: &WebviewWindow) {
        let monitor = app
            .cursor_position()
            .ok()
            .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
            .or_else(|| app.primary_monitor().ok().flatten());
        let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
            return;
        };
        let area = monitor.work_area();
        let spare_x = i64::from(area.size.width) - i64::from(size.width);
        let spare_y = i64::from(area.size.height) - i64::from(size.height);
        let x = i64::from(area.position.x) + spare_x / 2;
        let y = i64::from(area.position.y) + spare_y * 2 / 5;
        let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
    }
}
