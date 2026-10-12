//! Spike for issue #5: a non-activating panel with Tauri 2.12 and tauri-nspanel 2.1.
//!
//! ⇧⌘V shows a borderless panel on the screen with the cursor. The menu bar icon switches
//! between the ways of focusing it and the window levels we want to compare. Everything is
//! logged to ~/Library/Logs/KopiPanelSpike.log, so we can see whether Kopi became active and
//! which app was frontmost before, during and after.

use std::{
    fs::OpenOptions,
    io::Write,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use objc2::{rc::Retained, runtime::AnyClass, runtime::AnyObject, MainThreadMarker, Message};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSEvent, NSScreen, NSView, NSWindow,
    NSWindowStyleMask, NSWorkspace,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSProcessInfo};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    ActivationPolicy, AppHandle, Emitter, LogicalSize, Manager, Size, WebviewUrl, WindowEvent,
    Wry,
};
use tauri_nspanel::{CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

// The macro brings its own imports, so it lives in its own module.
mod panel_class {
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

use panel_class::KopiPanel;

const PANEL: &str = "panel";

#[derive(Clone, Copy, PartialEq, Debug)]
enum FocusMode {
    /// tauri-nspanel's `show_and_make_key`: orderFrontRegardless + makeKeyWindow.
    Panel,
    /// The same, then make the WKWebView the first responder.
    PanelAndWebView,
    /// Tauri's `show()` + `set_focus()`, as PLATFORM_NOTES.md suggested.
    TauriSetFocus,
}

#[derive(Clone, Copy)]
struct Settings {
    focus: FocusMode,
    level: i64,
    hide_on_blur: bool,
}

struct Spike {
    settings: Mutex<Settings>,
    shown_at: Mutex<Option<Instant>>,
    front_before: Mutex<Option<i32>>,
    seq: AtomicU32,
    check_items: Mutex<Vec<CheckMenuItem<Wry>>>,
}

fn log(msg: impl AsRef<str>) {
    let line = format!(
        "{} {}\n",
        chrono::Local::now().format("%H:%M:%S%.3f"),
        msg.as_ref()
    );
    eprint!("{line}");
    let path = format!(
        "{}/Library/Logs/KopiPanelSpike.log",
        std::env::var("HOME").unwrap_or_default()
    );
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn ms(d: Duration) -> String {
    format!("{:.1} ms", d.as_secs_f64() * 1000.0)
}

/// The frontmost app as "Name (pid N)", and its pid.
fn front_app() -> (String, i32) {
    let ws = NSWorkspace::sharedWorkspace();
    match ws.frontmostApplication() {
        Some(app) => {
            let name = app
                .localizedName()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "?".into());
            let pid = app.processIdentifier();
            (format!("{name} (pid {pid})"), pid)
        }
        None => ("none".into(), -1),
    }
}

fn class_name(obj: &AnyObject) -> String {
    obj.class().name().to_string_lossy().into_owned()
}

fn first_responder(ns: &NSWindow) -> String {
    ns.firstResponder()
        .map(|r| class_name(&r))
        .unwrap_or_else(|| "nil".into())
}

fn find_webview(view: &NSView, cls: &AnyClass) -> Option<Retained<NSView>> {
    if view.isKindOfClass(cls) {
        return Some(view.retain());
    }
    for sub in view.subviews().iter() {
        if let Some(found) = find_webview(&sub, cls) {
            return Some(found);
        }
    }
    None
}

fn screen_with_mouse(mtm: MainThreadMarker) -> Option<Retained<NSScreen>> {
    let p = NSEvent::mouseLocation();
    for screen in NSScreen::screens(mtm).iter() {
        let f = screen.frame();
        if p.x >= f.origin.x
            && p.x < f.origin.x + f.size.width
            && p.y >= f.origin.y
            && p.y <= f.origin.y + f.size.height
        {
            return Some(screen);
        }
    }
    NSScreen::mainScreen(mtm)
}

fn describe_panel(ns: &NSWindow) -> String {
    let mask = ns.styleMask();
    format!(
        "class {}, level {}, styleMask {:#x} (nonactivating {}, titled {}), collectionBehavior {:#x}, hidesOnDeactivate {}",
        class_name(ns),
        ns.level(),
        mask.0,
        mask.contains(NSWindowStyleMask::NonactivatingPanel),
        mask.contains(NSWindowStyleMask::Titled),
        ns.collectionBehavior().0,
        ns.hidesOnDeactivate(),
    )
}

fn policy_name(mtm: MainThreadMarker) -> &'static str {
    match NSApplication::sharedApplication(mtm).activationPolicy() {
        NSApplicationActivationPolicy::Regular => "Regular",
        NSApplicationActivationPolicy::Accessory => "Accessory",
        NSApplicationActivationPolicy::Prohibited => "Prohibited",
        _ => "unknown",
    }
}

/// Runs `f` now if we're on the main thread, otherwise queues it there.
fn on_main(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
    if MainThreadMarker::new().is_some() {
        f(app);
    } else {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || f(&handle));
    }
}

