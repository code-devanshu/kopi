//! The menu bar icon and its menu: on Windows, the notification area icon.

use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Wry, include_image};

use crate::commands::history::{self, ClearRange};

const OPEN: &str = "open";
const SETTINGS: &str = "settings";
const ONBOARDING: &str = "onboarding";
const CHECK_UPDATES: &str = "check-updates";
const RESTART: &str = "restart";
const CLEAR_HOUR: &str = "clear-hour";
const CLEAR_DAY: &str = "clear-day";
const QUIT: &str = "quit";

/// The menu, kept so a downloaded update can add Restart to Update.
struct TrayMenu(Menu<Wry>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id(OPEN, "Open Kopi").build(app)?;
    let settings = MenuItemBuilder::with_id(SETTINGS, "Settings…")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
    let onboarding = MenuItemBuilder::with_id(ONBOARDING, "Set Up Kopi…").build(app)?;
    let check_updates = MenuItemBuilder::with_id(CHECK_UPDATES, "Check for Updates…").build(app)?;
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
        .item(&onboarding)
        .item(&check_updates)
        .separator()
        .item(&quit)
        .build()?;

    // A template image follows the menu bar's light or dark appearance on macOS. Windows has no
    // template images, and a black glyph would vanish on its dark taskbar, so it gets the app
    // icon.
    #[cfg(target_os = "macos")]
    let icon = include_image!("icons/tray.png");
    #[cfg(not(target_os = "macos"))]
    let icon = include_image!("icons/32x32.png");

    TrayIconBuilder::with_id("main")
        .icon(icon)
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
            ONBOARDING => {
                if let Err(error) = crate::onboarding::open(app) {
                    eprintln!("kopi: couldn't open onboarding: {error}");
                }
            }
            CHECK_UPDATES => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { crate::updates::check(&app, true).await });
            }
            RESTART => crate::updates::restart(app),
            CLEAR_HOUR => clear_recent(app, ClearRange::LastHour),
            CLEAR_DAY => clear_recent(app, ClearRange::LastDay),
            QUIT => {
                crate::updates::install_before_quit(app);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;
    app.manage(TrayMenu(menu));
    Ok(())
}

/// Adds Restart to Update to the top of the menu, once an update is downloaded.
pub fn show_restart(app: &AppHandle) {
    let menu = &app.state::<TrayMenu>().0;
    if menu.get(RESTART).is_some() {
        return;
    }
    let added = MenuItemBuilder::with_id(RESTART, "Restart to Update")
        .build(app)
        .and_then(|restart| {
            let separator = PredefinedMenuItem::separator(app)?;
            menu.insert_items(&[&restart, &separator], 0)
        });
    if let Err(error) = added {
        eprintln!("kopi: couldn't add Restart to Update: {error}");
    }
}

/// Shows what an update check is doing in place of Check for Updates…, greyed out so another
/// click doesn't look like it did nothing. `None` brings Check for Updates… back.
pub fn show_update_status(app: &AppHandle, status: Option<&str>) {
    let Some(menu) = app.try_state::<TrayMenu>() else {
        return;
    };
    let Some(item) = menu.0.get(CHECK_UPDATES) else {
        return;
    };
    let Some(item) = item.as_menuitem() else {
        return;
    };
    let result = item
        .set_text(status.unwrap_or("Check for Updates…"))
        .and_then(|()| item.set_enabled(status.is_none()));
    if let Err(error) = result {
        eprintln!("kopi: couldn't show the update status: {error}");
    }
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
