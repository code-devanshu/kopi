//! Watches the clipboard on its own thread and reports each new copy.
//!
//! Every change goes through the same steps, and content is read only at the end:
//!
//! 1. Poll the change count: every 250 ms, or every 500 ms once the user has been idle for a
//!    minute.
//! 2. Read the format names of every item.
//! 3. Check [`PasteboardAccess`]. Without a standing grant, stop here. A read would raise the
//!    system alert, and on macOS it blocks the main thread until the alert is answered.
//! 4. Find the source app: the `org.nspasteboard.source` marker if the writer set one,
//!    otherwise the frontmost app. A marker holding Kopi's own bundle ID means Kopi pasted,
//!    which is reported as [`WatchEvent::OwnWrite`] and not captured.
//! 5. Ask the [`ReadFilter`], which sees only the format names and the source app.
//! 6. Read the formats in [`uti::CAPTURE`] and nothing else, up to the size limit.
//!
//! See `docs/ARCHITECTURE.md` and `docs/decisions/0003-clipboard-watching-approach.md`.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use kopi_platform::{
    AppRef, ForegroundApp, Pasteboard, PasteboardAccess, PasteboardError, PasteboardTypes,
    RawFormat, UserIdle, uti,
};

use crate::privacy::DEFAULT_MAX_CLIP_BYTES;

/// How often the change count is polled while the user is active.
pub const ACTIVE_POLL: Duration = Duration::from_millis(250);
/// How often it's polled once the user has been idle for [`IDLE_AFTER`].
pub const IDLE_POLL: Duration = Duration::from_millis(500);
/// How long without input before polling slows down.
pub const IDLE_AFTER: Duration = Duration::from_secs(60);

/// How many polls in a row may find a changed but empty pasteboard before the change is
/// dropped. An app clears the pasteboard and then writes, and a poll can land in between.
const EMPTY_RETRIES: u32 = 4;
/// The most a source marker may hold. It's a bundle ID.
const MAX_SOURCE_BYTES: u64 = 1024;

/// What the [`ReadFilter`] sees. None of it is clipboard content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observed {
    pub types: PasteboardTypes,
    pub source: Option<AppRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Read,
    /// Don't read it. The reason is for the "copies skipped" counter, never for a log line
    /// with content.
    Skip(&'static str),
}

/// Decides, before anything is read, whether a copy may be read at all. The privacy filter
/// ([`PrivacyFilter`](crate::privacy::PrivacyFilter)) implements this.
pub trait ReadFilter: Send {
    fn check(&self, observed: &Observed) -> Verdict;

    /// The size limit for a read, in place of [`WatcherConfig::max_clip_bytes`]. Asked on each
    /// read, so a changed setting applies to the next copy.
    fn max_clip_bytes(&self) -> Option<u64> {
        None
    }
}

impl<F> ReadFilter for F
where
    F: Fn(&Observed) -> Verdict + Send,
{
    fn check(&self, observed: &Observed) -> Verdict {
        self(observed)
    }
}

/// A copy that passed the filter and was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capture {
    pub change_count: u64,
    pub source: Option<AppRef>,
    /// Only formats from [`uti::CAPTURE`].
    pub formats: Vec<RawFormat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEvent {
    Copied(Capture),
    /// Kopi wrote this change itself, by pasting. It counts as a use of a clip, not a copy.
    OwnWrite {
        change_count: u64,
    },
    /// The filter turned it down, or it held no format Kopi captures. Nothing was read.
    Skipped {
        change_count: u64,
        reason: &'static str,
    },
    /// It couldn't be read: no grant, too large, or the main thread was busy.
    Missed {
        change_count: u64,
        error: PasteboardError,
    },
    /// The access state changed, or was read for the first time at startup.
    AccessChanged(PasteboardAccess),
}

pub type WatchListener = Box<dyn FnMut(WatchEvent) + Send + 'static>;

