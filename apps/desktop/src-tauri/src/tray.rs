//! The menu bar icon and its menu.

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, include_image};

use crate::commands::history::{self, ClearRange};

const OPEN: &str = "open";
const SETTINGS: &str = "settings";
const CLEAR_HOUR: &str = "clear-hour";
const CLEAR_DAY: &str = "clear-day";
const QUIT: &str = "quit";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id(OPEN, "Open Kopi").build(app)?;
    let settings = MenuItemBuilder::with_id(SETTINGS, "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let quit = MenuItemBuilder::with_id(QUIT, "Quit Kopi")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    // Clearing everything needs a confirmation, so it's in Settings.
    let clear = SubmenuBuilder::new(app, "Clear History")
        .text(CLEAR_HOUR, "Last Hour")
        .text(CLEAR_DAY, "Last Day")
        .separator()
        .item(
            &MenuItemBuilder::new("Pinned clips and pinboards stay")
                .enabled(false)
                .build(app)?,
        )
        .build()?;
    let menu = MenuBuilder::new(app)
        .item(&open)
        .item(&clear)
        .separator()
        .item(&settings)
        .separator()
        .item(&quit)
        .build()?;

    TrayIconBuilder::with_id("main")
        .icon(include_image!("icons/tray.png"))
        // A template image follows the menu bar's light or dark appearance on macOS.
        .icon_as_template(true)
        .tooltip("Kopi")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            OPEN => crate::panel::show(app),
            SETTINGS => {
                if let Err(error) = crate::settings_window::open(app) {
                    eprintln!("kopi: couldn't open Settings: {error}");
                }
            }
            CLEAR_HOUR => clear_recent(app, ClearRange::LastHour),
            CLEAR_DAY => clear_recent(app, ClearRange::LastDay),
            QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// Clears recent history off the main thread, keeping pinned clips and pinboards.
fn clear_recent(app: &AppHandle, range: ClearRange) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Err(error) = history::clear(&app, range, true) {
            eprintln!("kopi: couldn't clear history: {error}");
        }
    });
}
