//! Runs retention in the background. At startup it sweeps orphan files and applies the limits.
//! After that it runs at most once per interval while new clips arrive, and right away when
//! asked, for example after a limit is lowered. See `docs/DATA_MODEL.md`.

use std::sync::Arc;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::storage::{StorageResult, Store, now_ms};

pub use crate::storage::Limits;

/// How often retention runs at most while clips keep arriving.
pub const INTERVAL: Duration = Duration::from_secs(60);

/// Called on the retention thread after each run, with the ids it removed (often none) or
/// what went wrong.
pub type RetentionListener = Box<dyn Fn(StorageResult<Vec<i64>>) + Send + 'static>;

enum Wake {
    ClipsAdded,
    Now,
}

/// Owns the retention thread. Dropping it stops the thread once a run in progress finishes.
pub struct Retention {
    tx: Option<Sender<Wake>>,
    worker: Option<JoinHandle<()>>,
}

impl Retention {
    /// Starts the thread, which sweeps and runs once straight away. `interval` is
    /// [`INTERVAL`] outside tests.
    pub fn spawn(
        store: Arc<Store>,
        interval: Duration,
        listener: RetentionListener,
    ) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel::<Wake>();
        let worker = thread::Builder::new()
            .name("kopi-retention".into())
            .spawn(move || {
                if let Err(e) = store.sweep_orphans() {
                    listener(Err(e));
                }
                let run = || listener(apply(&store));
                run();
                let mut last_run = Instant::now();
                // A run is owed for clips that arrived too soon after the last one.
                let mut pending = false;
                loop {
                    let wake = if pending {
                        let due = (last_run + interval).saturating_duration_since(Instant::now());
                        match rx.recv_timeout(due) {
                            Ok(wake) => Some(wake),
                            Err(RecvTimeoutError::Timeout) => None,
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    } else {
                        match rx.recv() {
                            Ok(wake) => Some(wake),
                            Err(_) => break,
                        }
                    };
                    if let Some(Wake::ClipsAdded) = wake
                        && last_run.elapsed() < interval
                    {
                        pending = true;
                        continue;
                    }
                    pending = false;
                    run();
                    last_run = Instant::now();
                }
            })?;
        Ok(Self {
            tx: Some(tx),
            worker: Some(worker),
        })
    }

    /// A clip was added. Retention runs now, or once the interval since the last run is up.
    pub fn clips_added(&self) {
        self.send(Wake::ClipsAdded);
    }

    /// Runs retention now, for example after a limit is lowered.
    pub fn run_now(&self) {
        self.send(Wake::Now);
    }

    fn send(&self, wake: Wake) {
        if let Some(tx) = &self.tx {
            // The thread only stops when we drop the sender, so this can't fail.
            let _ = tx.send(wake);
        }
    }
}

impl Drop for Retention {
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn apply(store: &Store) -> StorageResult<Vec<i64>> {
    let limits = store.retention_limits()?;
    let now = now_ms();
    store.prune_tombstones(now)?;
    store.apply_retention(&limits, now)
}