/// Logs the state again after `delay`, because activation changes arrive asynchronously.
fn log_later(app: &AppHandle, delay: Duration, label: String) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(delay);
        let inner = handle.clone();
        let _ = handle.run_on_main_thread(move || {
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            let Ok(panel) = inner.get_webview_panel(PANEL) else {
                return;
            };
            let ns: &NSWindow = panel.as_panel();
            log(format!(
                "{label} +{}: Kopi active={}, panel visible={} key={}, frontmost: {}, first responder: {}",
                ms(delay),
                NSApplication::sharedApplication(mtm).isActive(),
                ns.isVisible(),
                ns.isKeyWindow(),
                front_app().0,
                first_responder(ns),
            ));
        });
    });
}

fn show_panel(app: &AppHandle, pressed_at: Option<Instant>) {
    let Some(mtm) = MainThreadMarker::new() else {
        log("show_panel called off the main thread");
        return;
    };
    let state = app.state::<Spike>();
    let settings = *state.settings.lock().unwrap();
    let Ok(panel) = app.get_webview_panel(PANEL) else {
        log("no panel");
        return;
    };
    let ns: &NSWindow = panel.as_panel();
    let seq = state.seq.fetch_add(1, Ordering::SeqCst) + 1;

    let (front, pid) = front_app();
    *state.front_before.lock().unwrap() = Some(pid);

    // Center on the visible frame of the screen with the cursor (Cocoa coordinates).
    let mouse = NSEvent::mouseLocation();
    let screen = screen_with_mouse(mtm);
    let screen_name = screen
        .as_ref()
        .map(|s| s.localizedName().to_string())
        .unwrap_or_else(|| "no screen".into());
    if let Some(screen) = &screen {
        let vf = screen.visibleFrame();
        let size = ns.frame().size;
        ns.setFrameOrigin(NSPoint::new(
            (vf.origin.x + (vf.size.width - size.width) / 2.0).round(),
            (vf.origin.y + (vf.size.height - size.height) / 2.0).round(),
        ));
    }
    panel.set_level(settings.level);

    let t = Instant::now();
    match settings.focus {
        FocusMode::Panel => panel.show_and_make_key(),
        FocusMode::PanelAndWebView => {
            panel.show_and_make_key();
            let webview = AnyClass::get(c"WKWebView")
                .and_then(|cls| ns.contentView().and_then(|v| find_webview(&v, cls)));
            match webview {
                Some(wv) => {
                    let ok = ns.makeFirstResponder(Some(&wv));
                    log(format!(
                        "show #{seq}: makeFirstResponder({}) returned {ok}",
                        class_name(&wv)
                    ));
                }
                None => log(format!("show #{seq}: no WKWebView found")),
            }
        }
        FocusMode::TauriSetFocus => {
            if let Some(w) = app.get_webview_window(PANEL) {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    }
    let took = t.elapsed();
    *state.shown_at.lock().unwrap() = Some(Instant::now());

    log(format!(
        "show #{seq} [{:?}, level {}] frontmost before: {front}; mouse ({:.0}, {:.0}) on {screen_name}; show call took {}{}",
        settings.focus,
        settings.level,
        mouse.x,
        mouse.y,
        ms(took),
        pressed_at
            .map(|p| format!(", {} after the hotkey", ms(p.elapsed())))
            .unwrap_or_default(),
    ));
    log(format!(
        "show #{seq} now: Kopi active={}, panel key={}, frontmost: {}, first responder: {}, on screen {}",
        NSApplication::sharedApplication(mtm).isActive(),
        ns.isKeyWindow(),
        front_app().0,
        first_responder(ns),
        ns.screen()
            .map(|s| s.localizedName().to_string())
            .unwrap_or_else(|| "none".into()),
    ));
    let _ = app.emit_to(PANEL, "panel-shown", seq);
    log_later(app, Duration::from_millis(300), format!("show #{seq}"));
}

fn hide_panel(app: &AppHandle, reason: &str) {
    let Some(mtm) = MainThreadMarker::new() else {
        log("hide_panel called off the main thread");
        return;
    };
    let Ok(panel) = app.get_webview_panel(PANEL) else {
        return;
    };
    if !panel.is_visible() {
        return;
    }
    let state = app.state::<Spike>();
    let seq = state.seq.load(Ordering::SeqCst);
    let shown_for = state
        .shown_at
        .lock()
        .unwrap()
        .map(|t| ms(t.elapsed()))
        .unwrap_or_default();

    panel.hide();

    let (front, pid) = front_app();
    let before = *state.front_before.lock().unwrap();
    log(format!(
        "hide #{seq} ({reason}) after {shown_for}: Kopi active={}, frontmost: {front}, same app as before show: {}",
        NSApplication::sharedApplication(mtm).isActive(),
        before == Some(pid),
    ));
    log_later(app, Duration::from_millis(300), format!("hide #{seq}"));
}

fn toggle_panel(app: &AppHandle, pressed_at: Instant) {
    let visible = app
        .get_webview_panel(PANEL)
        .map(|p| p.is_visible())
        .unwrap_or(false);
    if visible {
        hide_panel(app, "hotkey");
    } else {
        show_panel(app, Some(pressed_at));
    }
}

#[tauri::command]
fn hide(app: AppHandle, reason: String) {
    on_main(&app, move |app| hide_panel(app, &reason));
}

#[tauri::command]
fn js_log(app: AppHandle, msg: String) {
    let state = app.state::<Spike>();
    let since = state
        .shown_at
        .lock()
        .unwrap()
        .map(|t| format!(" +{}", ms(t.elapsed())))
        .unwrap_or_default();
    log(format!("[js{since}] {msg}"));
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_string();
    let state = app.state::<Spike>();
    match id.as_str() {
        "show" => on_main(app, |app| show_panel(app, None)),
        "log" => {
            let path = format!(
                "{}/Library/Logs/KopiPanelSpike.log",
                std::env::var("HOME").unwrap_or_default()
            );
            let _ = std::process::Command::new("open").arg(path).spawn();
        }
        "quit" => app.exit(0),
        "hide-on-blur" => {
            let mut s = state.settings.lock().unwrap();
            s.hide_on_blur = !s.hide_on_blur;
            log(format!("hide on blur: {}", s.hide_on_blur));
        }
        _ => {
            let Some((group, value)) = id.split_once(':') else {
                return;
            };
            {
                let mut s = state.settings.lock().unwrap();
                match (group, value) {
                    ("focus", "panel") => s.focus = FocusMode::Panel,
                    ("focus", "webview") => s.focus = FocusMode::PanelAndWebView,
                    ("focus", "tauri") => s.focus = FocusMode::TauriSetFocus,
                    ("level", n) => s.level = n.parse().unwrap_or(s.level),
                    _ => return,
                }
                log(format!("settings: focus {:?}, level {}", s.focus, s.level));
            }
            // Radio behaviour: only the clicked item in its group stays checked.
            let prefix = format!("{group}:");
            for item in state.check_items.lock().unwrap().iter() {
                if item.id().as_ref().starts_with(&prefix) {
                    let _ = item.set_checked(item.id().as_ref() == id);
                }
            }
        }
    }
}

fn main() {
    let hotkey = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);

    tauri::Builder::default()
        .plugin(tauri_nspanel::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut(hotkey)
                .expect("valid shortcut")
                .with_handler(move |app, shortcut, event| {
                    if shortcut == &hotkey && event.state == ShortcutState::Pressed {
                        let pressed_at = Instant::now();
                        on_main(app, move |app| toggle_panel(app, pressed_at));
                    }
                })
                .build(),
        )
        .manage(Spike {
            settings: Mutex::new(Settings {
                focus: FocusMode::Panel,
                level: PanelLevel::Floating.value(),
                hide_on_blur: true,
            }),
            shown_at: Mutex::new(None),
            front_before: Mutex::new(None),
            seq: AtomicU32::new(0),
            check_items: Mutex::new(Vec::new()),
        })
        .invoke_handler(tauri::generate_handler![hide, js_log])
        .on_window_event(|window, event| {
            if window.label() != PANEL {
                return;
            }
            if let WindowEvent::Focused(focused) = event {
                log(format!("tauri WindowEvent::Focused({focused})"));
                let app = window.app_handle();
                let hide_on_blur = app.state::<Spike>().settings.lock().unwrap().hide_on_blur;
                if !focused && hide_on_blur {
                    on_main(app, |app| hide_panel(app, "blur"));
                }
            }
        })
        .setup(|app| {
            let mtm = MainThreadMarker::new().expect("setup runs on the main thread");
            log("-----");
            log(format!(
                "launch: macOS {}, exe {}, policy at launch {}",
                NSProcessInfo::processInfo().operatingSystemVersionString(),
                std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
                policy_name(mtm),
            ));

            app.set_activation_policy(ActivationPolicy::Accessory);
            log(format!("policy after set_activation_policy: {}", policy_name(mtm)));

            let panel = PanelBuilder::<_, KopiPanel>::new(app.handle(), PANEL)
                .url(WebviewUrl::App("index.html".into()))
                .size(Size::Logical(LogicalSize::new(640.0, 400.0)))
                .level(PanelLevel::Floating)
                .style_mask(StyleMask::empty().nonactivating_panel())
                .collection_behavior(
                    CollectionBehavior::new()
                        .can_join_all_spaces()
                        .full_screen_auxiliary(),
                )
                .hides_on_deactivate(false)
                .has_shadow(true)
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
            log(format!("panel built: {}", describe_panel(panel.as_panel())));
            log(format!(
                "after build: Kopi active={}, frontmost: {}",
                NSApplication::sharedApplication(mtm).isActive(),
                front_app().0
            ));

            let check = |id: &str, text: &str, checked: bool| {
                CheckMenuItem::with_id(app, id, text, true, checked, None::<&str>)
            };
            let focus_items = [
                check("focus:panel", "show_and_make_key (tauri-nspanel)", true)?,
                check("focus:webview", "show_and_make_key + focus WKWebView", false)?,
                check("focus:tauri", "Tauri show() + set_focus()", false)?,
            ];
            let level_items = [
                check("level:4", "Floating (4)", true)?,
                check("level:25", "Status (25)", false)?,
                check("level:101", "Pop-up menu (101)", false)?,
            ];
            let hide_on_blur = check("hide-on-blur", "Hide on blur", true)?;
            let focus_menu = Submenu::with_items(
                app,
                "Focus method",
                true,
                &[&focus_items[0], &focus_items[1], &focus_items[2]],
            )?;
            let level_menu = Submenu::with_items(
                app,
                "Panel level",
                true,
                &[&level_items[0], &level_items[1], &level_items[2]],
            )?;
            let menu = Menu::with_items(
                app,
                &[
                    &MenuItem::with_id(app, "show", "Show panel (⇧⌘V)", true, None::<&str>)?,
                    &PredefinedMenuItem::separator(app)?,
                    &focus_menu,
                    &level_menu,
                    &hide_on_blur,
                    &PredefinedMenuItem::separator(app)?,
                    &MenuItem::with_id(app, "log", "Open log", true, None::<&str>)?,
                    &MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?,
                ],
            )?;
            app.state::<Spike>()
                .check_items
                .lock()
                .unwrap()
                .extend(focus_items.into_iter().chain(level_items));

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("icon").clone())
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(on_menu)
                .build(app)?;

            log("ready: press ⇧⌘V");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the spike");
}
