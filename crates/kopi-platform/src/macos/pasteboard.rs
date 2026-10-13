//! `NSPasteboard` behind the [`Pasteboard`] trait.

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::sel;
use objc2_app_kit::{NSPasteboard, NSPasteboardAccessBehavior, NSPasteboardItem};
use objc2_foundation::{NSArray, NSString};

use super::on_main;
use crate::pasteboard::{
    Pasteboard, PasteboardAccess, PasteboardError, PasteboardTypes, RawFormat,
};

/// A macOS pasteboard. Every call runs on the main thread (see [`on_main`]).
#[derive(Debug, Clone)]
pub struct MacPasteboard {
    /// `None` for the general pasteboard, the one ⌘C and ⌘V use.
    name: Option<String>,
}

impl MacPasteboard {
    /// The general pasteboard.
    pub fn general() -> Self {
        Self { name: None }
    }

    /// A named pasteboard. Tests use a private one so they don't touch the user's clipboard.
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
        }
    }

    fn run<R, F>(&self, f: F) -> Result<R, PasteboardError>
    where
        F: FnOnce(&NSPasteboard) -> Result<R, PasteboardError> + Send + 'static,
        R: Send + 'static,
    {
        let name = self.name.clone();
        on_main(move || {
            let pasteboard = match &name {
                None => NSPasteboard::generalPasteboard(),
                Some(name) => NSPasteboard::pasteboardWithName(&NSString::from_str(name)),
            };
            f(&pasteboard)
        })
        .ok_or(PasteboardError::MainThreadBusy)?
    }
}

impl Pasteboard for MacPasteboard {
    fn change_count(&self) -> Result<u64, PasteboardError> {
        self.run(|pb| Ok(change_count(pb)))
    }

    fn types(&self) -> Result<PasteboardTypes, PasteboardError> {
        self.run(|pb| {
            Ok(PasteboardTypes {
                change_count: change_count(pb),
                items: items(pb).iter().map(|item| item_types(&item)).collect(),
            })
        })
    }

    fn read(
        &self,
        expected: u64,
        formats: &[Vec<String>],
        max_bytes: u64,
    ) -> Result<Vec<RawFormat>, PasteboardError> {
        let formats = formats.to_vec();
        self.run(move |pb| {
            let access = access(pb);
            if !access.allows_background_reads() {
                return Err(PasteboardError::AccessDenied(access));
            }
            if change_count(pb) != expected {
                return Err(PasteboardError::Changed);
            }
            // Items belong to the copy they were fetched for. If another copy lands while we
            // read, their data comes back empty instead of mixing in the new copy.
            let mut out = Vec::new();
            let mut total: u64 = 0;
            for (index, (item, wanted)) in items(pb).iter().zip(&formats).enumerate() {
                let has = item_types(&item);
                for format in wanted.iter().filter(|f| has.contains(f)) {
                    let Some(data) = item.dataForType(&NSString::from_str(format)) else {
                        continue;
                    };
                    total = total.saturating_add(data.len() as u64);
                    if total > max_bytes {
                        return Err(PasteboardError::TooLarge);
                    }
                    out.push(RawFormat {
                        item_index: index as u32,
                        format: format.to_string(),
                        data: data.to_vec(),
                    });
                }
            }
            if change_count(pb) != expected {
                return Err(PasteboardError::Changed);
            }
            Ok(out)
        })
    }

    fn access(&self) -> Result<PasteboardAccess, PasteboardError> {
        self.run(|pb| Ok(access(pb)))
    }
}

fn change_count(pb: &NSPasteboard) -> u64 {
    u64::try_from(pb.changeCount()).unwrap_or(0)
}

fn items(pb: &NSPasteboard) -> Retained<NSArray<NSPasteboardItem>> {
    pb.pasteboardItems().unwrap_or_default()
}

fn item_types(item: &NSPasteboardItem) -> Vec<String> {
    item.types().iter().map(|t| t.to_string()).collect()
}

fn access(pb: &NSPasteboard) -> PasteboardAccess {
    // Added in macOS 15.4. Kopi supports 14, where there's no permission to ask for.
    if !pb.respondsToSelector(sel!(accessBehavior)) {
        return PasteboardAccess::AlwaysAllow;
    }
    match pb.accessBehavior() {
        NSPasteboardAccessBehavior::Default => PasteboardAccess::Default,
        NSPasteboardAccessBehavior::AlwaysAllow => PasteboardAccess::AlwaysAllow,
        NSPasteboardAccessBehavior::AlwaysDeny => PasteboardAccess::AlwaysDeny,
        // `Ask`, and anything a later macOS adds. Not reading is the safe guess.
        _ => PasteboardAccess::Ask,
    }
}
