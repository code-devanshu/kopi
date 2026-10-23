//! Kopi's app shell: builder, plugins and setup.

mod alert;
mod capture;
mod commands;
mod events;
mod hotkeys;
mod login_item;
mod ocr;
mod onboarding;
mod panel;
mod protocol;
mod settings_window;
mod sync;
mod tray;
mod updates;

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
            commands::settings::get_settings,
            commands::settings::set_shortcut,
            commands::settings::suspend_shortcut,
            commands::settings::set_capture_kinds,
            commands::settings::set_theme,
            commands::settings::set_auto_update,
            commands::settings::image_text_status,
            commands::settings::set_image_text,
            commands::settings::open_language_settings,
            commands::settings::excluded_apps,
            commands::settings::add_excluded_app,
            commands::settings::remove_excluded_app,
            commands::settings::running_apps,
            commands::settings::permission_status,
            commands::settings::request_permission,
            commands::settings::open_permission_settings,
            commands::settings::open_settings,
            commands::onboarding::finish_onboarding,
            commands::onboarding::open_privacy_design,
            commands::paste::paste_clip,
            commands::paste::copy_clip,
            commands::paste::copy_image_text,
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
            commands::snippets::snippet_search,
            commands::snippets::snippet_keyword,
            commands::snippets::snippet_get,
            commands::snippets::snippet_create,
            commands::snippets::snippet_update,
            commands::snippets::snippet_reorder,
            commands::snippets::snippet_delete,
            commands::paste::paste_snippet,
            commands::paste::copy_snippet,
            commands::paste::clipboard_text,
            commands::sync::sync_status,
            commands::sync::set_sync_enabled,
            commands::sync::set_sync_device_name,
            commands::sync::set_sync_kinds,
            commands::sync::set_sync_snippets,
            commands::sync::remove_sync_device,
            commands::sync::start_pairing,
            commands::sync::stop_pairing,
            commands::sync::pair_with,
            commands::sync::open_local_network_settings,
        ])
        .events(tauri_specta::collect_events![
            events::ClipAdded,
            events::ClipUpdated,
            events::ClipsRemoved,
            events::PinboardsChanged,
            events::SnippetsChanged,
            events::ThumbnailReady,
            events::ImageTextRead,
            events::PanelShown,
            events::PanelHidden,
            events::ThemeChanged,
            events::SyncChanged,
            events::SyncPaired,
            events::SyncPairingFailed,
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
        // Launch at login on Windows. On macOS it only finds the LaunchAgent of earlier
        // versions, see login_item.rs. Stays off until the user turns it on.
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build());
    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());
    // macOS shows Kopi's own alerts, see alert.rs.
    #[cfg(not(target_os = "macos"))]
    let builder = builder.plugin(tauri_plugin_dialog::init());

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

            // Before capture and sync, which add the images it reads.
            ocr::start(app.handle(), &store)?;

            // No Dock icon or app switcher entry. Info.plist sets LSUIElement for bundled
            // builds; this covers `tauri dev`.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            login_item::migrate(app.handle());
            commands::settings::apply_theme(app.handle());
            panel::create(app.handle())?;
            commands::paste::manage_engine(app.handle());
            hotkeys::register(app.handle(), &store);
            capture::start(app.handle(), Arc::clone(&store))?;
            // Only if the user turned it on. Otherwise no socket is opened.
            sync::start_if_enabled(app.handle(), &store);
            tray::create(app.handle())?;
            updates::start(app.handle());
            onboarding::open_on_first_run(app.handle(), &store);
            Ok(())
        })
        .on_window_event(|window, event| {
            let label = window.label();
            if label == settings_window::SETTINGS || label == onboarding::ONBOARDING {
                match event {
                    // Closing onboarding counts as finishing it.
                    WindowEvent::CloseRequested { .. } if label == onboarding::ONBOARDING => {
                        onboarding::mark_done(window.app_handle());
                    }
                    // Closed while recording a shortcut. Off the main thread: the shortcut
                    // plugin waits for it.
                    WindowEvent::Destroyed => {
                        let app = window.app_handle().clone();
                        tauri::async_runtime::spawn_blocking(move || hotkeys::suspend(&app, false));
                    }
                    _ => {}
                }
                return;
            }
            if label != panel::PANEL {
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
