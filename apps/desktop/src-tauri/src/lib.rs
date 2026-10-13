//! Kopi's app shell: builder, plugins and setup.

mod commands;
mod tray;

use std::sync::Arc;

use kopi_core::storage::Store;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

/// The main window's label, as set in `tauri.conf.json`.
pub(crate) const MAIN_WINDOW: &str = "main";

/// The typed IPC surface. `bindings.ts` is generated from this.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        commands::app_info,
        commands::launch_at_login,
        commands::set_launch_at_login,
        commands::search::search_clips,
    ])
}

pub fn run() {
    let specta = specta_builder();

    // Dev builds keep the UI's bindings in step with the commands.
    #[cfg(debug_assertions)]
    export_bindings(&specta);

    tauri::Builder::default()
        // Must be registered first. A second launch shows the running instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        // Only registers the plugin. Launch at login stays off until the user turns it on.
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            specta.mount_events(app);

            // ~/Library/Application Support/in.devanshuverma.kopi on macOS,
            // %LOCALAPPDATA%\in.devanshuverma.kopi on Windows.
            let store = Store::open(app.path().app_local_data_dir()?)?;
            app.manage(Arc::new(store));

            // No Dock icon or app switcher entry. Info.plist sets LSUIElement for bundled
            // builds; this covers `tauri dev`.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides it. Kopi keeps running in the menu bar until Quit.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Kopi");
}

pub(crate) fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Where the generated TypeScript bindings live, relative to this crate.
#[cfg(any(test, debug_assertions))]
const BINDINGS_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../src/lib/bindings.ts");

#[cfg(any(test, debug_assertions))]
fn export_bindings(specta: &tauri_specta::Builder<tauri::Wry>) {
    specta
        .export(specta_typescript::Typescript::default(), BINDINGS_PATH)
        .expect("failed to export TypeScript bindings");
}

#[cfg(test)]
mod tests {
    /// Regenerates `bindings.ts`. CI runs this and fails if the file changed.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::specta_builder());
    }
}
