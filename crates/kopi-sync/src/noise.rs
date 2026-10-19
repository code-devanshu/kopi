//! Noise over TCP: the handshake, then messages of up to [`MAX_MESSAGE`] bytes.
//!
//! Each Noise message has a 2-byte length and carries at most 65,519 bytes. A protocol message
//! starts with its own 4-byte length and is split across as many Noise messages as it needs.
//! After the handshake the connection splits into a [`Sender`] and a [`Receiver`], each with
//! its own nonce, so one task can write while another reads.

use std::sync::Arc;

use snow::{HandshakeState, StatelessTransportState};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::Error;

/// Sessions between paired computers. XX sends both static keys encrypted, and the
/// responder's arrives first, so the initiator checks it before revealing its own.
pub const SYNC_PARAMS: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
/// Pairing: XX with the key from SPAKE2 mixed in, so a wrong code fails the handshake.
pub const PAIR_PARAMS: &str = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s";
pub const SYNC_PROLOGUE: &[u8] = b"kopi-sync-v1";
pub const PAIR_PROLOGUE: &[u8] = b"kopi-pair-v1";

/// The largest protocol message. A session that's sent a larger one ends. A batch stops
/// growing at 8 MiB of clips, but one clip can be up to the 50 MiB capture limit, and a clip
/// that comes with a pinboard item can join a batch that's nearly full.
pub const MAX_MESSAGE: usize = 96 * 1024 * 1024;

const MAX_NOISE: usize = 65535;
const TAG: usize = 16;
const MAX_CHUNK: usize = MAX_NOISE - TAG;

pub fn generate_keypair() -> snow::Keypair {
    snow::Builder::new(SYNC_PARAMS.parse().expect("valid Noise params"))
        .generate_keypair()
        .expect("the OS has randomness")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Initiator,
    Responder,
}

/// Runs the handshake on `stream`. `accept` gets the other side's static key as soon as it's
/// known, and returning `false` ends the handshake before anything more is sent. Returns the
/// transport state and the other side's static key.
pub async fn handshake<S, F>(
    stream: &mut S,
    params: &str,
    prologue: &[u8],
    private_key: &[u8],
    psk: Option<&[u8; 32]>,
    role: Role,
    mut accept: F,
) -> Result<(StatelessTransportState, Vec<u8>), Error>
where
    S: AsyncRead + AsyncWrite + Unpin,
    F: FnMut(&[u8]) -> bool,
{
    let mut builder = snow::Builder::new(params.parse().map_err(noise)?)
        .local_private_key(private_key)
        .map_err(noise)?
        .prologue(prologue)
        .map_err(noise)?;
    if let Some(psk) = psk {
        builder = builder.psk(3, psk).map_err(noise)?;
    }
    let mut hs: HandshakeState = match role {
        Role::Initiator => builder.build_initiator(),
        Role::Responder => builder.build_responder(),
    }
    .map_err(noise)?;

    let mut buf = vec![0u8; MAX_NOISE];
    let mut payload = vec![0u8; MAX_NOISE];
    let mut checked = false;
    let mut my_turn = role == Role::Initiator;
    while !hs.is_handshake_finished() {
        if my_turn {
            let n = hs.write_message(&[], &mut buf).map_err(noise)?;
            write_frame(stream, &buf[..n]).await?;
        } else {
            let frame = read_frame(stream).await?;
            hs.read_message(&frame, &mut payload).map_err(noise)?;
            if !checked && let Some(key) = hs.get_remote_static() {
                checked = true;
                if !accept(key) {
                    return Err(Error::Refused);
                }
            }
        }
        my_turn = !my_turn;
    }
    let key = hs
        .get_remote_static()
        .ok_or_else(|| Error::Protocol("no static key".into()))?
        .to_vec();
    if !checked && !accept(&key) {
        return Err(Error::Refused);
    }
    let transport = hs.into_stateless_transport_mode().map_err(noise)?;
    Ok((transport, key))
}

/// Splits a connection whose handshake is done.
pub fn split<R, W>(
    transport: StatelessTransportState,
    read: R,
    write: W,
) -> (Sender<W>, Receiver<R>) {
    let transport = Arc::new(transport);
    (
        Sender {
            write,
            transport: Arc::clone(&transport),
            nonce: 0,
            out: vec![0u8; MAX_NOISE],
        },
        Receiver {
            read,
            transport,
            nonce: 0,
            buf: vec![0u8; MAX_NOISE],
        },
    )
}

pub struct Sender<W> {
    write: W,
    transport: Arc<StatelessTransportState>,
    nonce: u64,
    out: Vec<u8>,
}

impl<W: AsyncWrite + Unpin> Sender<W> {
    pub async fn send(&mut self, message: &[u8]) -> Result<(), Error> {
        if message.len() > MAX_MESSAGE {
            return Err(Error::TooLarge(message.len()));
        }
        let mut plain = Vec::with_capacity(message.len() + 4);
        plain.extend_from_slice(&(message.len() as u32).to_be_bytes());
        plain.extend_from_slice(message);
        for chunk in plain.chunks(MAX_CHUNK) {
            let n = self
                .transport
                .write_message(self.nonce, chunk, &mut self.out)
                .map_err(noise)?;
            self.nonce += 1;
            write_frame(&mut self.write, &self.out[..n]).await?;
        }
        self.write.flush().await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) {
        let _ = self.write.shutdown().await;
    }
}

pub struct Receiver<R> {
    read: R,
    transport: Arc<StatelessTransportState>,
    nonce: u64,
    buf: Vec<u8>,
}

