# Sync design

> [!NOTE]
> **Status: accepted.** Nothing here is built yet. This is the design that [ADR 0005](decisions/0005-local-first-sync-later.md) promised to do in the open, and [ADR 0007](decisions/0007-lan-sync.md) records the decision. To comment, open a [discussion](https://github.com/code-devanshu/kopi/discussions) or a pull request against this file.

Kopi syncs history, pins and pinboards between a user's own computers, Mac and Windows in any mix, **directly over the local network**. There's no server, no account and no cloud. Computers find each other with Bonjour (mDNS), are paired once with a code, and talk over a connection that's encrypted end to end and authenticated with keys that never leave them.

Related docs: [DATA_MODEL.md](DATA_MODEL.md), [PRIVACY_DESIGN.md](PRIVACY_DESIGN.md), [PLATFORM_NOTES.md](PLATFORM_NOTES.md).

## Goals

- **Private.** Clips only ever travel between the user's paired computers, encrypted and authenticated. No server sees them, because there is no server.
- **Mac ↔ Windows.** Any mix of macOS and Windows computers, two or more.
- **Full history.** Text, rich text, links, code, colors and images, plus pins and pinboards.
- **Converges.** However often computers go offline, and in whatever order changes arrive, every computer ends with the same history. Deleting a clip deletes it everywhere.
- **Opt-in and quiet.** Off by default. While it's off, Kopi opens no sockets and advertises nothing.

## Not goals (for this version)

- **Syncing away from home.** Both computers must be on the same network at the same time. A computer that was away catches up when it's back. A relay for syncing over the internet could be added later as another transport (see [Later](#later)).
- **File clips.** A file clip is a list of paths, and `/Users/me/report.pdf` points at nothing on another computer. File clips stay on the computer they were copied on. Sending the files themselves would be a different feature.
- **Settings and excluded apps.** Each computer keeps its own. Shortcuts, limits and exclusions often differ by machine, and exclusions name apps by platform-specific IDs.
- **Setting the other computer's clipboard.** A synced clip is added to history. It doesn't replace what's on the clipboard over there, so nothing changes under the user's cursor. A "universal clipboard" option can come later.
- **Mobile.** Kopi has no mobile app.

## What the user sees

**Settings → Sync** (new section):

- **Sync with your other computers**: a switch, off by default. Its description: "Over your local network only. Clips go straight between your computers, encrypted, and never through a server."
- **This computer's name**: defaults to the computer's name, editable. Only paired computers see it.
- **Computers**: each paired computer with its OS and status: *Connected*, *Last synced 2 hours ago*, or *Not seen for 90 days* (see [Tombstones](#deletes-and-tombstones)). Each has **Remove**.
- **Add a computer…**: opens the pairing sheet.
- **What to sync**: a checkbox per kind, all on. File clips are listed, disabled, with "File clips stay on the computer they were copied on."

**Pairing**, done once per computer:

1. On the Mac, the user opens **Add a computer…**. The sheet shows a 6-digit code, such as `482 913`, and lists nearby computers that are in pairing mode.
2. On the Windows PC, they open **Add a computer…** too, pick the Mac from its list, and type the code shown on the Mac.
3. Both show "Paired with *Windows PC*" and start syncing. A third computer only needs to pair with one of them, because computers share who's in the group.

A wrong code ends the attempt and the Mac shows a new code. Codes expire after 5 minutes, or when the sheet closes.

**In the panel**, a clip copied on another computer shows "Copied on *Windows PC*" in the preview, next to the source app. Otherwise synced clips look and paste like any other.

## Threat model

Protected against:

- **Anyone on the network**, passive or active: they can't read clips, change them, inject clips, or pose as a paired computer.
- **Unpaired computers running Kopi**: they can't connect to a paired one or learn anything about its history.
- **A malicious peer's data**: everything received is treated as untrusted input. It's size-limited, validated, and content hashes are recomputed (see [Receiving](#receiving)).

Not protected against:

- **A paired computer that's compromised**: it can read and change the synced history, and add or remove computers. Sync can't do better than the computers in it.
- **Malware or someone at an unlocked computer**: they can read `kopi.db` already, sync or not ([PRIVACY_DESIGN.md](PRIVACY_DESIGN.md)).
- **Seeing that Kopi is running**: while sync is on, a computer advertises a Kopi service on the network, and anyone who connects can learn its public key, which is a stable identifier. See [Discovery](#discovery) and [Open questions](#open-questions).

## Devices and keys

Each computer, once sync is first turned on, creates:

- A **device ID**: a UUIDv7.
- A **static key pair**: X25519, used by the Noise handshake. The private key never leaves the computer.

Both are kept in `<data dir>/sync/device.key`, readable only by the user (`0600` on macOS, the user's ACL on Windows). Not the Keychain or Credential Manager: the key protects nothing that `kopi.db`, beside it, doesn't already expose, and keychain items on a self-signed app risk an access prompt after updates. Turning sync off and choosing **Forget this computer's sync identity** moves the file to the Trash, and paired computers then see it as removed.

A **sync group** is the set of computers the user has paired, kept on each of them as the `sync_devices` table: device ID, public key, name, OS, and when it was added or removed. The group is synced like any other data, so pairing with one member introduces a computer to all of them.

## Discovery

Computers advertise and browse a DNS-SD service over mDNS, `_kopi._tcp.local.`, only while sync is on.

- **Instance name**: random, new every launch. It never includes the computer's name.
- **TXT record**: `v=1`, the protocol version. While the pairing sheet is open, also `pair=1` and `name=<this computer's name>`, so the other computer can list it. Both go when the sheet closes.
- **Port**: chosen by the OS when Kopi starts listening.

A computer connects to every `_kopi._tcp` instance it finds and runs the Noise handshake. The handshake reveals the other side's public key first, and the connection is dropped before anything else is sent if that key isn't in the group. When two computers dial each other at once, the connection opened by the lower device ID wins and the other is closed.

Which mDNS implementation to use is a spike question: the pure-Rust [`mdns-sd`](https://crates.io/crates/mdns-sd) crate, or the system's own (Bonjour's `dns_sd` on macOS, `DnsServiceRegister`/`DnsServiceBrowse` on Windows 10+), which avoids sharing port 5353 with the system responder. See [Open questions](#open-questions).

## Pairing

The 6-digit code is short, so it can't be used as a key directly. It's run through a **PAKE**, SPAKE2 ([`spake2`](https://crates.io/crates/spake2)), which turns a short code into a strong shared key and gives an attacker one guess per attempt. Kopi allows one attempt per code, so the chance of guessing right is one in a million, and a failed attempt is visible to the user.

1. The computer showing the code (A) listens with `pair=1`. The other (B) connects to it.
2. They run SPAKE2 with the code as the password and `kopi-pair-v1` as the identity. Each sends a key-confirmation MAC, and a wrong code fails here.
3. Encrypted with the SPAKE2 key, B sends its device ID, public key, name and OS. A replies with the same, plus the group: every member's device ID, public key, name and OS.
4. Each adds the other to `sync_devices`. A's members learn about B the next time they sync with A, and B connects to them directly.

## Sessions

After pairing, every connection uses the [Noise Protocol Framework](https://noiseprotocol.org/noise.html) pattern `Noise_XX_25519_ChaChaPoly_BLAKE2s` through the [`snow`](https://crates.io/crates/snow) crate, over TCP.

- **XX** sends both static keys encrypted. The responder's key arrives first, so the initiator checks it against the group before revealing its own.
- **Both keys are pinned.** Each side refuses a static key that isn't in its group and not removed.
- **Prologue**: `kopi-sync-v1`, so a handshake can't be replayed into another protocol.
- Noise transport messages are at most 65,535 bytes, each sent with a 2-byte length prefix. A protocol message starts with its 4-byte length and is split across as many Noise messages as it needs.

## What syncs, and how conflicts resolve

The sync layer is **state-based**: computers exchange the current state of changed rows, and a merge rule decides the result. The rules are [CRDTs](https://crdt.tech): applying the same changes in any order, any number of times, gives the same result, so no change log has to be replayed in order.

### Clocks

Every change carries a **hybrid logical clock** (HLC) timestamp: 48 bits of Unix milliseconds and a 16-bit counter. An HLC follows wall time but never goes backwards, and it moves past any timestamp it receives, so a change made after seeing another always sorts after it, even when the two computers' clocks disagree. Ties are broken by device ID.

A received HLC more than 5 minutes ahead of the local clock is clamped to local time plus 5 minutes, so one computer with a wrong clock can't push every clip it touches to the top for days. Kopi logs a warning, without content, when that happens.

### Clips

A clip's identity across computers is its **`content_hash`**, not its `uuid`. Two computers that copy the same text independently have different `uuid`s but the same hash, and end with one clip, which is what [Duplicates](DATA_MODEL.md#duplicates) does on one computer.

| Field | Rule |
| --- | --- |
| Content (`kind`, `plain_text`, formats) | Fixed by the hash, so it never conflicts. For `rich_text`, the formats come with the latest use, as they do locally. |
| `last_used_at`, `source_app_*`, copied on | **Latest wins**, by the HLC of the last copy or paste (`used_hlc`). The source app and computer come from that use. |
| `created_at` | **Earliest wins.** |
| `pinned` | **Latest wins**, by `pinned_hlc`. |
| `use_count` | **Not synced.** It's a local ranking signal, and adding counts across computers would double-count on every round trip. |
| `metadata` | Fixed by the content. |

### Pinboards

| Data | Identity | Rule |
| --- | --- | --- |
| Pinboard (`name`, `color`, `position`) | `uuid` | **Latest wins** for the whole row, by `updated_hlc` |
| Pinboard item (`position`, `added_at`) | pinboard `uuid` + clip `content_hash` | **Latest wins**, add or remove, by `hlc` |

Two computers that reorder the same board at once get the later order. When positions tie, the later change goes first.

### Deletes and tombstones

Deleting must reach every computer, or a clip deleted on one comes back from another. So an explicit delete leaves a **tombstone**: the clip's `content_hash` (or the pinboard's `uuid`, or the item's key) with the HLC of the delete.

- A clip exists while its latest use or pin is newer than its newest tombstone. **Deleting wins over everything older, including a pin.** Copying the same content again afterwards, on any computer, brings it back, just as it would locally.
- **Clear history** leaves one **range tombstone**: the time window it cleared (everything, the past hour or the past day), whether it included pins and pinboards, and its HLC. Each computer applies it to its own clips, and keeps it to check clips that arrive later from a computer that was away.
- **Limits don't sync.** When `history.max_items`, `max_age_days` or `max_total_bytes` removes a clip, no tombstone is written, so each computer keeps its own limits. A clip received from a computer with longer limits is removed again by the next retention run.
- Tombstones are kept for **90 days**, then dropped. A computer that hasn't synced for 90 days could miss a delete, so Settings shows it as *Not seen for 90 days* and suggests removing and pairing it again.

A tombstone stores a hash, not content. A BLAKE3 hash of a very short secret can be guessed, but the tombstone only exists on the user's own computers, next to a database that held the content itself.

## Catch-up

Computers sync whatever changed since they last spoke, whatever order they come and go in.

- Each computer has a counter, `sync_seq`. Every change to a synced row, local or received, sets the row's `sync_seq` to the next value. Tombstones and group changes get one too.
- For each paired computer, Kopi remembers the highest of that computer's `sync_seq`s it has applied (`sync_devices.received_seq`).
- When a session starts, each side says what it has from the other. Each sends its rows with a higher `sync_seq`, in order, in batches, and the receiver stores the batch's highest `sync_seq` only after committing it. An interrupted sync resumes where it stopped.
- A change that came from computer B isn't sent back to B. That's an optimisation: the merge rules make the echo harmless.

This works for any number of computers in any shape: a change made on A reaches C through B if A and C are never on at the same time.

**The first sync** after pairing sends a **snapshot**, newest clips first, so recent history is there in seconds while older history follows. The receiver sends its limits in the hello, and the sender skips clips the receiver would remove straight away, such as clips older than its `max_age_days`. The cursor is set to the `sync_seq` from when the snapshot began, once the snapshot is complete. A snapshot that's interrupted starts again, skipping clips the receiver already has.

## Images and other large formats

Formats are already stored with the same names on both platforms. The Windows layer reads and writes the macOS names (UTIs), and `paste::clipboard_formats` reads those on both. A clip copied on a Mac pastes on Windows as it is, with one exception: an image held only as TIFF is converted to PNG before it's sent, because Windows can't paste TIFF. The color archive (`com.apple.cocoa.pasteboard.color`) isn't sent to Windows, where the hex text is enough.

- Formats of 64 KiB or less travel inside the clip.
- Larger formats travel as references to their BLAKE3 hash. The receiver asks for blobs it doesn't have and the sender streams them in chunks. The receiver checks the hash before keeping the file, so a blob two clips share is sent once.
- A clip is committed only when all its blobs are present, so paste never meets a missing file.
- Thumbnails aren't sent. Each computer makes its own.
- The 50 MiB `limits.max_clip_bytes` applies to what's received too.

## Privacy rules

Everything in [PRIVACY_DESIGN.md](PRIVACY_DESIGN.md) still holds. Sync adds:

- **Off by default.** No socket and no advertisement while it's off.
- **Only paired computers**, which the user adds one at a time with a code shown on screen.
- **Never synced:**
  - File clips.
  - Clips marked local-only. On Windows, that's a copy with `CanUploadToCloudClipboard` set to 0. Kopi doesn't read that flag yet; capture starts recording it in a new `local_only` column.
  - Kinds the user unticks in **What to sync**.
- **Concealed and transient copies** are never stored, so they can't sync.
- **Excluded apps on the receiving computer** apply to clips copied on a computer of the same OS. A clip from an app this computer excludes is dropped on arrival.
- **Deletes and Clear history reach every computer** in the group. Copying a password by accident and deleting it removes it everywhere.
- **Removing a computer** stops the others accepting it. It keeps what it has already received; Kopi can't reach into another computer, and Settings says so.
- **Logs** have device IDs, counts and errors, never clip content.

## Receiving

All received data is untrusted, even from a paired computer:

- A protocol message over 20 MiB ends the session, which is room for a full batch of clips with inline formats. So does a blob larger than its stated size.
- A clip's `content_hash` is recomputed from its content, and a mismatch drops the clip.
- Kinds, field lengths, HLCs and pinboard names are checked against the same rules as local writes.
- Messages are CBOR ([`ciborium`](https://crates.io/crates/ciborium)), so new fields can be added without breaking old versions. A peer with a newer major protocol version is shown as *Needs an update*.

## Wire protocol

After the handshake, each side sends CBOR messages:

| Message | Contents |
| --- | --- |
| `Hello` | Protocol version, app version, device ID, name, OS, the highest `sync_seq` received from the other side, this side's limits and kinds to receive |
| `Changes` | A batch of up to 256 rows: clips (with inline formats and blob references), pinboards, pinboard items, tombstones, range tombstones, group members. Ends with the batch's highest `sync_seq` |
| `Ack` | The `sync_seq` committed, so the sender can show progress |
| `Want` | Blob hashes to send |
| `Blob` | A blob chunk: hash, offset, bytes, and whether it's the last |
| `Live` | Sent once catch-up is done. From then on, each new change is sent as a `Changes` of one |
| `Ping` | Every 30 seconds. A session that's silent for 90 seconds is closed |

## Schema changes

Migration `0004_sync.sql`, sketched:

```sql
-- Change tracking. Filled for every change whether sync is on or not, so turning it on later
-- has a starting point. Existing rows get HLCs from their timestamps and sync_seqs in
-- last_used_at order.
ALTER TABLE clips ADD COLUMN used_hlc     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clips ADD COLUMN pinned_hlc   INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clips ADD COLUMN sync_seq     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clips ADD COLUMN copied_on    TEXT;     -- device ID of the last use; NULL = this computer
ALTER TABLE clips ADD COLUMN local_only   INTEGER NOT NULL DEFAULT 0 CHECK (local_only IN (0,1));
ALTER TABLE pinboards      ADD COLUMN updated_hlc INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pinboards      ADD COLUMN sync_seq    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pinboard_items ADD COLUMN hlc         INTEGER NOT NULL DEFAULT 0;
ALTER TABLE pinboard_items ADD COLUMN sync_seq    INTEGER NOT NULL DEFAULT 0;

CREATE TABLE tombstones (
  entity    TEXT    NOT NULL CHECK (entity IN ('clip','pinboard','pinboard_item')),
  key       TEXT    NOT NULL,   -- content_hash, pinboard uuid, or "<pinboard uuid>/<content_hash>"
  hlc       INTEGER NOT NULL,
  sync_seq  INTEGER NOT NULL,
  PRIMARY KEY (entity, key)
) WITHOUT ROWID;

CREATE TABLE clear_ranges (
  id            INTEGER PRIMARY KEY,
  from_ms       INTEGER,          -- NULL = from the beginning
  to_ms         INTEGER NOT NULL,
  include_kept  INTEGER NOT NULL CHECK (include_kept IN (0,1)),
  hlc           INTEGER NOT NULL UNIQUE,
  sync_seq      INTEGER NOT NULL
);

CREATE TABLE sync_devices (
  device_id     TEXT    PRIMARY KEY,
  public_key    BLOB    NOT NULL UNIQUE,
  name          TEXT    NOT NULL,
  os            TEXT    NOT NULL CHECK (os IN ('macos','windows')),
  added_hlc     INTEGER NOT NULL,
  removed_hlc   INTEGER,          -- set when removed from the group
  sync_seq      INTEGER NOT NULL,
  received_seq  INTEGER NOT NULL DEFAULT 0,  -- highest of its sync_seqs applied here
  last_seen_at  INTEGER
) WITHOUT ROWID;

CREATE INDEX clips_by_sync_seq ON clips (sync_seq);
CREATE INDEX pinboards_by_sync_seq ON pinboards (sync_seq);
CREATE INDEX pinboard_items_by_sync_seq ON pinboard_items (sync_seq);
CREATE INDEX tombstones_by_sync_seq ON tombstones (sync_seq);
```

This computer's device ID, its HLC and the `sync_seq` counter live in `settings`, under `sync.*` keys. Removing a pinboard item now writes a tombstone as well as deleting the row. Retention's deletes don't.

## Platform work

**macOS.** Since macOS 15, talking to the local network needs the **Local Network** permission. macOS asks the first time Kopi browses or connects, which is when the user turns sync on. `Info.plist` gains `NSLocalNetworkUsageDescription` ("Kopi finds your other computers on this network to sync your clips with them.") and `NSBonjourServices` (`_kopi._tcp`). The grant is tied to the app's signature, so whether it survives updates with the self-signed certificate ([ADR 0006](decisions/0006-macos-distribution-without-notarization.md)) needs checking.

**Windows.** Listening for connections makes Windows Defender Firewall ask to allow Kopi. On a standard account, allowing it on private networks needs an administrator. The installer is per-user, so it can't add the rule itself. mDNS is blocked on networks marked *Public*, so Settings says to set the network to *Private* if no computers appear.

**Both.** Some networks, such as guest Wi-Fi and many offices, keep devices apart or block multicast. Sync can't work there, and Settings says so when it finds no computers.

## Code layout

- **`crates/kopi-core`**: migration `0004`, HLC stamping and `sync_seq` on every write, tombstones, and the merge functions that apply received rows. Testable without a network.
- **`crates/kopi-sync`** (new): discovery, pairing, Noise sessions, the wire protocol, blob transfer and the session state machine. It runs on Tauri's async runtime (`tokio`) and depends on `kopi-core`.
- **`apps/desktop/src-tauri`**: commands for Settings → Sync and pairing, and events for status.
- **`apps/desktop/src`**: the Sync section, the pairing sheet and "Copied on" in the preview.

New dependencies: `snow`, `spake2`, `ciborium`, `tokio` (already in the tree through Tauri) and an mDNS crate if the spike chooses one. `spake2` hasn't had a third-party audit. It's small and used by Magic Wormhole's Rust port, and the PAKE only protects pairing: sessions rely on Noise and pinned keys.

## Testing

- **Merge rules**: property tests (`proptest`) that apply random sets of changes to several stores in random orders, with duplicates, and check that they end identical.
- **Two or three stores in one process**, synced through in-memory pipes: catch-up, interruption and resume, snapshot, tombstones, Clear history, limits, local-only and excluded apps.
- **Loopback network tests**: pairing with right and wrong codes, a stranger's key refused, a peer with a newer protocol version.
- **By hand, before each release that touches sync**: Mac ↔ Windows ↔ Mac on one network, pairing a third computer through the second, a computer away for a day, and images both ways.

## Plan

Each step is its own pull request, and sync stays hidden until the last.

1. **This design** and [ADR 0007](decisions/0007-lan-sync.md).
2. **Spike 0004: local network on macOS and Windows.** Local Network permission with the self-signed build, and whether it survives an update. The Windows firewall prompt on a standard account. `mdns-sd` against the system responders. Results go in `docs/spikes/0004-lan-sync.md`.
3. **Change tracking in storage.** Migration `0004`, HLC, `sync_seq`, tombstones, `local_only` from Windows captures. No network.
4. **Merge and the protocol** in `kopi-sync`, tested between in-process stores.
5. **Discovery, pairing, sessions and blobs.**
6. **Settings → Sync, the pairing sheet, "Copied on".**
7. **Docs and the website**: PRIVACY_DESIGN, INSTALL_ALPHA (the permission prompts), and the site's sync copy, which currently promises sync "between your computers" without saying "on the same network".

## Open questions

1. **mDNS implementation.** `mdns-sd` is portable, but shares port 5353 with mDNSResponder on macOS and the DNS Client service on Windows. The system APIs avoid that, at the cost of two code paths. Spike 0004 decides.
2. **Local Network permission and updates.** Does the grant survive an update signed with the same self-signed certificate? If it doesn't, every update asks again.
3. **Connecting by address.** Should Settings let the user type another computer's IP address, for networks that block multicast but not connections?
4. **Hiding the public key.** Anyone on the network can connect and learn a computer's static key, a stable identifier. The responder could hide it behind a per-group secret (Noise `XXpsk`), but that needs all computers to know it before the handshake.
5. **Retention across computers.** Should a computer with a 7-day age limit ask others not to send older clips at all, or keep receiving and removing them?

## Later

- **A relay**, for syncing over the internet. The merge layer doesn't care how changes travel, so a relay that stores and forwards ciphertext would be another transport, using the same keys and pairing. That would need its own ADR, since someone has to run it.
- **Universal clipboard**: optionally put a synced clip on the clipboard of the computer the user is at.
- **Sending files**, rather than paths.
