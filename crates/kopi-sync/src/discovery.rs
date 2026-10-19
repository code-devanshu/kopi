//! Finding other computers running Kopi with DNS-SD over mDNS, as `_kopi._tcp.local.`.
//!
//! The instance name is random and new every time sync starts, and the host name is made from
//! it, so neither says which computer this is. The TXT record holds the protocol version, and
//! while the pairing sheet is open, `pair=1` and this computer's name, so others can list it.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::sync::mpsc;

use crate::Error;
use crate::protocol::PROTOCOL;

pub const SERVICE_TYPE: &str = "_kopi._tcp.local.";

/// What browsing found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// A computer advertising Kopi, or a change to what it advertises.
    Resolved {
        instance: String,
        addrs: Vec<SocketAddr>,
        /// Its name, while it's in pairing mode.
        pairing_name: Option<String>,
    },
    Removed {
        instance: String,
    },
}

pub struct Discovery {
    daemon: ServiceDaemon,
    instance: String,
    port: u16,
    fullname: Option<String>,
}

impl Discovery {
    /// Starts the mDNS responder and browses for other computers. What it finds goes to `found`.
    pub fn start(port: u16, found: mpsc::UnboundedSender<Found>) -> Result<Self, Error> {
        let daemon = ServiceDaemon::new().map_err(mdns)?;
        let instance = random_instance();
        let events = daemon.browse(SERVICE_TYPE).map_err(mdns)?;
        std::thread::Builder::new()
            .name("kopi-sync-browse".into())
            .spawn(move || {
                // Ends when the daemon shuts down, or nobody listens any more.
                while let Ok(event) = events.recv() {
                    let found_one = match event {
                        ServiceEvent::ServiceResolved(info) => Found::Resolved {
                            instance: info.fullname.clone(),
                            addrs: order(
                                info.addresses
                                    .iter()
                                    .map(|ip| SocketAddr::new(ip.to_ip_addr(), info.port)),
                            ),
                            pairing_name: (info.txt_properties.get_property_val_str("pair")
                                == Some("1"))
                            .then(|| info.txt_properties.get_property_val_str("name"))
                            .flatten()
                            .map(str::to_owned),
                        },
                        ServiceEvent::ServiceRemoved(_, fullname) => {
                            Found::Removed { instance: fullname }
                        }
                        _ => continue,
                    };
                    if found.send(found_one).is_err() {
                        break;
                    }
                }
            })?;
        let mut discovery = Self {
            daemon,
            instance,
            port,
            fullname: None,
        };
        discovery.advertise(None)?;
        Ok(discovery)
    }

    /// This computer's instance name, to skip itself in what browsing finds.
    pub fn own_fullname(&self) -> String {
        format!("{}.{SERVICE_TYPE}", self.instance)
    }

    /// Advertises this computer, with its name while `pairing_name` is set.
    pub fn advertise(&mut self, pairing_name: Option<&str>) -> Result<(), Error> {
        if let Some(fullname) = self.fullname.take() {
            // Waits for nothing: the goodbye goes out in the background.
            let _ = self.daemon.unregister(&fullname);
        }
        let mut txt: HashMap<String, String> = HashMap::new();
        txt.insert("v".into(), PROTOCOL.to_string());
        if let Some(name) = pairing_name {
            txt.insert("pair".into(), "1".into());
            txt.insert("name".into(), name.to_owned());
        }
        let host = format!("{}.local.", self.instance);
        let info = ServiceInfo::new(SERVICE_TYPE, &self.instance, &host, "", self.port, txt)
            .map_err(mdns)?
            .enable_addr_auto();
        self.fullname = Some(info.get_fullname().to_owned());
        self.daemon.register(info).map_err(mdns)?;
        Ok(())
    }

    pub fn stop(self) {
        if let Some(fullname) = &self.fullname
            && let Ok(done) = self.daemon.unregister(fullname)
        {
            // The goodbye packet, so others drop this computer at once.
            let _ = done.recv_timeout(Duration::from_millis(500));
        }
        if let Ok(done) = self.daemon.shutdown() {
            let _ = done.recv_timeout(Duration::from_secs(1));
        }
    }
}

/// IPv4 first, then IPv6 that isn't link-local, which would need a scope to dial.
fn order(addrs: impl Iterator<Item = SocketAddr>) -> Vec<SocketAddr> {
    let mut addrs: Vec<SocketAddr> = addrs
        .filter(|a| match a.ip() {
            IpAddr::V4(ip) => !ip.is_loopback() && !ip.is_unspecified(),
            IpAddr::V6(ip) => !ip.is_loopback() && (ip.segments()[0] & 0xffc0) != 0xfe80,
        })
        .collect();
    addrs.sort_by_key(|a| (a.is_ipv6(), *a));
    addrs.dedup();
    addrs
}

/// 16 random hex characters.
fn random_instance() -> String {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("the OS has randomness");
    format!("kopi-{}", crate::identity::hex(&bytes))
}

fn mdns(e: mdns_sd::Error) -> Error {
    Error::Discovery(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialable_addresses_ipv4_first() {
        let addrs = order(
            [
                "[fe80::1]:5",
                "[2001:db8::1]:5",
                "192.168.1.9:5",
                "127.0.0.1:5",
                "192.168.1.9:5",
            ]
            .into_iter()
            .map(|a| a.parse().unwrap()),
        );
        let addrs: Vec<String> = addrs.iter().map(ToString::to_string).collect();
        assert_eq!(addrs, ["192.168.1.9:5", "[2001:db8::1]:5"]);
    }
}