impl<R: AsyncRead + Unpin> Receiver<R> {
    pub async fn recv(&mut self) -> Result<Vec<u8>, Error> {
        let first = self.chunk().await?;
        let Some(len) = first.get(..4) else {
            return Err(Error::Protocol("short message".into()));
        };
        let len = u32::from_be_bytes(len.try_into().expect("4 bytes")) as usize;
        if len > MAX_MESSAGE {
            return Err(Error::TooLarge(len));
        }
        let mut message = Vec::with_capacity(len);
        message.extend_from_slice(&first[4..]);
        while message.len() < len {
            let chunk = self.chunk().await?;
            message.extend_from_slice(&chunk);
        }
        if message.len() != len {
            return Err(Error::Protocol("message longer than it said".into()));
        }
        Ok(message)
    }

    async fn chunk(&mut self) -> Result<Vec<u8>, Error> {
        let frame = read_frame(&mut self.read).await?;
        let n = self
            .transport
            .read_message(self.nonce, &frame, &mut self.buf)
            .map_err(noise)?;
        self.nonce += 1;
        Ok(self.buf[..n].to_vec())
    }
}

pub(crate) async fn write_frame<W: AsyncWrite + Unpin>(
    w: &mut W,
    bytes: &[u8],
) -> Result<(), Error> {
    let len = u16::try_from(bytes.len()).map_err(|_| Error::TooLarge(bytes.len()))?;
    let mut out = Vec::with_capacity(bytes.len() + 2);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
    w.write_all(&out).await?;
    Ok(())
}

pub(crate) async fn read_frame<R: AsyncRead + Unpin>(r: &mut R) -> Result<Vec<u8>, Error> {
    let mut len = [0u8; 2];
    r.read_exact(&mut len).await?;
    let mut frame = vec![0u8; u16::from_be_bytes(len) as usize];
    r.read_exact(&mut frame).await?;
    Ok(frame)
}

fn noise(e: snow::Error) -> Error {
    Error::Noise(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pair(
        a_key: &[u8],
        b_key: &[u8],
        b_accepts: bool,
    ) -> (
        Result<(StatelessTransportState, Vec<u8>), Error>,
        Result<(StatelessTransportState, Vec<u8>), Error>,
    ) {
        let (mut a, mut b) = tokio::io::duplex(1 << 20);
        let (a_key, b_key) = (a_key.to_vec(), b_key.to_vec());
        let a = tokio::spawn(async move {
            handshake(
                &mut a,
                SYNC_PARAMS,
                SYNC_PROLOGUE,
                &a_key,
                None,
                Role::Initiator,
                |_| true,
            )
            .await
        });
        let b = tokio::spawn(async move {
            handshake(
                &mut b,
                SYNC_PARAMS,
                SYNC_PROLOGUE,
                &b_key,
                None,
                Role::Responder,
                |_| b_accepts,
            )
            .await
        });
        (a.await.unwrap(), b.await.unwrap())
    }

    #[tokio::test]
    async fn large_messages_survive_the_trip() {
        let (a_pair, b_pair) = (generate_keypair(), generate_keypair());
        let (mut a, mut b) = tokio::io::duplex(1 << 20);
        let (ak, bk) = (a_pair.private.clone(), b_pair.private.clone());
        let ta = tokio::spawn(async move {
            let (t, key) = handshake(
                &mut a,
                SYNC_PARAMS,
                SYNC_PROLOGUE,
                &ak,
                None,
                Role::Initiator,
                |_| true,
            )
            .await
            .unwrap();
            (t, key, a)
        });
        let (t, key_a_saw) = handshake(
            &mut b,
            SYNC_PARAMS,
            SYNC_PROLOGUE,
            &bk,
            None,
            Role::Responder,
            |_| true,
        )
        .await
        .unwrap();
        let (ta, key_b_saw, a) = ta.await.unwrap();
        assert_eq!(key_a_saw, a_pair.public);
        assert_eq!(key_b_saw, b_pair.public);

        let (ar, aw) = tokio::io::split(a);
        let (br, bw) = tokio::io::split(b);
        let (mut a_send, _a_recv) = split(ta, ar, aw);
        let (_b_send, mut b_recv) = split(t, br, bw);
        let big: Vec<u8> = (0..3_000_000u32).map(|i| (i % 251) as u8).collect();
        let sent = big.clone();
        let writer = tokio::spawn(async move {
            a_send.send(b"").await.unwrap();
            a_send.send(&sent).await.unwrap();
            a_send.send(b"after").await.unwrap();
        });
        assert_eq!(b_recv.recv().await.unwrap(), b"");
        assert_eq!(b_recv.recv().await.unwrap(), big);
        assert_eq!(b_recv.recv().await.unwrap(), b"after");
        writer.await.unwrap();
    }

    #[tokio::test]
    async fn a_refused_key_ends_the_handshake() {
        let (a, b) = (generate_keypair(), generate_keypair());
        let (_, rb) = pair(&a.private, &b.private, false).await;
        assert!(matches!(rb, Err(Error::Refused)));
    }

    #[tokio::test]
    async fn a_wrong_psk_fails() {
        let (a, b) = (generate_keypair(), generate_keypair());
        let (mut x, mut y) = tokio::io::duplex(1 << 20);
        let a_task = tokio::spawn(async move {
            handshake(
                &mut x,
                PAIR_PARAMS,
                PAIR_PROLOGUE,
                &a.private,
                Some(&[1; 32]),
                Role::Initiator,
                |_| true,
            )
            .await
        });
        let rb = handshake(
            &mut y,
            PAIR_PARAMS,
            PAIR_PROLOGUE,
            &b.private,
            Some(&[2; 32]),
            Role::Responder,
            |_| true,
        )
        .await;
        assert!(rb.is_err());
        // The initiator only sees the connection end.
        drop(y);
        let _ = a_task.await;
    }
}