/// The OS pieces the watcher uses. Tests pass fakes.
pub struct WatcherDeps {
    pub pasteboard: Box<dyn Pasteboard>,
    pub apps: Box<dyn ForegroundApp>,
    pub idle: Box<dyn UserIdle>,
}

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Kopi's bundle ID. A change with this in its source marker is Kopi's own write.
    pub own_bundle_id: String,
    /// Copies larger than this are missed with [`PasteboardError::TooLarge`], unless the
    /// filter sets its own limit.
    pub max_clip_bytes: u64,
    /// The pause between seeing a new change count and reading the format names, so a
    /// writer that's mid-write can finish.
    pub settle: Duration,
}

impl WatcherConfig {
    /// The defaults: a 50 MiB limit and a 30 ms settle.
    pub fn new(own_bundle_id: impl Into<String>) -> Self {
        Self {
            own_bundle_id: own_bundle_id.into(),
            max_clip_bytes: DEFAULT_MAX_CLIP_BYTES,
            settle: Duration::from_millis(30),
        }
    }
}

/// Owns the watcher thread. Dropping it stops the thread and waits for it.
///
/// On macOS each pasteboard call is run on the main thread, so dropping this on the main
/// thread can wait for up to the platform's main-thread timeout (5 s).
pub struct Watcher {
    stop: Option<Sender<()>>,
    worker: Option<JoinHandle<()>>,
}

