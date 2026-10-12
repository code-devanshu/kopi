//! Spike for issue #6: paste into the active app on macOS.
//!
//! ⇧⌘V shows the non-activating panel from spike 0002. Return writes the selected clip to the
//! pasteboard, hides the panel and posts ⌘V with CGEvent. ⇧⌘1 pastes the first clip straight
//! away, while ⇧⌘ are still held. The menu bar icon switches between the key code, timing and
//! event options we want to compare. Everything is logged to ~/Library/Logs/KopiPasteSpike.log.

use std::{
    fs::OpenOptions,
    io::Write,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use core_foundation::{
    base::TCFType,
    boolean::CFBoolean,
    dictionary::CFDictionary,
    string::{CFString, CFStringRef},
};
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSEvent, NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypeString, NSScreen, NSWindow,
    NSWorkspace,
};
use objc2_foundation::{NSPoint, NSProcessInfo, NSString};
use tauri::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
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

/// The C APIs this spike needs. All of them are plain functions, so we declare them here rather
/// than pull in a crate for each framework.
#[allow(non_snake_case, non_upper_case_globals)]
mod ffi {
    use std::ffi::c_void;

    pub type Ref = *const c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        pub fn AXIsProcessTrusted() -> u8;
        pub fn AXIsProcessTrustedWithOptions(options: Ref) -> u8;
        pub fn AXUIElementCreateSystemWide() -> Ref;
        pub fn AXUIElementCopyAttributeValue(element: Ref, attribute: Ref, value: *mut Ref) -> i32;
        pub fn AXUIElementGetPid(element: Ref, pid: *mut i32) -> i32;
        pub fn AXUIElementSetMessagingTimeout(element: Ref, seconds: f32) -> i32;
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        pub fn CGPreflightPostEventAccess() -> bool;
        pub fn CGRequestPostEventAccess() -> bool;
        pub fn CGEventSourceCreate(state: i32) -> Ref;
        pub fn CGEventSourceSetLocalEventsFilterDuringSuppressionState(
            source: Ref,
            filter: u32,
            state: u32,
        );
        pub fn CGEventSourceFlagsState(state: i32) -> u64;
        pub fn CGEventSourceKeyState(state: i32, key: u16) -> bool;
        pub fn CGEventCreateKeyboardEvent(source: Ref, key: u16, down: bool) -> Ref;
        pub fn CGEventSetFlags(event: Ref, flags: u64);
        pub fn CGEventGetFlags(event: Ref) -> u64;
        pub fn CGEventKeyboardGetUnicodeString(
            event: Ref,
            max: usize,
            actual: *mut usize,
            chars: *mut u16,
        );
        pub fn CGEventPost(tap: u32, event: Ref);
        pub fn CGEventPostToPid(pid: i32, event: Ref);
    }

    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        pub static kTISPropertyUnicodeKeyLayoutData: Ref;
        pub static kTISPropertyLocalizedName: Ref;
        pub static kTISPropertyInputSourceID: Ref;
        pub static kTISPropertyInputSourceType: Ref;
        pub static kTISTypeKeyboardLayout: Ref;
        pub fn TISCopyCurrentKeyboardInputSource() -> Ref;
        pub fn TISCopyCurrentKeyboardLayoutInputSource() -> Ref;
        pub fn TISCreateInputSourceList(properties: Ref, include_all_installed: u8) -> Ref;
        pub fn TISGetInputSourceProperty(source: Ref, key: Ref) -> Ref;
        pub fn LMGetKbdType() -> u8;
        pub fn UCKeyTranslate(
            layout: *const u8,
            key: u16,
            action: u16,
            modifiers: u32,
            keyboard_type: u32,
            options: u32,
            dead_key_state: *mut u32,
            max: usize,
            actual: *mut usize,
            chars: *mut u16,
        ) -> i32;
        pub fn IsSecureEventInputEnabled() -> u8;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        pub fn CFRelease(cf: Ref);
        pub fn CFGetTypeID(cf: Ref) -> usize;
        pub fn CFStringGetTypeID() -> usize;
        pub fn CFDataGetBytePtr(data: Ref) -> *const u8;
        pub fn CFArrayGetCount(array: Ref) -> isize;
        pub fn CFArrayGetValueAtIndex(array: Ref, index: isize) -> Ref;
    }

    // CGEventFlags
    pub const SHIFT: u64 = 0x0002_0000;
    pub const CONTROL: u64 = 0x0004_0000;
    pub const OPTION: u64 = 0x0008_0000;
    pub const COMMAND: u64 = 0x0010_0000;
    /// NX_DEVICELCMDKEYMASK: the device-dependent bit for the left ⌘ key. Maccy sets it too.
    pub const LEFT_COMMAND_DEVICE: u64 = 0x0000_0008;
    pub const MODIFIERS: u64 = SHIFT | CONTROL | OPTION | COMMAND;

    // CGEventFilterMask and CGEventSuppressionState
    pub const PERMIT_LOCAL_MOUSE: u32 = 0x1;
    pub const PERMIT_SYSTEM_DEFINED: u32 = 0x4;
    pub const SUPPRESSION_INTERVAL: u32 = 0;

    // CGEventSourceStateID
    pub const COMBINED_SESSION: i32 = 0;

    // Virtual key codes (ANSI positions)
    pub const KEY_ANSI_V: u16 = 9;
    pub const KEY_RETURN: u16 = 36;
    pub const KEY_COMMAND: u16 = 55;
}

