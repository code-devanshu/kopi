//! Kopi's app shell: builder, plugins and setup.

mod capture;
mod commands;
mod events;
mod hotkeys;
mod panel;
mod protocol;
mod tray;

use std::sync::Arc;

use kopi_core::retention::{self, Retention};
use kopi_core::storage::Store;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_specta::Event;

/// The typed IPC surface. `bindings.ts` is generated from this.
pub fn specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .commands(tauri_specta::collect_commands![
            commands::app_info,
            commands::launch_at_login,
            commands::set_launch_at_login,
            commands::hide_panel,
            commands::search::search_clips,
            commands::clips::get_clip,
            commands::clips::delete_clips,
            commands::clips::source_apps,
            commands::history::history_limits,
            commands::history::set_history_limits,
            commands::history::clear_history,
            commands::paste::paste_clip,
            commands::paste::copy_clip,
            commands::pinboards::set_pinned,
            commands::pinboards::pinboard_list,
            commands::pinboards::pinboard_create,
            commands::pinboards::pinboard_rename,
            commands::pinboards::pinboard_recolor,
            commands::pinboards::pinboard_reorder,
            commands::pinboards::pinboard_delete,
            commands::pinboards::pinboard_add_clips,
            commands::pinboards::pinboard_remove_clips,
            commands::pinboards::clip_pinboards,
        ])
        .events(tauri_specta::collect_events![
            events::ClipAdded,
            events::ClipUpdated,
            events::ClipsRemoved,
            events::PinboardsChanged,
            events::ThumbnailReady,
            events::PanelShown,
            events::PanelHidden,
        ])
}

pub fn run() {
    let specta = specta_builder();

    // Dev builds keep the UI's bindings in step with the commands.
    #[cfg(debug_assertions)]
    export_bindings(&specta);

    let builder = tauri::Builder::default()
        // Must be registered first. A second launch shows the running instance instead.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            panel::show(app);
        }))
        // Only registers the plugin. Launch at login stays off until the user turns it on.
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .invoke_handler(specta.invoke_handler())
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, protocol::handle)
        .setup(move |app| {
            specta.mount_events(app);

            // ~/Library/Application Support/in.devanshuverma.kopi on macOS,
            // %LOCALAPPDATA%\in.devanshuverma.kopi on Windows.
            let handle = app.handle().clone();
            let store = Store::open_with_listener(
                app.path().app_local_data_dir()?,
                Box::new(move |done| {
                    if done.result.is_ok() {
                        let _ = events::ThumbnailReady {
                            content_hash: done.content_hash,
                        }
                        .emit(&handle);
                    }
                }),
            )?;
            let store = Arc::new(store);
            app.manage(Arc::clone(&store));

            // Sweeps orphan files and applies the history limits now, then again as clips
            // arrive. Before capture starts, which tells it about new clips.
            let handle = app.handle().clone();
            let retention = Retention::spawn(
                Arc::clone(&store),
                retention::INTERVAL,
                Box::new(move |result| match result {
                    Ok(ids) if ids.is_empty() => {}
                    Ok(ids) => {
                        let _ = events::ClipsRemoved { ids }.emit(&handle);
                    }
                    // Storage errors name the database or a file, never clip content.
                    Err(error) => eprintln!("kopi: retention failed: {error}"),
                }),
            )?;
            app.manage(retention);

            // No Dock icon or app switcher entry. Info.plist sets LSUIElement for bundled
            // builds; this covers `tauri dev`.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            panel::create(app.handle())?;
            commands::paste::manage_engine(app.handle());
            hotkeys::register(app.handle());
            capture::start(app.handle(), store)?;
            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != panel::PANEL {
                return;
            }
            match event {
                WindowEvent::Focused(focused) => {
                    panel::focus_changed(window.app_handle(), *focused)
                }
                // Closing hides it. Kopi keeps running in the menu bar until Quit.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    panel::hide(window.app_handle());
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Kopi");
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
