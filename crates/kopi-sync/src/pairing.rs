//! Pairing two computers with a 6-digit code.
//!
//! The code is short, so it isn't used as a key. Both sides run it through SPAKE2, which turns
//! it into a strong shared key and gives an attacker one guess per attempt. That key is mixed
//! into a Noise `XXpsk3` handshake, which swaps the two static keys: a wrong code fails the
//! handshake. Then each side sends its device ID, name and OS. The computer showing the code
//! makes a new one after every attempt, right or wrong.

use spake2::{Ed25519Group, Identity as SpakeIdentity, Password, Spake2};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;

use crate::Error;
use crate::identity::Identity;
use crate::noise::{self, PAIR_PARAMS, PAIR_PROLOGUE, Role, read_frame, write_frame};
use crate::protocol::{self, PROTOCOL, PairInfo};

/// How many digits a code has.
pub const CODE_DIGITS: usize = 6;

/// A random code of [`CODE_DIGITS`] digits, each equally likely.
pub fn new_code() -> String {
    let mut code = String::with_capacity(CODE_DIGITS);
    while code.len() < CODE_DIGITS {
        let mut byte = [0u8; 1];
        getrandom::fill(&mut byte).expect("the OS has randomness");
        // 250 is the largest multiple of 10 that fits, so every digit is as likely.
        if byte[0] < 250 {
            code.push(char::from(b'0' + byte[0] % 10));
        }
    }
    code
}

/// The digits of a code as typed, without spaces. `None` if it isn't [`CODE_DIGITS`] digits.
pub fn normalize_code(typed: &str) -> Option<String> {
    let digits: String = typed.chars().filter(|c| !c.is_whitespace()).collect();
    (digits.len() == CODE_DIGITS && digits.bytes().all(|b| b.is_ascii_digit())).then_some(digits)
}

/// Pairs over `stream`, from either side, and returns what the other computer sent and its
/// static key. `code` is the code shown on the computer that's responding.
pub(crate) async fn run(
    stream: &mut TcpStream,
    role: Role,
    code: &str,
    identity: &Identity,
    mine: PairInfo,
) -> Result<(PairInfo, Vec<u8>), Error> {
    let key = spake(stream, code).await?;
    let (transport, their_key) = noise::handshake(
        stream,
        PAIR_PARAMS,
        PAIR_PROLOGUE,
        &identity.private_key,
        Some(&key),
        role,
        |_| true,
    )
    .await
    .map_err(|e| match e {
        // Almost always a wrong code: the handshake can't decrypt.
        Error::Noise(_) => Error::WrongCode,
        e => e,
    })?;
    let (read, write) = stream.split();
    let (mut sender, mut receiver) = noise::split(transport, read, write);
    let mine = protocol::encode(&mine)?;
    let theirs = match role {
        Role::Initiator => {
            sender.send(&mine).await?;
            receiver.recv().await?
        }
        Role::Responder => {
            let theirs = receiver.recv().await?;
            sender.send(&mine).await?;
            theirs
        }
    };
    let theirs: PairInfo = protocol::decode(&theirs)?;
    if theirs.protocol > PROTOCOL {
        return Err(Error::NeedsUpdate);
    }
    Ok((theirs, their_key))
}

/// Runs SPAKE2 with the code, and returns the shared key.
async fn spake<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    code: &str,
) -> Result<[u8; 32], Error> {
    let (state, outbound) = Spake2::<Ed25519Group>::start_symmetric(
        &Password::new(code.as_bytes()),
        &SpakeIdentity::new(PAIR_PROLOGUE),
    );
    write_frame(stream, &outbound).await?;
    let inbound = read_frame(stream).await?;
    let key = state
        .finish(&inbound)
        .map_err(|e| Error::Protocol(format!("pairing: {e:?}")))?;
    key.get(..32)
        .and_then(|k| k.try_into().ok())
        .ok_or_else(|| Error::Protocol("pairing: short key".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_six_digits() {
        for _ in 0..100 {
            let code = new_code();
            assert_eq!(normalize_code(&code).as_deref(), Some(code.as_str()));
        }
        assert_eq!(normalize_code("482 913").as_deref(), Some("482913"));
        assert_eq!(normalize_code("48291"), None);
        assert_eq!(normalize_code("48291a"), None);
        assert_eq!(normalize_code("４８２９１３"), None);
    }

    async fn attempt(
        code_a: &str,
        code_b: &str,
    ) -> (Result<PairInfo, Error>, Result<PairInfo, Error>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (a, b) = (fake_identity("a"), fake_identity("b"));
        let info = |id: &str| PairInfo {
            protocol: PROTOCOL,
            device_id: id.into(),
            name: format!("{id} computer"),
            os: "macos".into(),
        };
        let (ia, ib) = (info("a"), info("b"));
        let code_a = code_a.to_owned();
        let responder = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            run(&mut stream, Role::Responder, &code_a, &a, ia)
                .await
                .map(|(info, _)| info)
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        let initiator = run(&mut stream, Role::Initiator, code_b, &b, ib)
            .await
            .map(|(info, _)| info);
        drop(stream);
        (responder.await.unwrap(), initiator)
    }

    fn fake_identity(id: &str) -> Identity {
        let pair = noise::generate_keypair();
        Identity {
            device_id: id.into(),
            private_key: pair.private,
            public_key: pair.public,
        }
    }

    #[tokio::test]
    async fn the_right_code_pairs() {
        let (responder, initiator) = attempt("482913", "482913").await;
        assert_eq!(responder.unwrap().device_id, "b");
        assert_eq!(initiator.unwrap().name, "a computer");
    }

    #[tokio::test]
    async fn a_wrong_code_fails_on_both_sides() {
        let (responder, initiator) = attempt("482913", "482914").await;
        assert!(responder.is_err());
        assert!(initiator.is_err());
    }
}
