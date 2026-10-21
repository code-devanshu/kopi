# 0008. Snippets are their own kind of data, pasted from the panel

- **Status:** Accepted
- **Date:** 2026-10-21

## Context

"Snippets with variables" is the first item on the Later list in [MVP_SCOPE.md](../MVP_SCOPE.md). Until now, pinboards have covered saved text: the website even suggests a board called "Snippets". The full design is in [SNIPPETS_DESIGN.md](../SNIPPETS_DESIGN.md).

Two choices shape it: what a snippet is in the database, and how a snippet gets triggered.

## Decision

- **A snippet is a row in a new `snippets` table**, not a clip. It has a name, an optional keyword, a plain-text body and a position, and it's identified by a UUID. Clips stay what was copied, identified by content hash. Retention and Clear history never remove snippets.
- **Snippets are pasted from the panel**, in a Snippets tab, with the same keys as clips. A keyword typed exactly in the panel's search puts its snippet first. Kopi doesn't watch typing in other apps.
- **A small set of placeholders**: `{date}`, `{date:FORMAT}`, `{time}`, `{datetime}`, `{clipboard}`, `{input:Label}` and `{cursor}`. Unknown names and double braces paste as typed. They're filled in by the panel, in TypeScript. Rust stores and syncs the text without parsing it.
- **`{clipboard}` is the newest text in Kopi's history**, not the live clipboard, so it can't paste what the privacy filter kept out.
- **`{cursor}`** is done by posting the Left arrow key after the paste, through a new `KeySender::send_left`.
- **Filled-in snippets aren't added to the history.**
- **Snippets sync** like pinboards: the latest change wins the whole row, and deletes leave tombstones. `Hello` gains `snippets_from`, so a computer that skipped snippets on an older version fetches them all after it updates.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Placeholders in pinboard clips | Editing a clip changes its content hash, so it becomes another clip, and sync sees a delete and a new copy. A copied `{date}` should paste `{date}`. Clips have no name or keyword. |
| Expanding keywords typed in any app (as TextExpander does) | Needs Input Monitoring on macOS and a low-level keyboard hook on Windows, which means seeing every key typed in every app. Too much to ask of a clipboard app for the first version. It could come later, off by default, with its own ADR. |
| Filling in placeholders in Rust | Date formats that follow the system language would need a new dependency. The `{input}` form and the preview's highlighting live in the panel anyway, and one parser is simpler than two. |
| `{clipboard}` from the live clipboard | Would paste content Kopi refused to keep, such as a password copied from a password manager. |
| Merging snippet text line by line across computers | Snippets are short and personal. The latest edit winning is easy to understand, and it's how pinboards work. |

## Consequences

- **Good:** no new permission, and nothing new for the privacy filter to cover. Snippets paste the same way on macOS and Windows.
- **Good:** a separate table keeps history's rules (dedupe by hash, limits, Clear history) away from things the user wrote.
- **Bad:** no expansion while typing in other apps, which some people expect from a "snippets" feature.
- **Bad:** `{cursor}` posts arrow keys, which goes wrong in apps that change pasted text, such as editors that re-indent it.
- **Bad:** migration `0005` rebuilds `tombstones` to allow the new entity.
