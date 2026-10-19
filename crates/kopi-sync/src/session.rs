//! A sync session with a paired computer, once the Noise handshake has checked its key.
//!
//! Each side says what it already has from the other in its [`Hello`], then sends every change
//! after that in batches, and from then on each new change as it happens. Received batches are
//! applied in one transaction each and acknowledged. A session that's silent for
//! [`IDLE_TIMEOUT`] ends.

use std::sync::Arc;
use std::time::Duration;

use kopi_core::storage::sync::Applied;
use kopi_core::storage::{ClipKind, Store, now_ms};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, watch};
use tokio::time::timeout;

use crate::noise::{Receiver, Sender};
use crate::protocol::{self, Hello, Message, PROTOCOL};
use crate::{Error, log};

/// How often a quiet session sends a ping.
pub const PING_EVERY: Duration = Duration::from_secs(30);
/// How long a session may be silent before it ends.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(90);
const HELLO_TIMEOUT: Duration = Duration::from_secs(15);

/// What a session needs from the service.
pub(crate) struct Context {
    pub store: Arc<Store>,
    pub self_id: String,
    pub peer_id: String,
    pub app_version: String,
    /// Bumped by the store after each change, so new changes go out at once.
    pub changes: watch::Receiver<u64>,
    /// Set to end the session.
    pub stop: watch::Receiver<bool>,
    /// Called after each batch from the other side is applied.
    pub on_applied: Arc<dyn Fn(Applied) + Send + Sync>,
}

/// Runs a session until it ends, and returns why. `Ok` means it was asked to stop.
pub(crate) async fn run<R, W>(
    mut ctx: Context,
    mut sender: Sender<W>,
    mut receiver: Receiver<R>,
) -> Result<(), Error>
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let store = Arc::clone(&ctx.store);
    let peer = ctx.peer_id.clone();
    let (received_seq, kinds) = blocking(move || {
        let received_seq = store
            .sync_devices()?
            .into_iter()
            .find(|d| d.device_id == peer)
            .map_or(0, |d| d.received_seq);
        Ok((received_seq, store.sync_kinds()?))
    })
    .await?;
    let hello = Hello {
        protocol: PROTOCOL,
        app_version: ctx.app_version.clone(),
        device_id: ctx.self_id.clone(),
        received_seq,
        kinds: kinds.iter().map(|k| k.as_str().to_owned()).collect(),
    };
    sender
        .send(&protocol::encode(&Message::Hello(hello))?)
        .await?;
    let theirs = match timeout(HELLO_TIMEOUT, receiver.recv()).await {
        Ok(bytes) => protocol::decode::<Message>(&bytes?)?,
        Err(_) => return Err(Error::Timeout),
    };
    let Message::Hello(theirs) = theirs else {
        return Err(Error::Protocol("expected a hello".into()));
    };
    if theirs.device_id != ctx.peer_id {
        return Err(Error::Protocol("the hello names another computer".into()));
    }
    if theirs.protocol > PROTOCOL {
        return Err(Error::NeedsUpdate);
    }
    let their_kinds = theirs
        .kinds
        .iter()
        .filter_map(|k| k.parse::<ClipKind>().ok())
        .collect();

    // One task writes. The others hand it messages.
    let (out_tx, out_rx) = mpsc::channel::<Message>(4);
    let writer = tokio::spawn(write_loop(sender, out_rx));
    let outgoing = tokio::spawn(send_changes(
        Arc::clone(&ctx.store),
        ctx.self_id.clone(),
        theirs.received_seq,
        their_kinds,
        ctx.changes.clone(),
        out_tx.clone(),
    ));

    let result = loop {
        let next = tokio::select! {
            next = timeout(IDLE_TIMEOUT, receiver.recv()) => next,
            _ = ctx.stop.changed() => break Ok(()),
        };
        let bytes = match next {
            Ok(Ok(bytes)) => bytes,
            Ok(Err(e)) => break Err(e),
            Err(_) => break Err(Error::Timeout),
        };
        match protocol::decode::<Message>(&bytes) {
            Ok(Message::Changes(changes)) => {
                let store = Arc::clone(&ctx.store);
                let (peer, me) = (ctx.peer_id.clone(), ctx.self_id.clone());
                let seq = changes.max_seq;
                let rows = changes.len();
                match blocking(move || Ok(store.apply_changes(&peer, &me, changes)?)).await {
                    Ok(applied) => {
                        if applied.rejected > 0 {
                            log(&format!(
                                "{}: dropped {} of {rows} clips that failed their checks",
                                short(&ctx.peer_id),
                                applied.rejected
                            ));
                        }
                        (ctx.on_applied)(applied);
                        // Never wait to queue an ack. If both sides are sending a lot, each
                        // writer waits for the other to read, and a reader stuck behind its own
                        // writer would leave both waiting for ever. An ack only shows progress.
                        match out_tx.try_send(Message::Ack { seq }) {
                            Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => {}
                            Err(mpsc::error::TrySendError::Closed(_)) => break Err(Error::Closed),
                        }
                    }
                    Err(e) => break Err(e),
                }
            }
            Ok(Message::Ack { .. } | Message::Ping) => {}
            Ok(Message::Hello(_)) => break Err(Error::Protocol("a second hello".into())),
            Err(e) => break Err(e),
        }
    };
    outgoing.abort();
    drop(out_tx);
    // The writer ends once every sender is gone, after what's queued.
    let _ = timeout(Duration::from_secs(2), writer).await;
    let store = Arc::clone(&ctx.store);
    let peer = ctx.peer_id.clone();
    let _ = blocking(move || Ok(store.device_seen(&peer, now_ms())?)).await;
    result
}

