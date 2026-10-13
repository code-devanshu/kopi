//! Message boxes, for updates.
//!
//! On macOS they're Kopi's own `NSAlert`, on the screen with the pointer. The dialog plugin's
//! alert without a parent window is a system alert instead: it opens on the main display, which
//! may not be the one the user is looking at, and another one queues behind it on every click.

use tauri::AppHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Info,
    Error,
}

/// Shows a message with an OK button.
pub fn message(app: &AppHandle, kind: Kind, title: &str, text: &str) {
    platform::show(app, kind, title.into(), text.into(), None, |_| {});
}

/// Asks with two buttons. `answer` gets whether the user chose `yes`.
pub fn ask(
    app: &AppHandle,
    title: &str,
    text: &str,
    yes: &str,
    no: &str,
    answer: impl FnOnce(bool) + Send + 'static,
) {
    platform::show(
        app,
        Kind::Info,
        title.into(),
        text.into(),
        Some((yes.into(), no.into())),
        answer,
    );
}

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::Cell;

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{
        NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSApplication, NSModalPanelRunLoopMode,
    };
    use objc2_foundation::{NSArray, NSRunLoop, NSString};
    use tauri::AppHandle;

    use super::Kind;

    pub fn show(
        app: &AppHandle,
        kind: Kind,
        title: String,
        text: String,
        buttons: Option<(String, String)>,
        answer: impl FnOnce(bool) + Send + 'static,
    ) {
        let _ = app.run_on_main_thread(move || {
            let Some(mtm) = MainThreadMarker::new() else {
                return;
            };
            // Not inside this callback: the event loop holds back Kopi's other events, such as
            // the shortcut, until it returns, and an alert waits for the user.
            let job = Cell::new(Some(move || run(mtm, kind, &title, &text, buttons, answer)));
            let block = RcBlock::new(move || {
                if let Some(job) = job.take() {
                    job();
                }
            });
            // SAFETY: the main run loop runs the block on this thread.
            unsafe { NSRunLoop::mainRunLoop().performBlock(&block) };
        });
    }

    fn run(
        mtm: MainThreadMarker,
        kind: Kind,
        title: &str,
        text: &str,
        buttons: Option<(String, String)>,
        answer: impl FnOnce(bool),
    ) {
        let app = NSApplication::sharedApplication(mtm);
        // One at a time. Another click brings the open one forward.
        if let Some(window) = app.modalWindow() {
            app.activate();
            window.makeKeyAndOrderFront(None);
            return;
        }

        let alert = NSAlert::new(mtm);
        alert.setAlertStyle(match kind {
            Kind::Info => NSAlertStyle::Informational,
            Kind::Error => NSAlertStyle::Critical,
        });
        alert.setMessageText(&NSString::from_str(title));
        alert.setInformativeText(&NSString::from_str(text));
        if let Some((yes, no)) = buttons {
            alert.addButtonWithTitle(&NSString::from_str(&yes));
            // Only a button titled Cancel gets Escape by itself.
            alert
                .addButtonWithTitle(&NSString::from_str(&no))
                .setKeyEquivalent(&NSString::from_str("\u{1b}"));
        }
        alert.layout();

        // runModal centers the alert on the main display. It starts hidden and moves to the
        // pointer's screen as soon as it's up.
        let window = alert.window();
        window.setAlphaValue(0.0);
        let shown = RcBlock::new(move || {
            crate::panel::place(mtm, &window, 0.75);
            window.setAlphaValue(1.0);
        });
        // SAFETY: as above. The block runs once the alert's modal loop does.
        unsafe {
            let modes = NSArray::from_slice(&[NSModalPanelRunLoopMode]);
            NSRunLoop::mainRunLoop().performInModes_block(&modes, &shown);
        }

        // Kopi has no Dock icon, so the alert would open behind the app in front without this.
        app.activate();
        answer(alert.runModal() == NSAlertFirstButtonReturn);
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use tauri::AppHandle;
    use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

    use super::Kind;

    pub fn show(
        app: &AppHandle,
        kind: Kind,
        title: String,
        text: String,
        buttons: Option<(String, String)>,
        answer: impl FnOnce(bool) + Send + 'static,
    ) {
        let dialog = app.dialog().message(text).title(title).kind(match kind {
            Kind::Info => MessageDialogKind::Info,
            Kind::Error => MessageDialogKind::Error,
        });
        let dialog = match buttons {
            Some((yes, no)) => dialog.buttons(MessageDialogButtons::OkCancelCustom(yes, no)),
            None => dialog,
        };
        dialog.show(answer);
    }
}
