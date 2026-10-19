# 0007. Sync directly over the local network

- **Status:** Accepted
- **Date:** 2026-10-20
- **Partly supersedes:** [ADR 0005](0005-local-first-sync-later.md), on when sync comes and how it travels

## Context

[ADR 0005](0005-local-first-sync-later.md) kept the alpha local-only and said sync would be designed after the Windows alpha, end-to-end encrypted, with keys only on the user's devices. The Windows alpha has shipped, so the design is due. The full design is in [SYNC_DESIGN.md](../SYNC_DESIGN.md).

The biggest choice is how clips travel. Clipboard history is about the most sensitive data a desktop app holds. Kopi is free, has no income, and runs no servers apart from the static website and GitHub releases.

## Decision

- **Transport: the local network only.** Computers find each other with mDNS (`_kopi._tcp`) and connect directly over TCP. There's no server, relay or account. Both computers must be on the same network at the same time, and a computer that was away catches up when it's back.
- **Pairing:** a 6-digit code shown on one computer and typed on the other, through SPAKE2. Paired computers share the group, so a new one pairs with any member once.
- **Encryption:** every session is `Noise_XX_25519_ChaChaPoly_BLAKE2s`, with each computer's static key pinned at pairing. Keys never leave the computer.
- **Data model:** state-based CRDTs. Last-writer-wins by hybrid logical clock, clips identified by `content_hash`, tombstones for deletes and Clear history, kept for 90 days. Each computer tracks changes with a local counter and remembers how far it got from each peer.
- **Scope:** history (text, rich text, links, code, colors, images), pins and pinboards. File clips, local-only clips, settings and excluded apps don't sync. Limits stay per computer.
- **Off by default.** Nothing listens or advertises until the user turns sync on.
- It ships in an alpha once it's built and tested, rather than waiting for 1.0.

## Alternatives considered

| Option | Why not |
| --- | --- |
| A Kopi relay (for example Cloudflare Workers and Durable Objects) that stores and forwards ciphertext | Works away from home, and is end-to-end encrypted. But someone has to run, pay for and look after it, and it adds an account or device registry, abuse handling and an outage that's ours. The merge layer doesn't depend on the transport, so a relay can be added later with its own ADR. |
| A folder the user picks in Dropbox, OneDrive, Google Drive or iCloud Drive | No server for us, but delivery takes seconds to minutes and depends on that app, the services create conflicted copies, and it leaves encrypted history in a third-party cloud. |
| The OS clouds (iCloud, Microsoft account) | Ruled out in ADR 0005: no Mac ↔ Windows path, and no end-to-end encryption we control. |
| TLS with pinned self-signed certificates | Works, but certificate handling adds code for nothing Noise doesn't give, and Noise XX hides both keys from a passive observer. |
| libp2p | Discovery, NAT traversal and many transports we don't need, for a large dependency tree. |
| An operation log replayed in order | Needs every operation kept until every peer has it, and is fragile when computers come and go. State-based merge needs only the latest row and a tombstone. |
| A CRDT database extension (cr-sqlite) | Would replace much of the storage layer, interacts with the FTS5 triggers, and its maintenance is uncertain. Kopi's data is a few tables with simple rules. |

## Consequences

- **Good:** no server, account, cost or third party. Clips never leave the user's network. Deletes reach every computer. A Mac and a Windows PC can share history.
- **Bad:** no sync away from home or across networks that isolate devices, such as guest Wi-Fi and many offices.
- **Bad:** macOS asks for the Local Network permission, and Windows Defender Firewall asks to allow Kopi, which needs an administrator on a standard account. Spike 0004 checks both, and whether the macOS grant survives updates with the self-signed certificate.
- **Bad:** schema migration `0004` adds change tracking to every write, whether sync is on or not.
- **Commitment:** received data is untrusted input, and is validated as strictly as a local capture.