/// Sends everything after `cursor`, then waits for new changes and sends those.
async fn send_changes(
    store: Arc<Store>,
    self_id: String,
    mut cursor: i64,
    kinds: kopi_core::privacy::KindSet,
    mut changes: watch::Receiver<u64>,
    out: mpsc::Sender<Message>,
) {
    loop {
        changes.mark_unchanged();
        let (store2, me) = (Arc::clone(&store), self_id.clone());
        let batch = match blocking(move || Ok(store2.changes_since(cursor, kinds, &me)?)).await {
            Ok(batch) => batch,
            Err(e) => {
                log(&format!("couldn't read changes to send: {e}"));
                return;
            }
        };
        if batch.max_seq > cursor {
            cursor = batch.max_seq;
            if !batch.is_empty() && out.send(Message::Changes(batch)).await.is_err() {
                return;
            }
            continue;
        }
        if changes.changed().await.is_err() {
            return;
        }
    }
}

async fn write_loop<W: AsyncWrite + Unpin>(mut sender: Sender<W>, mut rx: mpsc::Receiver<Message>) {
    loop {
        let message = match timeout(PING_EVERY, rx.recv()).await {
            Ok(Some(message)) => message,
            Ok(None) => break,
            Err(_) => Message::Ping,
        };
        let bytes = match protocol::encode(&message) {
            Ok(bytes) => bytes,
            Err(e) => {
                log(&format!("couldn't encode a message: {e}"));
                break;
            }
        };
        if sender.send(&bytes).await.is_err() {
            break;
        }
    }
    sender.shutdown().await;
}

/// Runs store work off the async threads.
pub(crate) async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, Error> + Send + 'static,
) -> Result<T, Error> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| Error::Protocol(format!("a store task failed: {e}")))?
}

/// The end of a device ID, for logs. UUIDv7s start with the time, so the end tells them apart.
pub(crate) fn short(device_id: &str) -> &str {
    device_id
        .get(device_id.len().saturating_sub(8)..)
        .unwrap_or(device_id)
}

#[cfg(test)]
mod tests {
    use kopi_core::classifier::content_hash;
    use kopi_core::storage::{NewClip, NewFormat, Platform};

    use super::*;
    use crate::noise::{self, Role, SYNC_PARAMS, SYNC_PROLOGUE};

    struct Side {
        _dir: tempfile::TempDir,
        store: Arc<Store>,
        id: String,
        key: snow::Keypair,
        changes: watch::Sender<u64>,
    }

