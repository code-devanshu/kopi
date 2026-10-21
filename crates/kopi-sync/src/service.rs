//! The sync service, which runs while sync is on: it listens for connections, advertises this
//! computer, dials the other computers it finds, runs a session with each paired one, and
//! handles pairing.
//!
//! Every computer dials every `_kopi._tcp` instance it finds. The Noise handshake reveals the
//! other side's key first, and the connection ends there if the key isn't in the group. When
//! two computers dial each other at once, the connection opened by the lower device ID stays.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use kopi_core::storage::sync::Applied;
use kopi_core::storage::{Platform, Store, now_ms};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, mpsc, watch};
use tokio::task::JoinHandle;
use tokio::time::timeout;

use crate::discovery::{Discovery, Found};
use crate::identity::Identity;
use crate::noise::{self, Role, SYNC_PARAMS, SYNC_PROLOGUE};
use crate::pairing;
use crate::protocol::{PAIR_CONNECTION, PROTOCOL, PairInfo, SYNC_CONNECTION};
use crate::session::{self, Context, blocking, short};
use crate::{Error, log, probe};

/// How often to dial the computers found that no session runs with.
const DIAL_EVERY: Duration = Duration::from_secs(10);
/// How long to wait before dialing a computer again after its session ended.
const REDIAL_AFTER: Duration = Duration::from_secs(5);
/// How long to leave a computer that isn't paired with this one, or can't be reached.
const STRANGER_RETRY: Duration = Duration::from_secs(10 * 60);
const UNREACHABLE_RETRY: Duration = Duration::from_secs(30);
/// How long to leave a computer that needs a newer Kopi.
const NEEDS_UPDATE_RETRY: Duration = Duration::from_secs(5 * 60);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const PAIRING_TIMEOUT: Duration = Duration::from_secs(30);
/// How long a pairing code lasts.
pub const CODE_LIFETIME_MS: i64 = 5 * 60 * 1000;
/// How often to check the network permission and the code's age.
const CHECK_EVERY: Duration = Duration::from_secs(15);

pub struct Config {
    /// Kopi's data folder. The identity is in `sync/device.key` under it.
    pub data_dir: PathBuf,
    pub app_version: String,
    /// This computer's name, as the other computers show it.
    pub device_name: String,
    /// Advertise and browse with mDNS, and check the network permission. Always on in the
    /// app. Tests turn it off and connect by address, which needs no permission on loopback.
    pub discovery: bool,
}

/// What the service tells the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEvent {
    /// Another computer's changes were merged: clips or pinboards to show.
    Applied(Applied),
    /// Something Settings → Sync shows changed: who's connected, pairing, a problem.
    StatusChanged,
    /// Pairing finished. The other computer's name.
    Paired { name: String },
    /// A pairing attempt failed, on either side.
    PairingFailed { reason: String },
}

pub type EventSink = Arc<dyn Fn(SyncEvent) + Send + Sync>;

