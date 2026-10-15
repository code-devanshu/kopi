//! The global shortcut that opens the panel: ⇧⌘V on macOS and Ctrl+Shift+V on Windows, unless
//! the user picked another in Settings.

use std::sync::{Mutex, PoisonError};

use kopi_core::storage::Store;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::panel;

/// The default shortcut, as the plugin writes it.
#[cfg(target_os = "macos")]
pub const DEFAULT: &str = "shift+super+KeyV";
#[cfg(not(target_os = "macos"))]
pub const DEFAULT: &str = "shift+control+KeyV";

/// The OS, as error messages name it.
const OS: &str = if cfg!(target_os = "windows") {
    "Windows"
} else {
    "macOS"
};

/// The shortcut in use, and whether it's registered. Settings suspends it while recording a
/// new one, so pressing the current one there doesn't open the panel.
struct Current {
    shortcut: Shortcut,
    registered: bool,
}

struct State(Mutex<Current>);

/// Registers the saved shortcut, or the default if none is saved or it can't be read. If
/// another app holds it, Kopi still runs, and the menu bar icon opens the panel.
pub fn register(app: &AppHandle, store: &Store) {
    let saved = store.shortcut().ok().flatten();
    let shortcut = saved
        .and_then(|text| parse(&text).ok())
        .unwrap_or_else(default);
    let registered = match bind(app, shortcut) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("kopi: couldn't register the shortcut: {error}");
            false
        }
    };
    app.manage(State(Mutex::new(Current {
        shortcut,
        registered,
    })));
}

/// The shortcut in use, as the plugin writes it.
pub fn current(app: &AppHandle) -> String {
    lock(app, |current| current.shortcut.into_string())
}

/// Switches to `text` and saves it. The old shortcut stays if the new one can't be registered.
/// Returns the new shortcut as the plugin writes it.
pub fn change(app: &AppHandle, store: &Store, text: &str) -> Result<String, String> {
    let shortcut = parse(text)?;
    lock(app, |current| {
        if current.shortcut == shortcut && current.registered {
            return Ok(());
        }
        let global = app.global_shortcut();
        if current.registered {
            let _ = global.unregister(current.shortcut);
            current.registered = false;
        }
        if let Err(error) = bind(app, shortcut) {
            // Put the old one back.
            current.registered = bind(app, current.shortcut).is_ok();
            return Err(format!(
                "{} is taken by {OS} or another app ({error})",
                shortcut.into_string()
            ));
        }
        current.shortcut = shortcut;
        current.registered = true;
        Ok(())
    })?;
    let text = shortcut.into_string();
    store.set_shortcut(&text).map_err(|e| e.to_string())?;
    Ok(text)
}

/// Unregisters the shortcut while `suspended`, and registers it again after.
pub fn suspend(app: &AppHandle, suspended: bool) {
    lock(app, |current| {
        if suspended && current.registered {
            let _ = app.global_shortcut().unregister(current.shortcut);
            current.registered = false;
        } else if !suspended && !current.registered {
            current.registered = bind(app, current.shortcut).is_ok();
        }
    });
}

fn bind(app: &AppHandle, shortcut: Shortcut) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            if event.state == ShortcutState::Pressed {
                panel::toggle(app);
            }
        })
        .map_err(|e| e.to_string())
}

/// Reads a shortcut. It must hold ⌘, ⌃ or ⌥, or every press of the key would open the panel.
fn parse(text: &str) -> Result<Shortcut, String> {
    let shortcut: Shortcut = text
        .parse()
        .map_err(|_| format!("{text:?} isn't a shortcut"))?;
    let needed = Modifiers::SUPER | Modifiers::CONTROL | Modifiers::ALT;
    if !shortcut.mods.intersects(needed) {
        return Err(if cfg!(target_os = "windows") {
            "A shortcut needs Ctrl, Alt or Win".to_owned()
        } else {
            "A shortcut needs ⌘, ⌃ or ⌥".to_owned()
        });
    }
    Ok(shortcut)
}

fn default() -> Shortcut {
    parse(DEFAULT).expect("the default shortcut parses")
}

fn lock<T>(app: &AppHandle, f: impl FnOnce(&mut Current) -> T) -> T {
    let state = app.state::<State>();
    let mut current = state.0.lock().unwrap_or_else(PoisonError::into_inner);
    f(&mut current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_what_it_writes() {
        let shortcut = parse("Shift+Super+KeyV").unwrap();
        assert_eq!(shortcut.into_string(), "shift+super+KeyV");
        assert_eq!(parse(&shortcut.into_string()).unwrap(), shortcut);
        assert_eq!(default().into_string(), DEFAULT);
    }

    #[test]
    fn needs_a_modifier_other_than_shift() {
        assert!(parse("KeyV").is_err());
        assert!(parse("shift+KeyV").is_err());
        assert!(parse("alt+KeyV").is_ok());
        assert!(parse("control+shift+Space").is_ok());
        assert!(parse("super+").is_err());
        assert!(parse("super+Nonsense").is_err());
    }
}
