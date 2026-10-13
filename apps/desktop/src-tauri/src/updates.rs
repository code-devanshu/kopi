//! Updates, through the Tauri updater. Kopi checks a little after launch and then every few
//! hours, unless the user turned that off, and when they choose Check for Updates… in the menu
//! bar. An update downloads, is checked against the updater key and replaces Kopi.app. It runs
//! the next time Kopi starts, and the menu bar offers to restart now. See docs/RELEASING.md.

use std::cmp::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use kopi_core::storage::Store;
use semver::Version;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

/// How long after launch the first check waits, to stay out of the way of startup.
const FIRST_CHECK: Duration = Duration::from_secs(30);
/// How often Kopi checks while it runs.
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
/// How often the schedule wakes up. A sleeping Mac doesn't count down a thread's sleep, so it
/// wakes often and compares the clock instead.
const TICK: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
    Idle,
    /// A check, download or install is running. `manual` once the user has asked, so its result
    /// is shown even if an automatic check started it.
    Busy {
        manual: bool,
    },
    /// An update is installed and runs at the next launch. Holds its name.
    Installed(String),
}

struct Updates(Mutex<State>);

/// Starts the automatic checks. Development builds don't check by themselves.
pub fn start(app: &AppHandle) {
    app.manage(Updates(Mutex::new(State::Idle)));
    if cfg!(debug_assertions) {
        return;
    }
    let app = app.clone();
    let spawned = std::thread::Builder::new()
        .name("kopi-updates".into())
        .spawn(move || {
            std::thread::sleep(FIRST_CHECK);
            let mut last: Option<SystemTime> = None;
            loop {
                // A clock that went backwards counts as due.
                let due = last.is_none_or(|t| t.elapsed().map_or(true, |e| e >= CHECK_EVERY));
                if due && automatic(&app) {
                    tauri::async_runtime::block_on(check(&app, false));
                    last = Some(SystemTime::now());
                }
                std::thread::sleep(TICK);
            }
        });
    if let Err(error) = spawned {
        eprintln!("kopi: couldn't schedule update checks: {error}");
    }
}

/// Whether the user lets Kopi check by itself. An unreadable setting reads as on.
fn automatic(app: &AppHandle) -> bool {
    app.state::<Arc<Store>>().auto_update().unwrap_or(true)
}

/// Checks for an update and installs it. A manual check shows what it found. Safe to call
/// while another check runs: a manual one then shows that check's result.
pub async fn check(app: &AppHandle, manual: bool) {
    {
        let mut state = lock(app);
        match &*state {
            State::Busy { .. } => {
                if manual {
                    *state = State::Busy { manual: true };
                }
                return;
            }
            State::Installed(name) => {
                if manual {
                    offer_restart(app, name);
                }
                return;
            }
            State::Idle => *state = State::Busy { manual },
        }
    }

    let result = install_newer(app).await;

    let manual = {
        let mut state = lock(app);
        let manual = matches!(*state, State::Busy { manual: true });
        *state = match &result {
            Ok(Some(name)) => State::Installed(name.clone()),
            _ => State::Idle,
        };
        manual
    };
    match result {
        Ok(Some(name)) => {
            crate::tray::show_restart(app);
            if manual {
                offer_restart(app, &name);
            }
        }
        Ok(None) if manual => message(
            app,
            MessageDialogKind::Info,
            "Kopi is up to date",
            format!("You have the latest version, Kopi {}.", current_name(app)),
        ),
        Ok(None) => {}
        // Updater errors name a URL or a file, never clip content.
        Err(error) => {
            eprintln!("kopi: update failed: {error}");
            if manual {
                message(
                    app,
                    MessageDialogKind::Error,
                    "Couldn't update Kopi",
                    format!("{error}\n\nCheck your connection and try again."),
                );
            }
        }
    }
}

/// Restarts into the installed update.
pub fn restart(app: &AppHandle) {
    // Through the event loop, so the single-instance lock is released before the new Kopi
    // starts. Otherwise the new one would find it and quit.
    app.request_restart();
}

