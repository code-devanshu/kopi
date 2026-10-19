//! Two and three sync services in one process, over loopback: pairing, sessions, live changes
//! and removing a computer. Discovery is off, so nothing goes on the network and no permission
//! is asked for. Computers are added by address instead.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use kopi_core::classifier::content_hash;
use kopi_core::storage::{ClipKind, NewClip, NewFormat, Platform, Store};
use kopi_sync::{Config, Error, SyncEvent, SyncService};

struct Computer {
    _dir: tempfile::TempDir,
    store: Arc<Store>,
    service: SyncService,
    events: Arc<Mutex<Vec<SyncEvent>>>,
}

impl Computer {
    async fn start(name: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(Store::open(dir.path()).unwrap());
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let service = SyncService::start(
            Arc::clone(&store),
            Config {
                data_dir: dir.path().to_path_buf(),
                app_version: "test".into(),
                device_name: name.into(),
                discovery: false,
            },
            Arc::new(move |event| sink.lock().unwrap().push(event)),
        )
        .await
        .unwrap();
        Self {
            _dir: dir,
            store,
            service,
            events,
        }
    }

    fn addr(&self) -> SocketAddr {
        SocketAddr::from((Ipv4Addr::LOCALHOST, self.service.port()))
    }

    fn copy(&self, text: &str) {
        self.store
            .upsert_clip(NewClip {
                kind: ClipKind::Text,
                plain_text: Some(text.into()),
                content_hash: content_hash(ClipKind::Text, text.as_bytes()),
                source_app_name: Some("Notes".into()),
                source_app_id: Some("com.apple.Notes".into()),
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

    fn has(&self, text: &str) -> bool {
        self.store
            .clip_id_by_hash(&content_hash(ClipKind::Text, text.as_bytes()))
            .unwrap()
            .is_some()
    }

    fn connected_to(&self, other: &Computer) -> bool {
        self.service
            .status()
            .connected
            .contains(&other.service.device_id().to_owned())
    }

    fn code(&self) -> String {
        self.service.status().pairing.expect("pairing is open").code
    }

    /// Pairs with `other`, which shows its code.
    async fn pair_with(&self, other: &Computer) -> Result<String, Error> {
        other.service.start_pairing().unwrap();
        let id = self.service.add_address(other.addr(), Some("other".into()));
        // The address goes through the same queue as discovery.
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.service.pair(&id, &other.code()).await
    }
}

async fn eventually(what: &str, check: impl Fn() -> bool) {
    for _ in 0..400 {
        if check() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("timed out waiting for {what}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pair_sync_and_remove() {
    let mac = Computer::start("MacBook").await;
    let pc = Computer::start("Windows PC").await;
    mac.copy("on the mac before pairing");

    // A wrong code fails, on both sides, and the code changes.
    mac.service.start_pairing().unwrap();
    let first = mac.code();
    let id = pc.service.add_address(mac.addr(), Some("MacBook".into()));
    tokio::time::sleep(Duration::from_millis(50)).await;
    let wrong = if first == "000000" {
        "000001"
    } else {
        "000000"
    };
    assert!(pc.service.pair(&id, wrong).await.is_err());
    eventually("a new code", || mac.code() != first).await;
    assert!(
        mac.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, SyncEvent::PairingFailed { .. }))
    );
    assert!(!mac.connected_to(&pc));

    // The right one pairs, and the two start syncing.
    assert_eq!(pc.service.pair(&id, &mac.code()).await.unwrap(), "MacBook");
    eventually("the mac to see the pairing", || {
        mac.events
            .lock()
            .unwrap()
            .iter()
            .any(|e| matches!(e, SyncEvent::Paired { name } if name == "Windows PC"))
    })
    .await;
    eventually("a session", || {
        mac.connected_to(&pc) && pc.connected_to(&mac)
    })
    .await;
    eventually("catch-up", || pc.has("on the mac before pairing")).await;

    pc.copy("live from the pc");
    eventually("a live change", || mac.has("live from the pc")).await;
    let id = mac
        .store
        .clip_id_by_hash(&content_hash(ClipKind::Text, b"live from the pc"))
        .unwrap()
        .unwrap();
    assert_eq!(
        mac.store
            .get_clip(id)
            .unwrap()
            .unwrap()
            .copied_on
            .as_deref(),
        Some("Windows PC")
    );

    // Removed: the session ends, and the pc's key is refused from then on.
    let pc_id = pc.service.device_id().to_owned();
    assert!(mac.store.remove_device(&pc_id).unwrap());
    mac.service.disconnect(&pc_id);
    eventually("the session to end", || {
        !mac.connected_to(&pc) && !pc.connected_to(&mac)
    })
    .await;
    pc.copy("after removal");
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!mac.has("after removal"));
    assert!(!mac.connected_to(&pc));

    mac.service.stop().await;
    pc.service.stop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_third_computer_joins_through_the_second() {
    let a = Computer::start("A").await;
    let b = Computer::start("B").await;
    let c = Computer::start("C").await;
    b.pair_with(&a).await.unwrap();
    eventually("a and b", || a.connected_to(&b)).await;
    // C pairs with B only. A learns of C from B, then they connect directly.
    c.pair_with(&b).await.unwrap();
    eventually("b and c", || b.connected_to(&c)).await;
    eventually("a to learn of c", || {
        a.store
            .sync_devices()
            .unwrap()
            .iter()
            .any(|d| d.device_id == c.service.device_id())
    })
    .await;
    // C was told of A's address only through B's group, which has no addresses: add it, as
    // discovery would.
    c.service.add_address(a.addr(), None);
    eventually("a and c", || a.connected_to(&c) && c.connected_to(&a)).await;

    a.copy("from a");
    eventually("c to get it", || c.has("from a")).await;
    for computer in [a, b, c] {
        computer.service.stop().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn strangers_are_refused() {
    let a = Computer::start("A").await;
    let stranger = Computer::start("Stranger").await;
    a.copy("private");
    stranger.service.add_address(a.addr(), None);
    a.service.add_address(stranger.addr(), None);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(!a.connected_to(&stranger));
    assert!(!stranger.has("private"));
    a.service.stop().await;
    stranger.service.stop().await;
}
