//! Telling a denied Local Network permission from an empty network.
//!
//! mdns-sd doesn't report failed sends, so without this a "Don't Allow" on macOS would look
//! like no other computers. Sending one multicast packet from a socket of Kopi's own shows it:
//! macOS refuses with "No route to host".

/// Whether macOS is refusing Kopi's local network traffic. Always `false` elsewhere.
pub fn local_network_blocked() -> bool {
    #[cfg(target_os = "macos")]
    {
        // EHOSTUNREACH on macOS.
        const NO_ROUTE_TO_HOST: i32 = 65;
        let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") else {
            return false;
        };
        matches!(
            socket.send_to(&query(), "224.0.0.251:5353"),
            Err(e) if e.raw_os_error() == Some(NO_ROUTE_TO_HOST)
        )
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// An mDNS query for `_kopi._tcp.local` PTR records, which every responder can ignore.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn query() -> Vec<u8> {
    // ID 0, no flags, one question.
    let mut packet = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in ["_kopi", "_tcp", "local"] {
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    // The root, then type PTR and class IN.
    packet.extend_from_slice(&[0, 0, 12, 0, 1]);
    packet
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_query_is_well_formed() {
        let q = super::query();
        assert_eq!(q.len(), 12 + 6 + 5 + 6 + 1 + 4);
        assert_eq!(&q[q.len() - 4..], &[0, 12, 0, 1]);
    }
}
