# 0005. Local-first now, end-to-end encrypted sync later

- **Status:** Accepted. Partly superseded by [ADR 0007](0007-lan-sync.md), which designs sync over the local network and brings it before 1.0.
- **Date:** 2026-10-11

## Context

Mac ↔ Windows sync is a headline feature of Kopi's plan, and it's the hardest part to get right: encryption, key management, device pairing, conflicts and a server to run. Clipboard data is about the most sensitive data a desktop app handles. Designing sync before the local app works would slow the alpha down and risk a rushed security design.

## Decision

- The alpha and 1.0 are **local-only**: no accounts, no network calls (apart from the updater's signed manifest check, added later).
- Sync is designed **after the Windows alpha**, in the open, as its own document and ADR. It must be **end-to-end encrypted**, with keys only on the user's devices, and opt-in.
- Today's data model keeps sync possible without a rewrite: UUIDv7 IDs on synced entities, content hashes for cross-device dedupe, and `updated_at` timestamps. Tombstones, device IDs and logical clocks are added with the sync design, not before.

## Alternatives considered

| Option | Why not (now) |
| --- | --- |
| Design sync up front | Delays the alpha for something users can't try until both platforms exist. We'd also be designing against guesses instead of a working local model. |
| Use the OS clouds (iCloud, Microsoft account) | Different on each OS, so there's no Mac ↔ Windows path, and we can't guarantee end-to-end encryption. |
| Server-side encrypted sync (keys on the server) | Not acceptable for clipboard data. It contradicts the privacy promise on the website. |

## Consequences

- **Good:** a smaller, faster alpha, and privacy is easy to verify ("it never connects to anything").
- **Bad:** some schema migrations will come when sync lands. Users who want sync wait until after 1.0, as the roadmap already says.
- **Commitment:** nothing in the local design may assume clips stay on one device forever in a way that would block encrypted sync later. Reviewers should check this in storage PRs.
