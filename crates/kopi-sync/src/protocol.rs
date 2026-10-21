//! The messages computers send each other after the handshake, as CBOR. Fields an older Kopi
//! doesn't know are skipped, so new ones can be added. A new message type, or a change an
//! older Kopi would misread, needs a new [`PROTOCOL`] version.

use kopi_core::storage::sync::Changes;
use serde::{Deserialize, Serialize};

use crate::Error;

/// The protocol version. A computer with a newer one is shown as needing an update.
pub const PROTOCOL: u32 = 1;

/// Every connection starts with one of these bytes, then the protocol version, then Noise.
pub const SYNC_CONNECTION: u8 = b'S';
pub const PAIR_CONNECTION: u8 = b'P';

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Message {
    Hello(Hello),
    Changes(Changes),
    /// The `sync_seq` committed, so the sender can show progress.
    Ack {
        seq: i64,
    },
    /// Sent when nothing else has been for a while. A session silent for longer ends.
    Ping,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hello {
    pub protocol: u32,
    pub app_version: String,
    pub device_id: String,
    /// The highest of the other side's `sync_seq`s applied here, so it sends what's after.
    pub received_seq: i64,
    /// The kinds this side takes, such as `text` and `image`.
    pub kinds: Vec<String>,
    /// How far this side has the other's snippets, if it takes them. Missing from an older
    /// Kopi, which skips snippets, so none are sent to it. The other side first sends the
    /// snippets between this and `received_seq`, which this side doesn't have yet.
    pub snippets_from: Option<i64>,
}

/// What each side tells the other while pairing, once the code is confirmed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PairInfo {
    pub protocol: u32,
    pub device_id: String,
    pub name: String,
    /// `macos` or `windows`.
    pub os: String,
}

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    ciborium::into_writer(value, &mut out).map_err(|e| Error::Protocol(e.to_string()))?;
    Ok(out)
}

pub fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, Error> {
    ciborium::from_reader(bytes).map_err(|e| Error::Protocol(e.to_string()))
}

#[cfg(test)]
mod tests {
    use kopi_core::storage::sync::{SyncClip, SyncFormat};

    use super::*;

    #[test]
    fn round_trips() {
        let changes = Changes {
            clips: vec![SyncClip {
                content_hash: "ab".repeat(32),
                kind: "image".into(),
                formats: vec![SyncFormat {
                    item_index: 0,
                    format: "public.png".into(),
                    data: vec![7; 100_000],
                }],
                ..SyncClip::default()
            }],
            max_seq: 42,
            ..Changes::default()
        };
        let message = Message::Changes(changes);
        let bytes = encode(&message).unwrap();
        // Bytes travel as a byte string, not an array of numbers.
        assert!(bytes.len() < 101_000, "{}", bytes.len());
        assert_eq!(decode::<Message>(&bytes).unwrap(), message);
    }

    #[test]
    fn unknown_fields_are_skipped() {
        #[derive(Serialize)]
        struct NewerHello {
            protocol: u32,
            device_id: String,
            something_new: Vec<u32>,
        }
        #[derive(Serialize)]
        enum NewerMessage {
            Hello(NewerHello),
        }
        let bytes = encode(&NewerMessage::Hello(NewerHello {
            protocol: 2,
            device_id: "d".into(),
            something_new: vec![1, 2],
        }))
        .unwrap();
        let Message::Hello(hello) = decode(&bytes).unwrap() else {
            panic!("not a hello");
        };
        assert_eq!(hello.protocol, 2);
        assert_eq!(hello.device_id, "d");
        // An older Kopi's hello doesn't ask for snippets.
        assert_eq!(hello.snippets_from, None);
    }
}