/// Why sync can't do its job right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// macOS refuses Kopi's local network traffic: Local Network is turned off for it.
    LocalNetworkDenied,
    /// Advertising and browsing failed, so no computers can be found.
    Discovery(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    /// This computer's device ID.
    pub device_id: String,
    /// Paired computers with a session running now.
    pub connected: Vec<String>,
    /// Paired computers running a newer protocol, so this Kopi needs an update.
    pub needs_update: Vec<String>,
    pub problem: Option<Problem>,
    pub pairing: Option<PairingStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingStatus {
    /// The code to show, 6 digits.
    pub code: String,
    /// When it stops working, in Unix milliseconds.
    pub expires_at: i64,
    /// Computers nearby with their pairing sheet open.
    pub nearby: Vec<Nearby>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nearby {
    /// Names the computer for [`SyncService::pair`].
    pub id: String,
    pub name: String,
}

pub struct SyncService {
    inner: Arc<Inner>,
    stop: watch::Sender<bool>,
    tasks: Vec<JoinHandle<()>>,
}

struct Inner {
    store: Arc<Store>,
    identity: Identity,
    app_version: String,
    events: EventSink,
    changes: watch::Sender<u64>,
    stopped: watch::Receiver<bool>,
    state: Mutex<State>,
    discovery: Mutex<Option<Discovery>>,
    own_instance: String,
    dial_now: Notify,
    port: u16,
    /// Where discovery reports, and [`SyncService::add_address`] too.
    found: mpsc::UnboundedSender<Found>,
    check_network: bool,
}

#[derive(Default)]
struct State {
    name: String,
    sessions: HashMap<String, SessionEntry>,
    instances: HashMap<String, Instance>,
    pairing: Option<Pairing>,
    needs_update: HashSet<String>,
    problem: Option<Problem>,
    next_session: u64,
}

struct SessionEntry {
    id: u64,
    dialed_here: bool,
    stop: watch::Sender<bool>,
}

struct Instance {
    addrs: Vec<SocketAddr>,
    pairing_name: Option<String>,
    /// The paired computer behind it, once a handshake has shown which.
    device: Option<String>,
    dialing: bool,
    retry_at: Instant,
}

struct Pairing {
    code: String,
    expires_at: i64,
    /// An attempt is running. One at a time.
    busy: bool,
}

impl SyncService {
    /// Starts sync: makes this computer's identity if it has none, puts it in the group,
    /// listens and advertises. Fails if it can't listen.
    pub async fn start(
        store: Arc<Store>,
        config: Config,
        events: EventSink,
    ) -> Result<Self, Error> {
        let data_dir = config.data_dir.clone();
        let identity = blocking(move || Ok(Identity::load_or_create(&data_dir)?)).await?;
        {
            let (store, identity, name) = (
                Arc::clone(&store),
                identity.clone(),
                config.device_name.clone(),
            );
            blocking(move || {
                Ok(store.set_self_device(
                    &identity.device_id,
                    &identity.public_key,
                    &name,
                    local_platform(),
                )?)
            })
            .await?;
        }

        let listener = TcpListener::bind(("0.0.0.0", 0)).await?;
        let port = listener.local_addr()?.port();
        let (found_tx, found_rx) = mpsc::unbounded_channel();
        let (discovery, problem) = if config.discovery {
            match Discovery::start(port, found_tx.clone()) {
                Ok(discovery) => (Some(discovery), None),
                Err(e) => {
                    log(&format!("discovery failed to start: {e}"));
                    (None, Some(Problem::Discovery(e.to_string())))
                }
            }
        } else {
            (None, None)
        };
        let own_instance = discovery
            .as_ref()
            .map(Discovery::own_fullname)
            .unwrap_or_default();

        let (changes, _) = watch::channel(0u64);
        let notify = changes.clone();
        store.set_change_listener(Box::new(move || notify.send_modify(|n| *n += 1)));
        let (stop, stopped) = watch::channel(false);
        let inner = Arc::new(Inner {
            store,
            identity,
            app_version: config.app_version,
            events,
            changes,
            stopped,
            state: Mutex::new(State {
                name: config.device_name,
                problem,
                ..State::default()
            }),
            discovery: Mutex::new(discovery),
            own_instance,
            dial_now: Notify::new(),
            port,
            found: found_tx,
            check_network: config.discovery,
        });
        log(&format!(
            "on as {} ({}), port {port}",
            short(&inner.identity.device_id),
            crate::identity::fingerprint(&inner.identity.public_key)
        ));
        let tasks = vec![
            tokio::spawn(Arc::clone(&inner).accept_loop(listener)),
            tokio::spawn(Arc::clone(&inner).discovery_loop(found_rx)),
            tokio::spawn(Arc::clone(&inner).dial_loop()),
            tokio::spawn(Arc::clone(&inner).check_loop()),
        ];
        Ok(Self { inner, stop, tasks })
    }

    /// Stops everything: sessions end, the port closes and the advertisement is withdrawn.
    pub async fn stop(self) {
        let _ = self.stop.send(true);
        self.inner.store.set_change_listener(Box::new(|| {}));
        for entry in self.inner.lock().sessions.drain().map(|(_, entry)| entry) {
            let _ = entry.stop.send(true);
        }
        for task in &self.tasks {
            task.abort();
        }
        let discovery = self
            .inner
            .discovery
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take();
        if let Some(discovery) = discovery {
            let _ = tokio::task::spawn_blocking(move || discovery.stop()).await;
        }
        log("off");
    }

    pub fn status(&self) -> Status {
        let state = self.inner.lock();
        let mut connected: Vec<String> = state.sessions.keys().cloned().collect();
        connected.sort();
        let mut needs_update: Vec<String> = state.needs_update.iter().cloned().collect();
        needs_update.sort();
        Status {
            device_id: self.inner.identity.device_id.clone(),
            connected,
            needs_update,
            problem: state.problem.clone(),
            pairing: state.pairing.as_ref().map(|p| PairingStatus {
                code: p.code.clone(),
                expires_at: p.expires_at,
                nearby: nearby(&state.instances),
            }),
        }
    }

    /// This computer's device ID.
    pub fn device_id(&self) -> &str {
        &self.inner.identity.device_id
    }

    /// The port this computer listens on.
    pub fn port(&self) -> u16 {
        self.inner.port
    }

    /// Adds a computer by address, as if discovery had found it, for networks where mDNS
    /// doesn't get through but connections do. With `pairing_name`, it's listed for pairing.
    /// Returns the ID [`pair`](Self::pair) takes.
    pub fn add_address(&self, addr: SocketAddr, pairing_name: Option<String>) -> String {
        let instance = format!("address:{addr}");
        let _ = self.inner.found.send(Found::Resolved {
            instance: instance.clone(),
            addrs: vec![addr],
            pairing_name,
        });
        instance
    }

    /// Opens pairing: shows a code and lists this computer to others in pairing mode.
    pub fn start_pairing(&self) -> Result<(), Error> {
        let name = {
            let mut state = self.inner.lock();
            if state.pairing.is_none() {
                state.pairing = Some(Pairing {
                    code: pairing::new_code(),
                    expires_at: now_ms() + CODE_LIFETIME_MS,
                    busy: false,
                });
            }
            state.name.clone()
        };
        self.inner.advertise(Some(&name))?;
        self.inner.status_changed();
        Ok(())
    }

    /// Closes pairing. The code stops working.
    pub fn stop_pairing(&self) -> Result<(), Error> {
        let was_pairing = self.inner.lock().pairing.take().is_some();
        if was_pairing {
            self.inner.advertise(None)?;
            self.inner.status_changed();
        }
        Ok(())
    }

    /// Pairs with a computer nearby that's in pairing mode, with the code it shows. Returns its
    /// name.
    pub async fn pair(&self, nearby_id: &str, code: &str) -> Result<String, Error> {
        let code = pairing::normalize_code(code).ok_or(Error::BadCode)?;
        let addrs = {
            let state = self.inner.lock();
            match state.instances.get(nearby_id) {
                Some(instance) if instance.pairing_name.is_some() => instance.addrs.clone(),
                _ => return Err(Error::NotPairing),
            }
        };
        let mut stream = connect(&addrs).await.ok_or(Error::NotPairing)?;
        stream.write_all(&[PAIR_CONNECTION, PROTOCOL as u8]).await?;
        let mine = self.inner.pair_info();
        let result = timeout(
            PAIRING_TIMEOUT,
            pairing::run(
                &mut stream,
                Role::Initiator,
                &code,
                &self.inner.identity,
                mine,
            ),
        )
        .await
        .unwrap_or(Err(Error::Timeout));
        let (info, key) = match result {
            Ok(done) => done,
            Err(e) => {
                log(&format!("pairing failed: {e}"));
                return Err(e);
            }
        };
        let name = self.inner.finish_pairing(info, key).await?;
        if let Some(instance) = self.inner.lock().instances.get_mut(nearby_id) {
            instance.retry_at = Instant::now();
        }
        self.inner.dial_now.notify_one();
        Ok(name)
    }

    /// Renames this computer in the group. The others see the new name when they next sync.
    pub async fn rename(&self, name: &str) -> Result<(), Error> {
        let (store, identity, owned) = (
            Arc::clone(&self.inner.store),
            self.inner.identity.clone(),
            name.to_owned(),
        );
        blocking(move || {
            Ok(store.set_self_device(
                &identity.device_id,
                &identity.public_key,
                &owned,
                local_platform(),
            )?)
        })
        .await?;
        let pairing = {
            let mut state = self.inner.lock();
            state.name = name.to_owned();
            state.pairing.is_some()
        };
        if pairing {
            self.inner.advertise(Some(name))?;
        }
        Ok(())
    }

    /// Ends the session with a computer that was just removed from the group.
    pub fn disconnect(&self, device_id: &str) {
        let entry = self.inner.lock().sessions.remove(device_id);
        if let Some(entry) = entry {
            let _ = entry.stop.send(true);
            self.inner.status_changed();
        }
    }

    /// Ends every session, to be dialed again in a moment. A session's hello says what this
    /// computer takes, so a change to that needs new sessions.
    pub fn reconnect(&self) {
        let entries: Vec<SessionEntry> = self
            .inner
            .lock()
            .sessions
            .drain()
            .map(|(_, entry)| entry)
            .collect();
        if entries.is_empty() {
            return;
        }
        for entry in entries {
            let _ = entry.stop.send(true);
        }
        self.inner.status_changed();
    }
}

impl Inner {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn is_stopped(&self) -> bool {
        *self.stopped.borrow()
    }

    fn status_changed(&self) {
        (self.events)(SyncEvent::StatusChanged);
    }

    fn advertise(&self, pairing_name: Option<&str>) -> Result<(), Error> {
        let mut discovery = self
            .discovery
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match discovery.as_mut() {
            Some(discovery) => discovery.advertise(pairing_name),
            None => Ok(()),
        }
    }

    fn pair_info(&self) -> PairInfo {
        PairInfo {
            protocol: PROTOCOL,
            device_id: self.identity.device_id.clone(),
            name: self.lock().name.clone(),
            os: local_platform().as_str().to_owned(),
        }
    }

    async fn accept_loop(self: Arc<Self>, listener: TcpListener) {
        let mut stopped = self.stopped.clone();
        loop {
            let accepted = tokio::select! {
                accepted = listener.accept() => accepted,
                _ = stopped.changed() => return,
            };
            match accepted {
                Ok((stream, _)) => {
                    tokio::spawn(Arc::clone(&self).incoming(stream));
                }
                Err(e) => {
                    log(&format!("accept failed: {e}"));
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    async fn incoming(self: Arc<Self>, mut stream: TcpStream) {
        let mut head = [0u8; 2];
        if !matches!(
            timeout(HANDSHAKE_TIMEOUT, stream.read_exact(&mut head)).await,
            Ok(Ok(_))
        ) {
            return;
        }
        match head[0] {
            SYNC_CONNECTION => self.sync_session(stream, Role::Responder, None).await,
            PAIR_CONNECTION => self.pair_respond(stream).await,
            _ => {}
        }
    }

    /// Runs a handshake, then a session if the other side is in the group.
    async fn sync_session(
        self: Arc<Self>,
        mut stream: TcpStream,
        role: Role,
        instance: Option<String>,
    ) {
        let _ = stream.set_nodelay(true);
        let store = Arc::clone(&self.store);
        let self_id = self.identity.device_id.clone();
        let members: HashMap<Vec<u8>, String> = match blocking(move || {
            Ok(store
                .sync_devices()?
                .into_iter()
                .filter(|d| !d.removed && d.device_id != self_id)
                .map(|d| (d.public_key, d.device_id))
                .collect())
        })
        .await
        {
            Ok(members) => members,
            Err(e) => {
                log(&format!("couldn't read the group: {e}"));
                return;
            }
        };
        let mut device = None;
        let handshake = timeout(
            HANDSHAKE_TIMEOUT,
            noise::handshake(
                &mut stream,
                SYNC_PARAMS,
                SYNC_PROLOGUE,
                &self.identity.private_key,
                None,
                role,
                |key| {
                    device = members.get(key).cloned();
                    device.is_some()
                },
            ),
        )
        .await
        .unwrap_or(Err(Error::Timeout));
        let transport = match handshake {
            Ok((transport, _)) => transport,
            Err(Error::Refused) => {
                // Kopi on a computer that isn't in the group. Leave it be for a while.
                if let Some(instance) = &instance
                    && let Some(entry) = self.lock().instances.get_mut(instance)
                {
                    entry.retry_at = Instant::now() + STRANGER_RETRY;
                }
                return;
            }
            Err(_) => return,
        };
        let Some(device) = device else {
            return;
        };
        if let Some(instance) = &instance
            && let Some(entry) = self.lock().instances.get_mut(instance)
        {
            entry.device = Some(device.clone());
        }
        let Some((id, stop)) = self.register(&device, role == Role::Initiator) else {
            return;
        };
        self.status_changed();

        let (read, write) = stream.into_split();
        let (sender, receiver) = noise::split(transport, read, write);
        let events = Arc::clone(&self.events);
        let ctx = Context {
            store: Arc::clone(&self.store),
            self_id: self.identity.device_id.clone(),
            peer_id: device.clone(),
            app_version: self.app_version.clone(),
            changes: self.changes.subscribe(),
            stop,
            on_applied: Arc::new(move |applied: Applied| {
                let devices = applied.devices_changed;
                if !applied.is_empty() {
                    events(SyncEvent::Applied(applied));
                }
                if devices {
                    events(SyncEvent::StatusChanged);
                }
            }),
        };
        let result = session::run(ctx, sender, receiver).await;
        let retry = match &result {
            Err(Error::NeedsUpdate) => {
                self.lock().needs_update.insert(device.clone());
                NEEDS_UPDATE_RETRY
            }
            Err(e) => {
                log(&format!("session with {} ended: {e}", short(&device)));
                REDIAL_AFTER
            }
            Ok(()) => REDIAL_AFTER,
        };
        {
            let mut state = self.lock();
            if state
                .sessions
                .get(&device)
                .is_some_and(|entry| entry.id == id)
            {
                state.sessions.remove(&device);
            }
            if let Some(instance) = &instance
                && let Some(entry) = state.instances.get_mut(instance)
            {
                entry.retry_at = Instant::now() + retry;
            }
        }
        self.status_changed();
    }

    /// Records a session, unless one with the same computer should stay instead. Returns its
    /// id and the receiver that ends it.
    fn register(&self, device: &str, dialed_here: bool) -> Option<(u64, watch::Receiver<bool>)> {
        if self.is_stopped() {
            return None;
        }
        let mut state = self.lock();
        // The connection opened by the computer with the lower device ID stays.
        let preferred_dialed_here = self.identity.device_id.as_str() < device;
        if let Some(existing) = state.sessions.get(device) {
            if existing.dialed_here == preferred_dialed_here || dialed_here != preferred_dialed_here
            {
                return None;
            }
            let _ = existing.stop.send(true);
        }
        state.next_session += 1;
        let id = state.next_session;
        let (stop, stopped) = watch::channel(false);
        state.sessions.insert(
            device.to_owned(),
            SessionEntry {
                id,
                dialed_here,
                stop,
            },
        );
        state.needs_update.remove(device);
        Some((id, stopped))
    }

    async fn discovery_loop(self: Arc<Self>, mut found: mpsc::UnboundedReceiver<Found>) {
        let mut stopped = self.stopped.clone();
        loop {
            let next = tokio::select! {
                next = found.recv() => next,
                _ = stopped.changed() => return,
            };
            let Some(next) = next else {
                return;
            };
            let mut status_changed = false;
            let mut dial = false;
            {
                let mut state = self.lock();
                let pairing_open = state.pairing.is_some();
                match next {
                    Found::Resolved {
                        instance,
                        addrs,
                        pairing_name,
                    } if instance != self.own_instance => {
                        match state.instances.get_mut(&instance) {
                            Some(entry) => {
                                status_changed = pairing_open && entry.pairing_name != pairing_name;
                                entry.addrs = addrs;
                                entry.pairing_name = pairing_name;
                            }
                            None => {
                                status_changed = pairing_open && pairing_name.is_some();
                                state.instances.insert(
                                    instance,
                                    Instance {
                                        addrs,
                                        pairing_name,
                                        device: None,
                                        dialing: false,
                                        retry_at: Instant::now(),
                                    },
                                );
                                dial = true;
                            }
                        }
                    }
                    Found::Resolved { .. } => {}
                    Found::Removed { instance } => {
                        if let Some(entry) = state.instances.remove(&instance) {
                            status_changed = pairing_open && entry.pairing_name.is_some();
                        }
                    }
                }
            }
            if status_changed {
                self.status_changed();
            }
            if dial {
                self.dial_now.notify_one();
            }
        }
    }

    async fn dial_loop(self: Arc<Self>) {
        let mut stopped = self.stopped.clone();
        loop {
            tokio::select! {
                _ = tokio::time::sleep(DIAL_EVERY) => {}
                _ = self.dial_now.notified() => {}
                _ = stopped.changed() => return,
            }
            let now = Instant::now();
            let targets: Vec<(String, Vec<SocketAddr>)> = {
                let mut state = self.lock();
                let connected: HashSet<String> = state.sessions.keys().cloned().collect();
                state
                    .instances
                    .iter_mut()
                    .filter(|(_, entry)| {
                        !entry.dialing
                            && entry.retry_at <= now
                            && !entry.addrs.is_empty()
                            && entry.device.as_ref().is_none_or(|d| !connected.contains(d))
                    })
                    .map(|(name, entry)| {
                        entry.dialing = true;
                        (name.clone(), entry.addrs.clone())
                    })
                    .collect()
            };
            for (instance, addrs) in targets {
                tokio::spawn(Arc::clone(&self).dial(instance, addrs));
            }
        }
    }

    async fn dial(self: Arc<Self>, instance: String, addrs: Vec<SocketAddr>) {
        match connect(&addrs).await {
            Some(mut stream) => {
                if stream
                    .write_all(&[SYNC_CONNECTION, PROTOCOL as u8])
                    .await
                    .is_ok()
                {
                    Arc::clone(&self)
                        .sync_session(stream, Role::Initiator, Some(instance.clone()))
                        .await;
                }
            }
            None => {
                if let Some(entry) = self.lock().instances.get_mut(&instance) {
                    entry.retry_at = Instant::now() + UNREACHABLE_RETRY;
                }
            }
        }
        if let Some(entry) = self.lock().instances.get_mut(&instance) {
            entry.dialing = false;
        }
    }

    /// Checks the network permission, and replaces a pairing code that's run out.
    async fn check_loop(self: Arc<Self>) {
        let mut stopped = self.stopped.clone();
        loop {
            let blocked = self.check_network
                && tokio::task::spawn_blocking(probe::local_network_blocked)
                    .await
                    .unwrap_or(false);
            let mut changed = false;
            {
                let mut state = self.lock();
                let problem = match (&state.problem, blocked) {
                    (Some(Problem::Discovery(e)), _) => Some(Problem::Discovery(e.clone())),
                    (_, true) => Some(Problem::LocalNetworkDenied),
                    (_, false) => None,
                };
                if problem != state.problem {
                    if problem == Some(Problem::LocalNetworkDenied) {
                        log("macOS is refusing local network traffic");
                    }
                    state.problem = problem;
                    changed = true;
                }
                if let Some(pairing) = state.pairing.as_mut()
                    && !pairing.busy
                    && pairing.expires_at <= now_ms()
                {
                    pairing.code = pairing::new_code();
                    pairing.expires_at = now_ms() + CODE_LIFETIME_MS;
                    changed = true;
                }
            }
            if changed {
                self.status_changed();
            }
            tokio::select! {
                _ = tokio::time::sleep(CHECK_EVERY) => {}
                _ = stopped.changed() => return,
            }
        }
    }

    /// The other side of [`SyncService::pair`]: someone typed this computer's code.
    async fn pair_respond(self: Arc<Self>, mut stream: TcpStream) {
        let code = {
            let mut state = self.lock();
            match state.pairing.as_mut() {
                Some(pairing) if !pairing.busy => {
                    pairing.busy = true;
                    pairing.code.clone()
                }
                _ => return,
            }
        };
        let mine = self.pair_info();
        let result = timeout(
            PAIRING_TIMEOUT,
            pairing::run(&mut stream, Role::Responder, &code, &self.identity, mine),
        )
        .await
        .unwrap_or(Err(Error::Timeout));
        // One attempt per code, right or wrong.
        if let Some(pairing) = self.lock().pairing.as_mut() {
            pairing.code = pairing::new_code();
            pairing.expires_at = now_ms() + CODE_LIFETIME_MS;
            pairing.busy = false;
        }
        match result {
            Ok((info, key)) => {
                if let Err(e) = self.finish_pairing(info, key).await {
                    log(&format!("pairing failed: {e}"));
                    (self.events)(SyncEvent::PairingFailed {
                        reason: e.to_string(),
                    });
                }
            }
            Err(e) => {
                log(&format!("pairing failed: {e}"));
                (self.events)(SyncEvent::PairingFailed {
                    reason: e.to_string(),
                });
            }
        }
        self.status_changed();
    }

    /// Adds the computer that was just paired to the group.
    async fn finish_pairing(&self, info: PairInfo, key: Vec<u8>) -> Result<String, Error> {
        let os: Platform = info
            .os
            .parse()
            .map_err(|_| Error::Protocol("unknown OS".into()))?;
        if info.device_id == self.identity.device_id {
            return Err(Error::Protocol("that's this computer".into()));
        }
        let store = Arc::clone(&self.store);
        let (id, name) = (info.device_id.clone(), info.name.clone());
        blocking(move || Ok(store.pair_device(&id, &key, &name, os)?)).await?;
        log(&format!("paired with {}", short(&info.device_id)));
        (self.events)(SyncEvent::Paired {
            name: info.name.clone(),
        });
        self.status_changed();
        Ok(info.name)
    }
}

/// The computers nearby in pairing mode, by name.
fn nearby(instances: &HashMap<String, Instance>) -> Vec<Nearby> {
    let mut nearby: Vec<Nearby> = instances
        .iter()
        .filter_map(|(id, entry)| {
            Some(Nearby {
                id: id.clone(),
                name: entry.pairing_name.clone()?,
            })
        })
        .collect();
    nearby.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.id.cmp(&b.id))
    });
    nearby
}

/// Connects to the first address that answers.
async fn connect(addrs: &[SocketAddr]) -> Option<TcpStream> {
    for addr in addrs {
        if let Ok(Ok(stream)) = timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
            return Some(stream);
        }
    }
    None
}

/// The platform this build runs on.
pub(crate) fn local_platform() -> Platform {
    match kopi_core::current_os() {
        kopi_core::Os::Windows => Platform::Windows,
        _ => Platform::MacOs,
    }
}
