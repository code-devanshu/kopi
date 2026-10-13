//! The global shortcut that opens the panel: ⇧⌘V on macOS. Settings (#16) makes it changeable.

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::panel;

#[cfg(target_os = "macos")]
const MODIFIERS: Modifiers = Modifiers::SUPER.union(Modifiers::SHIFT);
#[cfg(not(target_os = "macos"))]
const MODIFIERS: Modifiers = Modifiers::CONTROL.union(Modifiers::SHIFT);

/// Registers the shortcut. If another app holds it, Kopi still runs, and the menu bar icon
/// opens the panel.
pub fn register(app: &AppHandle) {
    let shortcut = Shortcut::new(Some(MODIFIERS), Code::KeyV);
    let registered = app
        .global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if event.state == ShortcutState::Pressed {
                panel::toggle(app);
            }
        });
    if let Err(error) = registered {
        eprintln!("kopi: couldn't register the shortcut: {error}");
    }
}