impl Watcher {
    /// Starts watching. Whatever is on the clipboard now is not captured, only later copies.
    pub fn spawn(
        deps: WatcherDeps,
        config: WatcherConfig,
        filter: Box<dyn ReadFilter>,
        mut listener: WatchListener,
    ) -> std::io::Result<Self> {
        let (stop, stopped) = mpsc::channel::<()>();
        let worker = thread::Builder::new()
            .name("kopi-watcher".into())
            .spawn(move || {
                let mut poller = Poller::new(deps, config, filter);
                poller.start(&mut listener);
                loop {
                    poller.poll(&mut listener);
                    let wait = poll_interval(poller.deps.idle.idle_time());
                    match stopped.recv_timeout(wait) {
                        Err(RecvTimeoutError::Timeout) => {}
                        Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
                    }
                }
            })?;
        Ok(Self {
            stop: Some(stop),
            worker: Some(worker),
        })
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        drop(self.stop.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// 250 ms while the user is active, 500 ms after a minute without input.
pub fn poll_interval(idle: Option<Duration>) -> Duration {
    match idle {
        Some(idle) if idle >= IDLE_AFTER => IDLE_POLL,
        _ => ACTIVE_POLL,
    }
}

/// The formats to read from each item: the ones in [`uti::CAPTURE`], minus TIFF when the item
/// also has PNG. Apps often offer both for one image, and PNG is the smaller.
pub fn capture_plan(types: &PasteboardTypes) -> Vec<Vec<String>> {
    types
        .items
        .iter()
        .map(|item| {
            let has = |format: &str| item.iter().any(|f| f == format);
            uti::CAPTURE
                .iter()
                .filter(|&&format| has(format))
                .filter(|&&format| !(format == uti::TIFF && has(uti::PNG)))
                .map(|&format| format.to_owned())
                .collect()
        })
        .collect()
}

/// One poll at a time, on the watcher thread.
struct Poller {
    deps: WatcherDeps,
    config: WatcherConfig,
    filter: Box<dyn ReadFilter>,
    /// The last change count that was dealt with. `None` until the first successful poll,
    /// which only records where the clipboard was.
    handled: Option<u64>,
    /// The change count that was empty on the last poll, and how many polls found it so.
    empty: Option<(u64, u32)>,
    access: Option<PasteboardAccess>,
}

impl Poller {
    fn new(deps: WatcherDeps, config: WatcherConfig, filter: Box<dyn ReadFilter>) -> Self {
        Self {
            deps,
            config,
            filter,
            handled: None,
            empty: None,
            access: None,
        }
    }

    /// Reports the access state, so the app can show it straight away.
    fn start(&mut self, emit: &mut dyn FnMut(WatchEvent)) {
        let _ = self.check_access(emit);
    }

    fn poll(&mut self, emit: &mut dyn FnMut(WatchEvent)) {
        let Ok(count) = self.deps.pasteboard.change_count() else {
            return;
        };
        let Some(handled) = self.handled else {
            self.handled = Some(count);
            return;
        };
        if count == handled {
            return;
        }
        if !self.config.settle.is_zero() {
            thread::sleep(self.config.settle);
        }
        let Ok(types) = self.deps.pasteboard.types() else {
            return;
        };
        let count = types.change_count;
        if types.is_empty() {
            let tries = match self.empty {
                Some((c, tries)) if c == count => tries + 1,
                _ => 1,
            };
            if tries < EMPTY_RETRIES {
                self.empty = Some((count, tries));
                return;
            }
        }
        self.empty = None;
        self.handled = Some(count);
        if !types.is_empty() {
            self.handle(types, emit);
        }
    }

    fn handle(&mut self, types: PasteboardTypes, emit: &mut dyn FnMut(WatchEvent)) {
        let count = types.change_count;
        // Sampled first, because the user may switch apps while we work.
        let frontmost = self.deps.apps.current();

        match self.check_access(emit) {
            Some(access) if access.allows_background_reads() => {}
            Some(access) => {
                return emit(WatchEvent::Missed {
                    change_count: count,
                    error: PasteboardError::AccessDenied(access),
                });
            }
            None => {
                return emit(WatchEvent::Missed {
                    change_count: count,
                    error: PasteboardError::MainThreadBusy,
                });
            }
        }

        let source = if types.contains(uti::SOURCE) {
            match self.source_marker(count) {
                Ok(Some(id)) if id == self.config.own_bundle_id => {
                    return emit(WatchEvent::OwnWrite {
                        change_count: count,
                    });
                }
                Ok(Some(id)) => Some(self.deps.apps.running(&id).unwrap_or(AppRef {
                    bundle_id: Some(id),
                    name: None,
                })),
                Ok(None) => frontmost,
                Err(error) => return self.missed(count, error, emit),
            }
        } else {
            frontmost
        };

        let observed = Observed { types, source };
        if let Verdict::Skip(reason) = self.filter.check(&observed) {
            return emit(WatchEvent::Skipped {
                change_count: count,
                reason,
            });
        }

        let plan = capture_plan(&observed.types);
        if plan.iter().all(Vec::is_empty) {
            return emit(WatchEvent::Skipped {
                change_count: count,
                reason: "no supported format",
            });
        }
        let limit = self
            .filter
            .max_clip_bytes()
            .unwrap_or(self.config.max_clip_bytes);
        match self.deps.pasteboard.read(count, &plan, limit) {
            Ok(formats) if formats.is_empty() => {}
            Ok(formats) => emit(WatchEvent::Copied(Capture {
                change_count: count,
                source: observed.source,
                formats,
            })),
            Err(error) => self.missed(count, error, emit),
        }
    }

    /// The bundle ID in the source marker, if it holds one.
    fn source_marker(&self, count: u64) -> Result<Option<String>, PasteboardError> {
        let plan: Vec<Vec<String>> = vec![vec![uti::SOURCE.to_owned()]];
        let read = self.deps.pasteboard.read(count, &plan, MAX_SOURCE_BYTES)?;
        Ok(read
            .into_iter()
            .next()
            .and_then(|f| String::from_utf8(f.data).ok())
            .map(|id| {
                id.trim_matches(|c: char| c.is_whitespace() || c == '\0')
                    .to_owned()
            })
            .filter(|id| !id.is_empty()))
    }

    /// Reads the access state and reports it if it changed. `None` if it couldn't be read.
    fn check_access(&mut self, emit: &mut dyn FnMut(WatchEvent)) -> Option<PasteboardAccess> {
        let access = self.deps.pasteboard.access().ok()?;
        if self.access != Some(access) {
            self.access = Some(access);
            emit(WatchEvent::AccessChanged(access));
        }
        Some(access)
    }

    fn missed(&self, count: u64, error: PasteboardError, emit: &mut dyn FnMut(WatchEvent)) {
        // A newer copy replaced this one mid-read. The next poll picks it up.
        if error != PasteboardError::Changed {
            emit(WatchEvent::Missed {
                change_count: count,
                error,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    /// A pasteboard the test controls. Records every read.
    #[derive(Default)]
    struct State {
        change_count: u64,
        items: Vec<Vec<(String, Vec<u8>)>>,
        access: Option<PasteboardAccess>,
        reads: Vec<Vec<Vec<String>>>,
        /// Bumps the change count during the next read, as another app copying would.
        change_during_read: bool,
    }

    #[derive(Clone, Default)]
    struct FakePasteboard(Arc<Mutex<State>>);

    impl FakePasteboard {
        fn copy(&self, items: &[&[(&str, &[u8])]]) {
            let mut s = self.0.lock().unwrap();
            s.change_count += 1;
            s.items = items
                .iter()
                .map(|item| {
                    item.iter()
                        .map(|(f, d)| ((*f).to_owned(), d.to_vec()))
                        .collect()
                })
                .collect();
        }

        fn reads(&self) -> Vec<Vec<Vec<String>>> {
            self.0.lock().unwrap().reads.clone()
        }

        fn set(&self, f: impl FnOnce(&mut State)) {
            f(&mut self.0.lock().unwrap());
        }
    }

    impl Pasteboard for FakePasteboard {
        fn change_count(&self) -> Result<u64, PasteboardError> {
            Ok(self.0.lock().unwrap().change_count)
        }

        fn types(&self) -> Result<PasteboardTypes, PasteboardError> {
            let s = self.0.lock().unwrap();
            Ok(PasteboardTypes {
                change_count: s.change_count,
                items: s
                    .items
                    .iter()
                    .map(|item| item.iter().map(|(f, _)| f.clone()).collect())
                    .collect(),
            })
        }

        fn read(
            &self,
            change_count: u64,
            formats: &[Vec<String>],
            max_bytes: u64,
        ) -> Result<Vec<RawFormat>, PasteboardError> {
            let mut s = self.0.lock().unwrap();
            s.reads.push(formats.to_vec());
            let access = s.access.unwrap_or(PasteboardAccess::AlwaysAllow);
            if !access.allows_background_reads() {
                return Err(PasteboardError::AccessDenied(access));
            }
            if std::mem::take(&mut s.change_during_read) {
                s.change_count += 1;
            }
            if s.change_count != change_count {
                return Err(PasteboardError::Changed);
            }
            let mut out = Vec::new();
            for (index, (item, wanted)) in s.items.iter().zip(formats).enumerate() {
                for format in wanted {
                    if let Some((_, data)) = item.iter().find(|(f, _)| f == format) {
                        out.push(RawFormat {
                            item_index: index as u32,
                            format: format.clone(),
                            data: data.clone(),
                        });
                    }
                }
            }
            if out.iter().map(|f| f.data.len() as u64).sum::<u64>() > max_bytes {
                return Err(PasteboardError::TooLarge);
            }
            Ok(out)
        }

        fn access(&self) -> Result<PasteboardAccess, PasteboardError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .access
                .unwrap_or(PasteboardAccess::AlwaysAllow))
        }
    }

    struct FakeApps;

    impl ForegroundApp for FakeApps {
        fn current(&self) -> Option<AppRef> {
            Some(app("com.apple.Safari", "Safari"))
        }

        fn running(&self, bundle_id: &str) -> Option<AppRef> {
            (bundle_id == "com.apple.TextEdit").then(|| app("com.apple.TextEdit", "TextEdit"))
        }
    }

    struct NeverIdle;

    impl UserIdle for NeverIdle {
        fn idle_time(&self) -> Option<Duration> {
            Some(Duration::ZERO)
        }
    }

    fn app(id: &str, name: &str) -> AppRef {
        AppRef {
            bundle_id: Some(id.into()),
            name: Some(name.into()),
        }
    }

    const KOPI: &str = "in.devanshuverma.kopi";

    fn deps(pasteboard: &FakePasteboard) -> WatcherDeps {
        WatcherDeps {
            pasteboard: Box::new(pasteboard.clone()),
            apps: Box::new(FakeApps),
            idle: Box::new(NeverIdle),
        }
    }

    fn config() -> WatcherConfig {
        WatcherConfig {
            settle: Duration::ZERO,
            ..WatcherConfig::new(KOPI)
        }
    }

    /// A started poller and the events it has emitted so far.
    struct Harness {
        pasteboard: FakePasteboard,
        poller: Poller,
        events: Vec<WatchEvent>,
    }

    impl Harness {
        fn new() -> Self {
            Self::with_filter(Box::new(|_: &Observed| Verdict::Read))
        }

        fn with_filter(filter: Box<dyn ReadFilter>) -> Self {
            let pasteboard = FakePasteboard::default();
            let mut h = Self {
                poller: Poller::new(deps(&pasteboard), config(), filter),
                pasteboard,
                events: Vec::new(),
            };
            h.poller.start(&mut |e| h.events.push(e));
            h.poll();
            h.events.clear();
            h
        }

        fn poll(&mut self) -> Vec<WatchEvent> {
            let start = self.events.len();
            let events = &mut self.events;
            self.poller.poll(&mut |e| events.push(e));
            self.events[start..].to_vec()
        }
    }

    fn copied(events: &[WatchEvent]) -> &Capture {
        match events {
            [WatchEvent::Copied(capture)] => capture,
            other => panic!("expected one Copied event, got {other:?}"),
        }
    }

    #[test]
    fn reports_access_at_startup() {
        let pasteboard = FakePasteboard::default();
        let mut poller = Poller::new(
            deps(&pasteboard),
            config(),
            Box::new(|_: &Observed| Verdict::Read),
        );
        let mut events = Vec::new();
        poller.start(&mut |e| events.push(e));
        assert_eq!(
            events,
            [WatchEvent::AccessChanged(PasteboardAccess::AlwaysAllow)]
        );
    }

    #[test]
    fn whatever_was_there_at_startup_is_not_captured() {
        let pasteboard = FakePasteboard::default();
        pasteboard.copy(&[&[(uti::PLAIN_TEXT, b"before launch")]]);
        let mut poller = Poller::new(
            deps(&pasteboard),
            config(),
            Box::new(|_: &Observed| Verdict::Read),
        );
        let mut events = Vec::new();
        poller.poll(&mut |e| events.push(e));
        poller.poll(&mut |e| events.push(e));
        assert!(events.is_empty());
        assert!(pasteboard.reads().is_empty());
    }

    #[test]
    fn no_change_reads_nothing() {
        let mut h = Harness::new();
        assert!(h.poll().is_empty());
        assert!(h.pasteboard.reads().is_empty());
    }

    #[test]
    fn captures_a_copy_with_the_frontmost_app() {
        let mut h = Harness::new();
        h.pasteboard
            .copy(&[&[(uti::PLAIN_TEXT, b"hello"), (uti::HTML, b"<b>hello</b>")]]);
        let events = h.poll();
        let capture = copied(&events);
        assert_eq!(capture.change_count, 1);
        assert_eq!(capture.source, Some(app("com.apple.Safari", "Safari")));
        let formats: Vec<_> = capture.formats.iter().map(|f| f.format.as_str()).collect();
        assert_eq!(formats, [uti::PLAIN_TEXT, uti::HTML]);
        // The same change isn't reported twice.
        assert!(h.poll().is_empty());
    }

    #[test]
    fn reads_only_the_allow_list() {
        let mut h = Harness::new();
        h.pasteboard.copy(&[&[
            ("com.example.private", b"secret"),
            (uti::PLAIN_TEXT, b"hello"),
            ("org.chromium.internal.source-rfh-token", b"t"),
        ]]);
        let capture = copied(&h.poll()).clone();
        assert_eq!(capture.formats.len(), 1);
        assert_eq!(
            h.pasteboard.reads(),
            [vec![vec![uti::PLAIN_TEXT.to_owned()]]]
        );
    }

    #[test]
    fn reads_every_item() {
        let mut h = Harness::new();
        h.pasteboard.copy(&[
            &[(uti::FILE_URL, b"file:///a.txt")],
            &[(uti::FILE_URL, b"file:///b.txt")],
            &[(uti::FILE_URL, b"file:///c.txt")],
        ]);
        let capture = copied(&h.poll()).clone();
        let items: Vec<_> = capture.formats.iter().map(|f| f.item_index).collect();
        assert_eq!(items, [0, 1, 2]);
    }

    #[test]
    fn own_write_is_not_captured() {
        let mut h = Harness::new();
        h.pasteboard.copy(&[&[
            (uti::PLAIN_TEXT, b"pasted by kopi"),
            (uti::SOURCE, KOPI.as_bytes()),
        ]]);
        assert_eq!(h.poll(), [WatchEvent::OwnWrite { change_count: 1 }]);
        // Only the marker was read.
        assert_eq!(h.pasteboard.reads(), [vec![vec![uti::SOURCE.to_owned()]]]);
    }

    #[test]
    fn source_marker_names_the_app() {
        let mut h = Harness::new();
        h.pasteboard.copy(&[&[
            (uti::PLAIN_TEXT, b"hello"),
            (uti::SOURCE, b"com.apple.TextEdit"),
        ]]);
        assert_eq!(
            copied(&h.poll()).source,
            Some(app("com.apple.TextEdit", "TextEdit"))
        );
    }

    #[test]
    fn source_marker_for_an_app_that_isnt_running_keeps_the_id() {
        let mut h = Harness::new();
        h.pasteboard.copy(&[&[
            (uti::PLAIN_TEXT, b"hello"),
            (uti::SOURCE, b"com.example.script\n"),
        ]]);
        assert_eq!(
            copied(&h.poll()).source,
            Some(AppRef {
                bundle_id: Some("com.example.script".into()),
                name: None,
            })
        );
    }

    #[test]
    fn filter_skip_reads_nothing() {
        let mut h = Harness::with_filter(Box::new(|o: &Observed| {
            if o.types.contains("org.nspasteboard.ConcealedType") {
                Verdict::Skip("concealed")
            } else {
                Verdict::Read
            }
        }));
        h.pasteboard.copy(&[&[
            (uti::PLAIN_TEXT, b"hunter2"),
            ("org.nspasteboard.ConcealedType", b""),
        ]]);
        assert_eq!(
            h.poll(),
            [WatchEvent::Skipped {
                change_count: 1,
                reason: "concealed",
            }]
        );
        assert!(h.pasteboard.reads().is_empty());
    }

    #[test]
    fn filter_sees_the_source_app() {
        let mut h = Harness::with_filter(Box::new(|o: &Observed| {
            match o.source.as_ref().and_then(|a| a.bundle_id.as_deref()) {
                Some("com.apple.Safari") => Verdict::Skip("excluded app"),
                _ => Verdict::Read,
            }
        }));
        h.pasteboard.copy(&[&[(uti::PLAIN_TEXT, b"hello")]]);
        assert!(matches!(
            h.poll()[..],
            [WatchEvent::Skipped {
                reason: "excluded app",
                ..
            }]
        ));
    }

    #[test]
    fn without_the_grant_nothing_is_read() {
        let mut h = Harness::new();
        h.pasteboard.set(|s| s.access = Some(PasteboardAccess::Ask));
        h.pasteboard
            .copy(&[&[(uti::PLAIN_TEXT, b"hello"), (uti::SOURCE, b"x")]]);
        assert_eq!(
            h.poll(),
            [
                WatchEvent::AccessChanged(PasteboardAccess::Ask),
                WatchEvent::Missed {
                    change_count: 1,
                    error: PasteboardError::AccessDenied(PasteboardAccess::Ask),
                },
            ]
        );
        assert!(h.pasteboard.reads().is_empty());

        // The access change is reported once, then copies are captured again.
        h.pasteboard
            .set(|s| s.access = Some(PasteboardAccess::AlwaysAllow));
        h.pasteboard.copy(&[&[(uti::PLAIN_TEXT, b"hello")]]);
        let events = h.poll();
        assert_eq!(
            events[0],
            WatchEvent::AccessChanged(PasteboardAccess::AlwaysAllow)
        );
        assert!(matches!(events[1], WatchEvent::Copied(_)));
    }

    #[test]
    fn unsupported_formats_are_skipped_unread() {
        let mut h = Harness::new();
        h.pasteboard.copy(&[&[("com.example.private", b"x")]]);
        assert_eq!(
            h.poll(),
            [WatchEvent::Skipped {
                change_count: 1,
                reason: "no supported format",
            }]
        );
        assert!(h.pasteboard.reads().is_empty());
    }

    #[test]
    fn too_large_is_missed() {
        let mut h = Harness::new();
        h.poller.config.max_clip_bytes = 4;
        h.pasteboard.copy(&[&[(uti::PLAIN_TEXT, b"hello")]]);
        assert_eq!(
            h.poll(),
            [WatchEvent::Missed {
                change_count: 1,
                error: PasteboardError::TooLarge,
            }]
        );
    }

    #[test]
    fn the_filter_limit_replaces_the_configured_one() {
        struct Limit(u64);
        impl ReadFilter for Limit {
            fn check(&self, _: &Observed) -> Verdict {
                Verdict::Read
            }
            fn max_clip_bytes(&self) -> Option<u64> {
                Some(self.0)
            }
        }
        let mut h = Harness::with_filter(Box::new(Limit(4)));
        h.pasteboard.copy(&[&[(uti::PLAIN_TEXT, b"hello")]]);
        assert_eq!(
            h.poll(),
            [WatchEvent::Missed {
                change_count: 1,
                error: PasteboardError::TooLarge,
            }]
        );
    }

    #[test]
    fn a_copy_replaced_mid_read_is_picked_up_next_poll() {
        let mut h = Harness::new();
        h.pasteboard.copy(&[&[(uti::PLAIN_TEXT, b"first")]]);
        h.pasteboard.set(|s| s.change_during_read = true);
        assert!(h.poll().is_empty());
        assert_eq!(copied(&h.poll()).change_count, 2);
    }

    #[test]
    fn empty_pasteboard_is_retried_then_dropped() {
        let mut h = Harness::new();
        h.pasteboard.copy(&[]);
        for _ in 0..EMPTY_RETRIES {
            assert!(h.poll().is_empty());
        }
        assert_eq!(h.poller.handled, Some(1));

        // Cleared, then written at the same change count, as `clearContents` and
        // `writeObjects` do.
        h.pasteboard.copy(&[]);
        assert!(h.poll().is_empty());
        h.pasteboard
            .set(|s| s.items = vec![vec![(uti::PLAIN_TEXT.into(), b"late".to_vec())]]);
        assert_eq!(copied(&h.poll()).change_count, 2);
    }

    #[test]
    fn plan_prefers_png_over_tiff_per_item() {
        let types = PasteboardTypes {
            change_count: 1,
            items: vec![
                vec![uti::TIFF.into(), uti::PNG.into(), "com.adobe.pdf".into()],
                vec![uti::TIFF.into()],
            ],
        };
        assert_eq!(
            capture_plan(&types),
            [vec![uti::PNG.to_owned()], vec![uti::TIFF.to_owned()]]
        );
    }

    #[test]
    fn polls_slower_when_idle() {
        assert_eq!(poll_interval(None), ACTIVE_POLL);
        assert_eq!(poll_interval(Some(Duration::from_secs(59))), ACTIVE_POLL);
        assert_eq!(poll_interval(Some(IDLE_AFTER)), IDLE_POLL);
    }

    #[test]
    fn watcher_thread_reports_copies_and_stops_on_drop() {
        let pasteboard = FakePasteboard::default();
        let (tx, rx) = mpsc::channel();
        let watcher = Watcher::spawn(
            deps(&pasteboard),
            config(),
            Box::new(|_: &Observed| Verdict::Read),
            Box::new(move |e| {
                let _ = tx.send(e);
            }),
        )
        .unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            WatchEvent::AccessChanged(PasteboardAccess::AlwaysAllow)
        );
        // Let the first poll record where the clipboard was before copying.
        thread::sleep(ACTIVE_POLL / 2);
        pasteboard.copy(&[&[(uti::PLAIN_TEXT, b"hello")]]);
        let event = rx.recv_timeout(Duration::from_secs(2)).unwrap();
        assert!(matches!(event, WatchEvent::Copied(_)), "{event:?}");
        drop(watcher);
        // The listener was dropped with the thread.
        assert!(rx.recv_timeout(Duration::from_secs(1)).is_err());
    }
}