const PANEL: &str = "panel";
const BUNDLE_ID: &str = "in.devanshuverma.kopi.pastespike";

#[derive(Clone, Copy, PartialEq, Debug)]
enum KeyCode {
    /// The key that types "v" in the current layout while ⌘ is held.
    LayoutWithCommand,
    /// The key that types "v" in the current layout with no modifiers.
    Layout,
    /// Always 9, the V position on an ANSI QWERTY keyboard.
    AnsiV,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Sequence {
    /// V down and up, both carrying the ⌘ flag. This is what Maccy does.
    VWithFlag,
    /// ⌘ down, V down, V up, ⌘ up.
    CommandKeyAround,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Flags {
    /// ⌘ plus the left-⌘ device bit, as Maccy does.
    CommandAndDevice,
    /// ⌘ only.
    Command,
    /// Leave whatever the event source puts on the event.
    Source,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Tap {
    Session,
    Hid,
    TargetPid,
}

#[derive(Clone, Copy, Debug)]
struct Settings {
    key: KeyCode,
    delay_ms: u64,
    sequence: Sequence,
    flags: Flags,
    /// CGEventSourceStateID: -1 private, 0 combined session, 1 HID system.
    source_state: i32,
    tap: Tap,
    wait_for_modifiers: bool,
    suppress_local_keys: bool,
    hide_on_blur: bool,
}

struct Spike {
    settings: Mutex<Settings>,
    target: Mutex<Option<(String, i32)>>,
    shows: AtomicU32,
    pastes: AtomicU32,
    radio_items: Mutex<Vec<CheckMenuItem<Wry>>>,
}

struct Clip {
    label: &'static str,
    text: String,
    html: Option<String>,
}

fn clips(paste_no: u32) -> Vec<Clip> {
    let plain = |label, text: String| Clip {
        label,
        text,
        html: None,
    };
    vec![
        plain("Plain text", format!("Kopi paste test {paste_no}")),
        plain(
            "Two lines",
            format!("Kopi line one ({paste_no})\nKopi line two"),
        ),
        plain(
            "Unicode",
            format!("Héllo — “quotes” ✓ 日本語 👋 ({paste_no})"),
        ),
        Clip {
            label: "Rich text (HTML)",
            text: format!("Bold, italic and a link ({paste_no})"),
            html: Some(format!(
                "<b>Bold</b>, <i>italic</i> and a <a href=\"https://example.com\">link</a> ({paste_no})"
            )),
        },
        plain(
            "Long text (about 5,000 characters)",
            format!(
                "Kopi long paste {paste_no}: {}END",
                "The quick brown fox jumps over the lazy dog. ".repeat(110)
            ),
        ),
        plain("Shell command", format!("echo kopi-paste-ok-{paste_no}")),
    ]
}

fn log(msg: impl AsRef<str>) {
    let line = format!(
        "{} {}\n",
        chrono::Local::now().format("%H:%M:%S%.3f"),
        msg.as_ref()
    );
    eprint!("{line}");
    let path = format!(
        "{}/Library/Logs/KopiPasteSpike.log",
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

fn screen_with_mouse(mtm: MainThreadMarker) -> Option<objc2::rc::Retained<NSScreen>> {
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

/// Runs `f` now if we're on the main thread, otherwise queues it there.
fn on_main(app: &AppHandle, f: impl FnOnce(&AppHandle) + Send + 'static) {
    if MainThreadMarker::new().is_some() {
        f(app);
    } else {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || f(&handle));
    }
}

// ---------------------------------------------------------------------------------------------
// Permissions

fn ax_trusted() -> bool {
    unsafe { ffi::AXIsProcessTrusted() != 0 }
}

fn post_event_access() -> bool {
    unsafe { ffi::CGPreflightPostEventAccess() }
}

fn secure_input() -> bool {
    unsafe { ffi::IsSecureEventInputEnabled() != 0 }
}

fn ax_prompt() -> bool {
    let options = CFDictionary::from_CFType_pairs(&[(
        CFString::from_static_string("AXTrustedCheckOptionPrompt"),
        CFBoolean::true_value(),
    )]);
    unsafe { ffi::AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef() as ffi::Ref) != 0 }
}

fn permission_text() -> String {
    format!(
        "Accessibility: {} (post events: {})",
        if ax_trusted() { "granted" } else { "not granted" },
        if post_event_access() { "yes" } else { "no" },
    )
}

// ---------------------------------------------------------------------------------------------
// Keyboard layouts

/// A CFString we don't own, as a Rust string.
fn cf_str(s: ffi::Ref) -> String {
    if s.is_null() {
        return "?".into();
    }
    unsafe {
        if ffi::CFGetTypeID(s) != ffi::CFStringGetTypeID() {
            return "(not a string)".into();
        }
        CFString::wrap_under_get_rule(s as CFStringRef).to_string()
    }
}

struct Layout {
    name: String,
    id: String,
    /// The key that types "v" with no modifiers.
    plain: Option<u16>,
    /// The key that types "v" while ⌘ is held.
    command: Option<u16>,
}

impl Layout {
    fn describe(&self) -> String {
        let code = |c: Option<u16>| c.map(|c| c.to_string()).unwrap_or_else(|| "none".into());
        format!(
            "{} ({}): V key {} plain, {} with ⌘",
            self.name,
            self.id,
            code(self.plain),
            code(self.command)
        )
    }
}

/// Finds the key that types `ch` in a 'uchr' layout. `modifiers` is the Carbon modifier state
/// shifted right by 8 (1 = ⌘).
fn find_key(layout: *const u8, ch: char, modifiers: u32) -> Option<u16> {
    let keyboard_type = unsafe { ffi::LMGetKbdType() } as u32;
    for key in 0..128u16 {
        let mut dead = 0u32;
        let mut len = 0usize;
        let mut buf = [0u16; 4];
        let status = unsafe {
            ffi::UCKeyTranslate(
                layout,
                key,
                0, // kUCKeyActionDown
                modifiers,
                keyboard_type,
                1, // kUCKeyTranslateNoDeadKeysMask
                &mut dead,
                buf.len(),
                &mut len,
                buf.as_mut_ptr(),
            )
        };
        if status == 0 && len == 1 && buf[0] == ch as u16 {
            return Some(key);
        }
    }
    None
}

/// Must run on the main thread: Text Input Sources complain otherwise.
fn layout_info(source: ffi::Ref) -> Layout {
    unsafe {
        let name = cf_str(ffi::TISGetInputSourceProperty(
            source,
            ffi::kTISPropertyLocalizedName,
        ));
        let id = cf_str(ffi::TISGetInputSourceProperty(
            source,
            ffi::kTISPropertyInputSourceID,
        ));
        let data = ffi::TISGetInputSourceProperty(source, ffi::kTISPropertyUnicodeKeyLayoutData);
        if data.is_null() {
            return Layout {
                name,
                id,
                plain: None,
                command: None,
            };
        }
        let bytes = ffi::CFDataGetBytePtr(data);
        Layout {
            name,
            id,
            plain: find_key(bytes, 'v', 0),
            command: find_key(bytes, 'v', 1),
        }
    }
}

fn current_layout() -> Layout {
    unsafe {
        let source = ffi::TISCopyCurrentKeyboardLayoutInputSource();
        if source.is_null() {
            return Layout {
                name: "none".into(),
                id: "none".into(),
                plain: None,
                command: None,
            };
        }
        let layout = layout_info(source);
        ffi::CFRelease(source);
        layout
    }
}

/// The current input source, which is an input method (not a layout) when one is in use.
fn current_input_source() -> String {
    unsafe {
        let source = ffi::TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return "none".into();
        }
        let id = cf_str(ffi::TISGetInputSourceProperty(
            source,
            ffi::kTISPropertyInputSourceID,
        ));
        ffi::CFRelease(source);
        id
    }
}

/// Every installed keyboard layout, enabled or not.
fn all_layouts() -> Vec<Layout> {
    let filter = CFDictionary::from_CFType_pairs(&[(
        unsafe { CFString::wrap_under_get_rule(ffi::kTISPropertyInputSourceType as CFStringRef) },
        unsafe { CFString::wrap_under_get_rule(ffi::kTISTypeKeyboardLayout as CFStringRef) },
    )]);
    let mut out = Vec::new();
    unsafe {
        let list = ffi::TISCreateInputSourceList(filter.as_concrete_TypeRef() as ffi::Ref, 1);
        if list.is_null() {
            return out;
        }
        for i in 0..ffi::CFArrayGetCount(list) {
            out.push(layout_info(ffi::CFArrayGetValueAtIndex(list, i)));
        }
        ffi::CFRelease(list);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Pasting

/// Writes the clip and marks it as ours. Returns the change count before and after.
fn write_clip(clip: &Clip) -> (isize, isize) {
    let pb = NSPasteboard::generalPasteboard();
    let before = pb.changeCount();
    pb.clearContents();
    unsafe {
        pb.setString_forType(&NSString::from_str(&clip.text), NSPasteboardTypeString);
        if let Some(html) = &clip.html {
            pb.setString_forType(&NSString::from_str(html), NSPasteboardTypeHTML);
        }
        pb.setString_forType(
            &NSString::from_str(BUNDLE_ID),
            &NSString::from_str("org.nspasteboard.source"),
        );
    }
    (before, pb.changeCount())
}

fn flag_names(flags: u64) -> String {
    let names: Vec<&str> = [
        (ffi::CONTROL, "⌃"),
        (ffi::OPTION, "⌥"),
        (ffi::SHIFT, "⇧"),
        (ffi::COMMAND, "⌘"),
    ]
    .iter()
    .filter(|(bit, _)| flags & bit != 0)
    .map(|(_, name)| *name)
    .collect();
    if names.is_empty() {
        "none".into()
    } else {
        names.concat()
    }
}

fn event_chars(event: ffi::Ref) -> String {
    let mut buf = [0u16; 8];
    let mut len = 0usize;
    unsafe { ffi::CGEventKeyboardGetUnicodeString(event, buf.len(), &mut len, buf.as_mut_ptr()) };
    String::from_utf16_lossy(&buf[..len.min(buf.len())])
}

/// Posts the paste keystroke and describes what was sent.
fn post_paste(s: &Settings, key: u16, pid: i32) -> String {
    unsafe {
        let source = ffi::CGEventSourceCreate(s.source_state);
        if s.suppress_local_keys && !source.is_null() {
            ffi::CGEventSourceSetLocalEventsFilterDuringSuppressionState(
                source,
                ffi::PERMIT_LOCAL_MOUSE | ffi::PERMIT_SYSTEM_DEFINED,
                ffi::SUPPRESSION_INTERVAL,
            );
        }
        let flags = match s.flags {
            Flags::CommandAndDevice => Some(ffi::COMMAND | ffi::LEFT_COMMAND_DEVICE),
            Flags::Command => Some(ffi::COMMAND),
            Flags::Source => None,
        };
        let events: Vec<(u16, bool, Option<u64>)> = match s.sequence {
            Sequence::VWithFlag => vec![(key, true, flags), (key, false, flags)],
            Sequence::CommandKeyAround => vec![
                (ffi::KEY_COMMAND, true, flags),
                (key, true, flags),
                (key, false, flags),
                (ffi::KEY_COMMAND, false, flags.map(|_| 0)),
            ],
        };
        let mut v_down = String::new();
        for (code, down, flags) in events.iter().copied() {
            let event = ffi::CGEventCreateKeyboardEvent(source, code, down);
            if event.is_null() {
                if !source.is_null() {
                    ffi::CFRelease(source);
                }
                return format!("CGEventCreateKeyboardEvent({code}) returned NULL");
            }
            if let Some(flags) = flags {
                ffi::CGEventSetFlags(event, flags);
            }
            if code == key && down {
                let f = ffi::CGEventGetFlags(event);
                v_down = format!(
                    "V down: key {key}, flags {:#x} ({}), chars {:?}",
                    f,
                    flag_names(f),
                    event_chars(event)
                );
            }
            match s.tap {
                Tap::Session => ffi::CGEventPost(1, event),
                Tap::Hid => ffi::CGEventPost(0, event),
                Tap::TargetPid => ffi::CGEventPostToPid(pid, event),
            }
            ffi::CFRelease(event);
        }
        if !source.is_null() {
            ffi::CFRelease(source);
        }
        format!("posted {} events; {v_down}", events.len())
    }
}

/// Polls until ⇧⌃⌥⌘ are all up, or `max` passes. Returns whether they came up and how long it took.
fn wait_for_modifiers(max: Duration) -> (bool, Duration) {
    let t = Instant::now();
    loop {
        let held = unsafe { ffi::CGEventSourceFlagsState(ffi::COMBINED_SESSION) } & ffi::MODIFIERS;
        if held == 0 {
            return (true, t.elapsed());
        }
        if t.elapsed() >= max {
            return (false, t.elapsed());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// What the Accessibility API says has focus: the app's pid and the focused element's role.
fn ax_focus() -> String {
    unsafe {
        let system = ffi::AXUIElementCreateSystemWide();
        ffi::AXUIElementSetMessagingTimeout(system, 0.25);
        let mut app: ffi::Ref = std::ptr::null();
        let attr = CFString::from_static_string("AXFocusedApplication");
        let err = ffi::AXUIElementCopyAttributeValue(
            system,
            attr.as_concrete_TypeRef() as ffi::Ref,
            &mut app,
        );
        if err != 0 {
            ffi::CFRelease(system);
            return format!("AX focused app: error {err}");
        }
        let mut pid = 0;
        ffi::AXUIElementGetPid(app, &mut pid);
        ffi::CFRelease(app);

        let mut element: ffi::Ref = std::ptr::null();
        let attr = CFString::from_static_string("AXFocusedUIElement");
        let err = ffi::AXUIElementCopyAttributeValue(
            system,
            attr.as_concrete_TypeRef() as ffi::Ref,
            &mut element,
        );
        ffi::CFRelease(system);
        if err != 0 {
            return format!("AX focused app pid {pid}, focused element: error {err}");
        }
        let mut role: ffi::Ref = std::ptr::null();
        let attr = CFString::from_static_string("AXRole");
        let err = ffi::AXUIElementCopyAttributeValue(
            element,
            attr.as_concrete_TypeRef() as ffi::Ref,
            &mut role,
        );
        ffi::CFRelease(element);
        if err != 0 {
            return format!("AX focused app pid {pid}, element role: error {err}");
        }
        let role_name = cf_str(role);
        ffi::CFRelease(role);
        format!("AX focused app pid {pid}, element {role_name}")
    }
}

/// Writes clip `index`, hides the panel and posts ⌘V to the remembered target app.
fn paste(app: &AppHandle, index: usize, trigger: &'static str, started: Instant) {
    let state = app.state::<Spike>();
    let s = *state.settings.lock().unwrap();
    let n = state.pastes.fetch_add(1, Ordering::SeqCst) + 1;
    let (target_name, target_pid) = state
        .target
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| ("unknown".into(), -1));

    // Text Input Sources must be read on the main thread, so look the key up before leaving it.
    let layout = current_layout();
    let key = match s.key {
        KeyCode::LayoutWithCommand => layout.command,
        KeyCode::Layout => layout.plain,
        KeyCode::AnsiV => Some(ffi::KEY_ANSI_V),
    }
    .unwrap_or(ffi::KEY_ANSI_V);

    let clip = clips(n).swap_remove(index.min(5));
    let (count_before, count_after) = write_clip(&clip);
    hide_panel(app, trigger);
    let hidden_at = Instant::now();

    let trusted = ax_trusted();
    log(format!(
        "paste #{n} ({trigger}): \"{}\" ({} chars{}) for {target_name}; pasteboard changeCount {count_before} -> {count_after}",
        clip.label,
        clip.text.chars().count(),
        if clip.html.is_some() { " + HTML" } else { "" },
    ));
    log(format!(
        "paste #{n}: Accessibility {trusted}, post events {}, secure input {}; input source {}; layout {}; using key {key} ({:?})",
        post_event_access(),
        secure_input(),
        current_input_source(),
        layout.describe(),
        s.key,
    ));
    log(format!("paste #{n}: settings {s:?}"));
    if !trusted {
        log(format!(
            "paste #{n}: no Accessibility, so Kopi would stop here and say \"Copied. Press ⌘V\". The spike posts anyway to see what happens."
        ));
    }

    let job = move || {
        let mut waited = String::new();
        if s.wait_for_modifiers {
            let (released, took) = wait_for_modifiers(Duration::from_secs(1));
            waited = format!(
                "; waited {} for modifiers ({})",
                ms(took),
                if released { "released" } else { "still held, gave up" }
            );
        }
        if s.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(s.delay_ms));
        }
        let held = unsafe { ffi::CGEventSourceFlagsState(ffi::COMBINED_SESSION) };
        let return_down = unsafe { ffi::CGEventSourceKeyState(ffi::COMBINED_SESSION, ffi::KEY_RETURN) };
        let (front, front_pid) = front_app();
        let posted = post_paste(&s, key, target_pid);
        log(format!(
            "paste #{n}: {posted}; {} after hiding, {} after the trigger{waited}; held before posting: {} (Return down {return_down}); frontmost: {front} (target {})",
            ms(hidden_at.elapsed()),
            ms(started.elapsed()),
            flag_names(held),
            if front_pid == target_pid { "yes" } else { "NO" },
        ));
        std::thread::sleep(Duration::from_millis(50));
        log(format!("paste #{n} +50 ms: {}", ax_focus()));
    };
    if s.delay_ms == 0 && !s.wait_for_modifiers {
        job();
    } else {
        std::thread::spawn(job);
    }
}

// ---------------------------------------------------------------------------------------------
// Panel

fn show_panel(app: &AppHandle) {
    let Some(mtm) = MainThreadMarker::new() else {
        log("show_panel called off the main thread");
        return;
    };
    let Ok(panel) = app.get_webview_panel(PANEL) else {
        log("no panel");
        return;
    };
    let state = app.state::<Spike>();
    let seq = state.shows.fetch_add(1, Ordering::SeqCst) + 1;
    let target = front_app();
    log(format!("show #{seq}: target {}", target.0));
    *state.target.lock().unwrap() = Some(target);

    let ns: &NSWindow = panel.as_panel();
    if let Some(screen) = screen_with_mouse(mtm) {
        let vf = screen.visibleFrame();
        let size = ns.frame().size;
        ns.setFrameOrigin(NSPoint::new(
            (vf.origin.x + (vf.size.width - size.width) / 2.0).round(),
            (vf.origin.y + (vf.size.height - size.height) / 2.0).round(),
        ));
    }
    panel.show_and_make_key();
    let _ = app.emit_to(PANEL, "panel-shown", seq);
}

fn hide_panel(app: &AppHandle, reason: &str) {
    let Ok(panel) = app.get_webview_panel(PANEL) else {
        return;
    };
    if !panel.is_visible() {
        return;
    }
    panel.hide();
    log(format!("hide ({reason}): frontmost {}", front_app().0));
}

#[tauri::command]
fn clip_labels() -> Vec<&'static str> {
    clips(0).iter().map(|c| c.label).collect()
}

#[tauri::command]
fn pick(app: AppHandle, index: usize) {
    let started = Instant::now();
    on_main(&app, move |app| paste(app, index, "return", started));
}

#[tauri::command]
fn hide(app: AppHandle, reason: String) {
    on_main(&app, move |app| hide_panel(app, &reason));
}

#[tauri::command]
fn js_log(msg: String) {
    log(format!("[js] {msg}"));
}

// ---------------------------------------------------------------------------------------------
// Menu

fn open_log() {
    let path = format!(
        "{}/Library/Logs/KopiPasteSpike.log",
        std::env::var("HOME").unwrap_or_default()
    );
    let _ = std::process::Command::new("open").arg(path).spawn();
}

fn on_menu(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_string();
    let state = app.state::<Spike>();
    match id.as_str() {
        "show" => on_main(app, show_panel),
        "ax-prompt" => log(format!("AXIsProcessTrustedWithOptions(prompt) returned {}", ax_prompt())),
        "cg-request" => log(format!(
            "CGRequestPostEventAccess returned {}",
            unsafe { ffi::CGRequestPostEventAccess() }
        )),
        "ax-settings" => {
            let _ = std::process::Command::new("open")
                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
                .spawn();
        }
        "layouts" => on_main(app, |_| {
            log(format!("current: {}", current_layout().describe()));
            for layout in all_layouts() {
                log(format!("installed: {}", layout.describe()));
            }
        }),
        "log" => open_log(),
        "quit" => app.exit(0),
        _ => {
            let Some((group, value)) = id.split_once(':') else {
                return;
            };
            {
                let mut s = state.settings.lock().unwrap();
                match (group, value) {
                    ("key", "layout-cmd") => s.key = KeyCode::LayoutWithCommand,
                    ("key", "layout") => s.key = KeyCode::Layout,
                    ("key", "ansi") => s.key = KeyCode::AnsiV,
                    ("delay", n) => s.delay_ms = n.parse().unwrap_or(s.delay_ms),
                    ("seq", "v") => s.sequence = Sequence::VWithFlag,
                    ("seq", "cmd") => s.sequence = Sequence::CommandKeyAround,
                    ("flags", "cmd-device") => s.flags = Flags::CommandAndDevice,
                    ("flags", "cmd") => s.flags = Flags::Command,
                    ("flags", "source") => s.flags = Flags::Source,
                    ("source", n) => s.source_state = n.parse().unwrap_or(s.source_state),
                    ("tap", "session") => s.tap = Tap::Session,
                    ("tap", "hid") => s.tap = Tap::Hid,
                    ("tap", "pid") => s.tap = Tap::TargetPid,
                    ("toggle", "wait") => s.wait_for_modifiers = !s.wait_for_modifiers,
                    ("toggle", "suppress") => s.suppress_local_keys = !s.suppress_local_keys,
                    ("toggle", "blur") => s.hide_on_blur = !s.hide_on_blur,
                    _ => return,
                }
                log(format!("settings: {s:?}"));
            }
            if group == "toggle" {
                return; // check items toggle themselves
            }
            // Radio behaviour: only the clicked item in its group stays checked.
            let prefix = format!("{group}:");
            for item in state.radio_items.lock().unwrap().iter() {
                if item.id().as_ref().starts_with(&prefix) {
                    let _ = item.set_checked(item.id().as_ref() == id);
                }
            }
        }
    }
}

/// A submenu whose items behave like radio buttons. The first option is the default.
fn radio(
    app: &AppHandle,
    group: &str,
    title: &str,
    options: &[(&str, &str)],
) -> tauri::Result<Submenu<Wry>> {
    let mut items = Vec::new();
    for (i, (value, label)) in options.iter().enumerate() {
        items.push(CheckMenuItem::with_id(
            app,
            format!("{group}:{value}"),
            *label,
            true,
            i == 0,
            None::<&str>,
        )?);
    }
    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|i| i as &dyn IsMenuItem<Wry>).collect();
    let submenu = Submenu::with_items(app, title, true, &refs)?;
    app.state::<Spike>().radio_items.lock().unwrap().extend(items);
    Ok(submenu)
}

fn main() {
    let show_hotkey = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyV);
    let quick_hotkey = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Digit1);

    tauri::Builder::default()
        .plugin(tauri_nspanel::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([show_hotkey, quick_hotkey])
                .expect("valid shortcuts")
                .with_handler(move |app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let pressed_at = Instant::now();
                    if shortcut == &show_hotkey {
                        on_main(app, |app| {
                            let visible = app
                                .get_webview_panel(PANEL)
                                .map(|p| p.is_visible())
                                .unwrap_or(false);
                            if visible {
                                hide_panel(app, "hotkey");
                            } else {
                                show_panel(app);
                            }
                        });
                    } else if shortcut == &quick_hotkey {
                        on_main(app, move |app| {
                            let target = front_app();
                            log(format!("⇧⌘1: target {}", target.0));
                            *app.state::<Spike>().target.lock().unwrap() = Some(target);
                            paste(app, 0, "⇧⌘1", pressed_at);
                        });
                    }
                })
                .build(),
        )
        .manage(Spike {
            settings: Mutex::new(Settings {
                key: KeyCode::LayoutWithCommand,
                delay_ms: 0,
                sequence: Sequence::VWithFlag,
                flags: Flags::CommandAndDevice,
                source_state: ffi::COMBINED_SESSION,
                tap: Tap::Session,
                wait_for_modifiers: false,
                suppress_local_keys: true,
                hide_on_blur: true,
            }),
            target: Mutex::new(None),
            shows: AtomicU32::new(0),
            pastes: AtomicU32::new(0),
            radio_items: Mutex::new(Vec::new()),
        })
        .invoke_handler(tauri::generate_handler![clip_labels, pick, hide, js_log])
        .on_window_event(|window, event| {
            if window.label() != PANEL {
                return;
            }
            if let WindowEvent::Focused(false) = event {
                let app = window.app_handle();
                if app.state::<Spike>().settings.lock().unwrap().hide_on_blur {
                    on_main(app, |app| hide_panel(app, "blur"));
                }
            }
        })
        .setup(|app| {
            log("-----");
            log(format!(
                "launch: macOS {}, exe {}",
                NSProcessInfo::processInfo().operatingSystemVersionString(),
                std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ));
            log(format!("{}, secure input {}", permission_text(), secure_input()));
            log(format!(
                "input source {}; layout {}",
                current_input_source(),
                current_layout().describe()
            ));
            for layout in all_layouts() {
                log(format!("installed: {}", layout.describe()));
            }

            app.set_activation_policy(ActivationPolicy::Accessory);

            PanelBuilder::<_, KopiPanel>::new(app.handle(), PANEL)
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

            let h = app.handle();
            let permission = MenuItem::with_id(h, "permission", permission_text(), false, None::<&str>)?;
            let key_menu = radio(
                h,
                "key",
                "V key code",
                &[
                    ("layout-cmd", "From the layout, with ⌘ held"),
                    ("layout", "From the layout, no modifiers"),
                    ("ansi", "Always 9 (ANSI V)"),
                ],
            )?;
            let delay_menu = radio(
                h,
                "delay",
                "Delay after hiding",
                &[
                    ("0", "0 ms"),
                    ("10", "10 ms"),
                    ("30", "30 ms"),
                    ("50", "50 ms"),
                    ("100", "100 ms"),
                    ("250", "250 ms"),
                ],
            )?;
            let seq_menu = radio(
                h,
                "seq",
                "Keystrokes",
                &[("v", "V down/up with the ⌘ flag"), ("cmd", "⌘ down, V down/up, ⌘ up")],
            )?;
            let flags_menu = radio(
                h,
                "flags",
                "Event flags",
                &[
                    ("cmd-device", "⌘ + left-⌘ device bit"),
                    ("cmd", "⌘ only"),
                    ("source", "Don't set (event source decides)"),
                ],
            )?;
            let source_menu = radio(
                h,
                "source",
                "Event source",
                &[("0", "Combined session state"), ("1", "HID system state"), ("-1", "Private state")],
            )?;
            let tap_menu = radio(
                h,
                "tap",
                "Post to",
                &[
                    ("session", "Session event tap"),
                    ("hid", "HID event tap"),
                    ("pid", "The target app's pid"),
                ],
            )?;
            let toggle = |id: &str, text: &str, checked: bool| {
                CheckMenuItem::with_id(h, id, text, true, checked, None::<&str>)
            };
            let menu = Menu::with_items(
                h,
                &[
                    &MenuItem::with_id(h, "show", "Show panel (⇧⌘V)", true, None::<&str>)?,
                    &MenuItem::with_id(h, "quick", "⇧⌘1 pastes the first clip directly", false, None::<&str>)?,
                    &PredefinedMenuItem::separator(h)?,
                    &permission,
                    &MenuItem::with_id(h, "ax-prompt", "Ask for Accessibility (AX prompt)", true, None::<&str>)?,
                    &MenuItem::with_id(h, "cg-request", "Ask with CGRequestPostEventAccess", true, None::<&str>)?,
                    &MenuItem::with_id(h, "ax-settings", "Open Accessibility settings", true, None::<&str>)?,
                    &PredefinedMenuItem::separator(h)?,
                    &key_menu,
                    &delay_menu,
                    &seq_menu,
                    &flags_menu,
                    &source_menu,
                    &tap_menu,
                    &toggle("toggle:wait", "Wait for modifiers to be released", false)?,
                    &toggle("toggle:suppress", "Suppress local keys while posting", true)?,
                    &toggle("toggle:blur", "Hide on blur", true)?,
                    &PredefinedMenuItem::separator(h)?,
                    &MenuItem::with_id(h, "layouts", "Log the V key for every layout", true, None::<&str>)?,
                    &MenuItem::with_id(h, "log", "Open log", true, None::<&str>)?,
                    &MenuItem::with_id(h, "quit", "Quit", true, None::<&str>)?,
                ],
            )?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().expect("icon").clone())
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(on_menu)
                .build(app)?;

            // Watch the permission, to see whether a grant takes effect without a relaunch.
            std::thread::spawn(move || {
                let mut last = None;
                loop {
                    let now = (ax_trusted(), post_event_access());
                    if last != Some(now) {
                        if last.is_some() {
                            log(format!("permission changed: {}", permission_text()));
                        }
                        let _ = permission.set_text(permission_text());
                        last = Some(now);
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            });

            log("ready: ⇧⌘V shows the panel, ⇧⌘1 pastes the first clip");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running the spike");
}
