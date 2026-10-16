//! Launch at login. On macOS Kopi.app registers itself as a login item through
//! `SMAppService.mainApp`, so macOS opens it like any other app at login and lists it in System
//! Settings → General → Login Items. Elsewhere the autostart plugin does it.
//!
//! Up to 0.1.0-alpha.4 macOS used the plugin's LaunchAgent, which starts the binary early in
//! login. Kopi then sometimes couldn't post ⌘V: pasting did nothing, with Accessibility shown as
//! allowed, until Kopi was restarted. [`migrate`] moves those users to the login item.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;

/// Whether Kopi starts at login.
pub fn is_enabled(app: &AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        Ok(mac::is_enabled())
    }
    #[cfg(not(target_os = "macos"))]
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Turns launch at login on or off.
pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let _ = app;
        mac::set_enabled(enabled)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let autostart = app.autolaunch();
        let result = if enabled {
            autostart.enable()
        } else {
            autostart.disable()
        };
        result.map_err(|e| e.to_string())
    }
}

/// Replaces the LaunchAgent of earlier versions with the login item. The plist is removed only
/// once the login item is registered, so a failure leaves launch at login as it was.
pub fn migrate(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    {
        let agent = app.autolaunch();
        if !agent.is_enabled().unwrap_or(false) {
            return;
        }
        if let Err(error) = mac::set_enabled(true) {
            eprintln!("kopi: couldn't move launch at login to a login item: {error}");
            return;
        }
        if let Err(error) = agent.disable() {
            eprintln!("kopi: couldn't remove the old launch agent: {error}");
        }
    }
    #[cfg(not(target_os = "macos"))]
    let _ = app;
}

#[cfg(target_os = "macos")]
mod mac {
    use objc2::rc::Retained;
    use objc2_service_management::{SMAppService, SMAppServiceStatus};

    fn service() -> Retained<SMAppService> {
        // SAFETY: a class method with no arguments.
        unsafe { SMAppService::mainAppService() }
    }

    pub fn is_enabled() -> bool {
        // SAFETY: reads the status of a valid service object.
        unsafe { service().status() == SMAppServiceStatus::Enabled }
    }

    pub fn set_enabled(enabled: bool) -> Result<(), String> {
        let service = service();
        // SAFETY: calls on a valid service object. They take no arguments besides the error.
        unsafe {
            let result = if enabled {
                service.registerAndReturnError()
            } else if service.status() == SMAppServiceStatus::NotRegistered {
                // Unregistering a service that isn't registered is an error.
                return Ok(());
            } else {
                service.unregisterAndReturnError()
            };
            result.map_err(|error| error.localizedDescription().to_string())?;
            // The user switched Kopi off under Login Items, and only they can switch it back on.
            if enabled && service.status() == SMAppServiceStatus::RequiresApproval {
                SMAppService::openSystemSettingsLoginItems();
                return Err("Turn on Kopi in System Settings → General → Login Items.".to_owned());
            }
        }
        Ok(())
    }
}