    fn side(id: &str) -> Side {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path()).unwrap());
        let key = noise::generate_keypair();
        store
            .set_self_device(id, &key.public, &format!("{id} computer"), Platform::MacOs)
            .unwrap();
        let (changes, _) = watch::channel(0u64);
        let notify = changes.clone();
        store.set_change_listener(Box::new(move || notify.send_modify(|n| *n += 1)));
        Side {
            _dir: dir,
            store,
            id: id.into(),
            key,
            changes,
        }
    }

    fn copy(store: &Store, text: &str) {
        store
            .upsert_clip(NewClip {
                kind: ClipKind::Text,
                plain_text: Some(text.into()),
                content_hash: content_hash(ClipKind::Text, text.as_bytes()),
                source_app_name: None,
                source_app_id: None,
                metadata: None,
                platform: Platform::MacOs,
                formats: vec![NewFormat {
                    item_index: 0,
                    format: "public.utf8-plain-text".into(),
                    data: text.as_bytes().to_vec(),
                }],
                local_only: false,
            })
            .unwrap();
    }

    fn has(store: &Store, text: &str) -> bool {
        store
            .clip_id_by_hash(&content_hash(ClipKind::Text, text.as_bytes()))
            .unwrap()
            .is_some()
    }

    async fn eventually(what: &str, check: impl Fn() -> bool) {
        for _ in 0..1200 {
            if check() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        panic!("timed out waiting for {what}");
    }

    async fn connect(a: &Side, b: &Side) -> (watch::Sender<bool>, watch::Sender<bool>) {
        let (mut x, mut y) = tokio::io::duplex(1 << 16);
        let (ak, bk) = (a.key.private.clone(), b.key.private.clone());
        let ta = tokio::spawn(async move {
            let (t, _) = noise::handshake(
                &mut x,
                SYNC_PARAMS,
                SYNC_PROLOGUE,
                &ak,
                None,
                Role::Initiator,
                |_| true,
            )
            .await
            .unwrap();
            (t, x)
        });
        let (tb, _) = noise::handshake(
            &mut y,
            SYNC_PARAMS,
            SYNC_PROLOGUE,
            &bk,
            None,
            Role::Responder,
            |_| true,
        )
        .await
        .unwrap();
        let (ta, x) = ta.await.unwrap();
        let mut stops = Vec::new();
        for (me, them, transport, stream) in [(a, b, ta, x), (b, a, tb, y)] {
            let (read, write) = tokio::io::split(stream);
            let (sender, receiver) = noise::split(transport, read, write);
            let (stop, stopped) = watch::channel(false);
            let ctx = Context {
                store: Arc::clone(&me.store),
                self_id: me.id.clone(),
                peer_id: them.id.clone(),
                app_version: "test".into(),
                changes: me.changes.subscribe(),
                stop: stopped,
                on_applied: Arc::new(|_| {}),
            };
            tokio::spawn(run(ctx, sender, receiver));
            stops.push(stop);
        }
        let b_stop = stops.pop().unwrap();
        (stops.pop().unwrap(), b_stop)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn a_big_catch_up_both_ways_finishes() {
        let (a, b) = (side("a"), side("b"));
        a.store
            .pair_device("b", &b.key.public, "b computer", Platform::Windows)
            .unwrap();
        b.store
            .pair_device("a", &a.key.public, "a computer", Platform::MacOs)
            .unwrap();
        // About 15 MB each way, through a 64 KiB pipe: both writers fill it at once.
        let filler = "x".repeat(300_000);
        for i in 0..50 {
            copy(&a.store, &format!("a {i} {filler}"));
            copy(&b.store, &format!("b {i} {filler}"));
        }
        let (stop_a, stop_b) = connect(&a, &b).await;
        eventually("a's clips on b", || {
            has(&b.store, &format!("a 49 {filler}"))
        })
        .await;
        eventually("b's clips on a", || {
            has(&a.store, &format!("b 49 {filler}"))
        })
        .await;
        let _ = stop_a.send(true);
        let _ = stop_b.send(true);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn catches_up_then_stays_live() {
        let (a, b) = (side("a"), side("b"));
        a.store
            .pair_device("b", &b.key.public, "b computer", Platform::Windows)
            .unwrap();
        b.store
            .pair_device("a", &a.key.public, "a computer", Platform::MacOs)
            .unwrap();
        for i in 0..300 {
            copy(&a.store, &format!("before {i}"));
        }
        let (stop_a, stop_b) = connect(&a, &b).await;
        eventually("catch-up", || {
            has(&b.store, "before 299") && has(&b.store, "before 0")
        })
        .await;

        copy(&a.store, "live from a");
        copy(&b.store, "live from b");
        eventually("a live change", || has(&b.store, "live from a")).await;
        eventually("a live change back", || has(&a.store, "live from b")).await;

        let _ = stop_a.send(true);
        let _ = stop_b.send(true);
        // What each has from the other is remembered for next time.
        let received = b
            .store
            .sync_devices()
            .unwrap()
            .into_iter()
            .find(|d| d.device_id == "a")
            .unwrap()
            .received_seq;
        assert!(received >= 301, "{received}");
    }
}