/// Finds a newer release and installs it, returning its name.
async fn install_newer(app: &AppHandle) -> Result<Option<String>, tauri_plugin_updater::Error> {
    // Every alpha build of a version has that version, so the updater can't compare them
    // itself. Kopi compares versions, then build numbers.
    let updater = app
        .updater_builder()
        .version_comparator(|_, _| true)
        .build()?;
    let Some(update) = updater.check().await? else {
        return Ok(None);
    };
    let (Ok(current), Ok(offered)) = (
        Version::parse(&update.current_version),
        Version::parse(&update.version),
    ) else {
        return Ok(None);
    };
    let offered_build = update.raw_json.get("build").and_then(|b| b.as_u64());
    if !is_newer(&current, build(app), &offered, offered_build) {
        return Ok(None);
    }
    update.download_and_install(|_, _| {}, || {}).await?;
    let name = update
        .raw_json
        .get("release")
        .and_then(|n| n.as_str())
        .unwrap_or(&update.version);
    Ok(Some(name.to_owned()))
}

/// Whether the offered release is newer: a greater version, or the same version with a
/// greater build number. A build without a number, such as a local one, is older than any
/// release of its version.
fn is_newer(
    current: &Version,
    build: Option<u64>,
    offered: &Version,
    offered_build: Option<u64>,
) -> bool {
    match offered.cmp_precedence(current) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match (offered_build, build) {
            (Some(offered), Some(build)) => offered > build,
            (Some(_), None) => true,
            (None, _) => false,
        },
    }
}

/// This build's number, `CFBundleVersion`. The Release workflow sets it to its run number.
fn build(app: &AppHandle) -> Option<u64> {
    app.config()
        .bundle
        .macos
        .bundle_version
        .as_deref()
        .and_then(|b| b.parse().ok())
}

/// The running version and its build, as "0.1.0 (57)".
fn current_name(app: &AppHandle) -> String {
    let version = app.package_info().version.to_string();
    match build(app) {
        Some(build) => format!("{version} ({build})"),
        None => version,
    }
}

fn offer_restart(app: &AppHandle, name: &str) {
    let app = app.clone();
    bring_forward(&app);
    app.dialog()
        .message(format!(
            "Kopi {name} is installed. Restart Kopi now to use it, or it starts the next time you open Kopi."
        ))
        .title("Kopi is ready to update")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Restart Now".into(),
            "Later".into(),
        ))
        .show(move |now| {
            if now {
                restart(&app);
            }
        });
}

fn message(app: &AppHandle, kind: MessageDialogKind, title: &str, text: String) {
    bring_forward(app);
    app.dialog()
        .message(text)
        .title(title)
        .kind(kind)
        .show(|_| {});
}

/// Kopi has no Dock icon, so a dialog would open behind the app in front without this.
fn bring_forward(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.run_on_main_thread(|| {
        use objc2::MainThreadMarker;
        use objc2_app_kit::NSApplication;

        if let Some(mtm) = MainThreadMarker::new() {
            NSApplication::sharedApplication(mtm).activate();
        }
    });
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

fn lock(app: &AppHandle) -> std::sync::MutexGuard<'_, State> {
    // The state is always valid, even after a panic elsewhere.
    app.state::<Updates>()
        .inner()
        .0
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn a_greater_version_is_newer_whatever_the_builds() {
        assert!(is_newer(&v("0.1.0"), Some(90), &v("0.2.0"), Some(80)));
        assert!(is_newer(&v("0.1.0"), Some(90), &v("0.1.1"), None));
        assert!(!is_newer(&v("0.2.0"), Some(80), &v("0.1.0"), Some(90)));
    }

    #[test]
    fn the_same_version_compares_builds() {
        assert!(is_newer(&v("0.1.0"), Some(57), &v("0.1.0"), Some(58)));
        assert!(!is_newer(&v("0.1.0"), Some(58), &v("0.1.0"), Some(58)));
        assert!(!is_newer(&v("0.1.0"), Some(58), &v("0.1.0"), Some(57)));
    }

    #[test]
    fn a_build_without_a_number_is_older_than_a_release() {
        assert!(is_newer(&v("0.1.0"), None, &v("0.1.0"), Some(1)));
        assert!(!is_newer(&v("0.1.0"), None, &v("0.1.0"), None));
        assert!(!is_newer(&v("0.1.0"), Some(57), &v("0.1.0"), None));
    }
}
