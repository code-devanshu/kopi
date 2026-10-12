//! The menu bar icon and its menu.

use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, include_image};

const OPEN: &str = "open";
const QUIT: &str = "quit";

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id(OPEN, "Open Kopi").build(app)?;
    let quit = MenuItemBuilder::with_id(QUIT, "Quit Kopi")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&open)
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
            OPEN => crate::show_main_window(app),
            QUIT => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
