# Snippets design

> [!NOTE]
> **Status: built,** for 0.1.0-alpha.9, on one computer and in sync. [ADR 0008](decisions/0008-snippets.md) records the decisions, and [What changed while building it](#what-changed-while-building-it) lists where the build differs from this design. To comment, open a [discussion](https://github.com/code-devanshu/kopi/discussions) or a pull request against this file.

A **snippet** is text the user writes once and pastes often: a reply, an address, a signature, a command. It can hold **placeholders** that are filled in when it's pasted, such as today's date, what's on the clipboard, a value typed just before pasting, or where the cursor should end up. Snippets live in their own tab in the panel, paste with the same keys as clips, and sync between paired computers like pinboards do.

Related docs: [DATA_MODEL.md](DATA_MODEL.md), [SYNC_DESIGN.md](SYNC_DESIGN.md), [PRIVACY_DESIGN.md](PRIVACY_DESIGN.md), [ADR 0004](decisions/0004-paste-simulation-approach.md) (how Kopi pastes).

## Why not pinboards

Pinboards already keep text the user reuses, and the website suggests a board called "Snippets". But a pinboard holds **clips**, and a clip is what was copied, stored as-is and identified by its content hash. That's right for history and wrong for something the user writes and edits:

- **Editing a clip changes its hash**, so it becomes a different clip. Sync would see a delete and a new copy, and an edit made on two computers at once would leave both.
- **Placeholders** are the point of a snippet. A clip that contains `{date}` should paste `{date}`.
- **A name and a keyword** help find a snippet. A clip has neither.
- **History rules don't fit.** Clear history and the limits remove clips. They should never remove something the user wrote.

So snippets are their own table, with their own tab. Pinboards stay as they are.

## Goals

- **Quick to use.** Open the panel, type a few letters of the name or the keyword, press Return. Same keys as clips.
- **Quick to make.** From scratch in the panel, or from any text clip with **Save as snippet…**.
- **Useful placeholders**, few of them, with simple rules: date and time, the clipboard, a typed value, and the cursor.
- **Syncs** between paired computers, Mac and Windows, with the same merge rules as pinboards.
- **No new permissions.** Snippets use Accessibility (macOS) for pasting, as clips do, and nothing else.

## Not goals (for this version)

- **Expanding a keyword typed in any app**, such as typing `;addr` in Mail and seeing it replaced. That needs Kopi to watch every key the user types in every app: the Input Monitoring permission on macOS and a low-level keyboard hook on Windows. That's keylogger territory for a clipboard app that promises it reads nothing it doesn't need. Keywords work in the panel's search instead (see [Keywords](#keywords)). Typed expansion could come later, off by default, with its own design.
- **Rich text, images or files in snippets.** Snippets are plain text. Rich text can come later.
- **Folders or groups.** One list, with search. If people build hundreds, groups can come later.
- **Scripts or shell commands as placeholders.** A synced snippet that runs code is a way for one computer to run code on another.
- **Nesting** one snippet in another.
- **Import and export** (Raycast, Alfred, TextExpander, JSON). Later.

## What the user sees

### The Snippets tab

The panel's tabs become **History**, **Pinned**, **Snippets**, then the pinboards. ⌘[ and ⌘] move through them as now.

- Each row shows the snippet's **name**, its first line of text, and its **keyword** as a small badge on the right, if it has one.
- Snippets are listed in the order the user puts them in. A new one goes at the top. They can be dragged to reorder, as pinboards can.
- **Search** matches the name, the keyword and the text. A snippet whose keyword matches the search exactly comes first. The kind and app filters are hidden in this tab, because they don't apply.
- **Empty tab**: "Text you paste often, with the date, the clipboard or a blank to fill in." and a **New snippet** button.

Keys in the Snippets tab:

| Key | Does |
| --- | --- |
| Return | Pastes the selected snippet, with its placeholders filled in |
| ⌘C | Copies it, filled in, without pasting |
| ⌘1–9 | Pastes the first nine |
| ⌘N | New snippet |
| ⌘E | Edits the selected snippet |
| ⌘⌫ | Deletes it, after "Delete *name*? This removes it from your other computers too." |
| ⌘K | The snippet's menu: Paste, Copy, Edit, Duplicate, Move up, Move down, Delete |

⌥Return pastes the same as Return, because snippets are already plain text. On Windows, Ctrl takes the place of ⌘ and Alt of ⌥, as elsewhere in the panel.

The preview pane shows the snippet's text, with each placeholder drawn as a small chip, and under it what it would paste right now, such as "Wednesday, 21 October 2026".

### Making and editing a snippet

**New snippet** (⌘N, or the button) and **Edit** (⌘E) replace the list and preview with an editor, inside the panel:

- **Name**: required, 1–64 characters.
- **Keyword**: optional, 1–32 characters, no spaces. The hint: "Type this in the search box to find it first." If another snippet already uses it, the field says "Also used by *Other name*". That's a warning only: two computers can set the same keyword at the same time, and sync can't refuse one of them.
- **Text**: the body, up to 64 KiB. An **Insert** menu adds a placeholder at the cursor, so nobody has to remember the syntax. Placeholders are highlighted as they're typed.
- Changes **save as they're made**, as in Notes. There's no Save button. Esc or ⌘↩ goes back to the list. A new snippet that's still empty when the editor closes is discarded.

Clicking outside the panel hides it, as now. The draft is already saved, so nothing is lost, and the editor is still open when the panel comes back.

**Save as snippet…** goes in the clip menu (⌘K) for text, code, links and rich text (its plain text). It makes a snippet with the clip's text, named after its first line (cut to 64 characters), puts it at the top of the list and opens the editor on it.

### Keywords

A keyword is a short code, such as `sig` or `;addr`. In the **History** tab as well as Snippets, when the search is exactly a snippet's keyword, that snippet shows as the first row, marked as a snippet, so `sig` then Return pastes the signature from anywhere in the panel. When it isn't exact, History shows clips only.

### Pasting

Pasting a snippet:

1. Fills in its placeholders. If it has `{input}` placeholders, a small form asks for them first (see below).
2. Puts the result on the clipboard as plain text, marked as Kopi's own write, so it **doesn't become a clip in the history**. The snippet is already kept, and a filled-in copy would add a near-duplicate every time.
3. Hides the panel and posts the paste shortcut, as for a clip.
4. If the snippet has `{cursor}`, moves the cursor back to that spot (see below).

Without the Accessibility permission, the text is only copied and the panel says "Press ⌘V", as it does for clips. The cursor can't be moved then.

## Placeholders

Placeholders are names in curly braces. The set is small on purpose.

| Placeholder | Pastes | Example |
| --- | --- | --- |
| `{date}` | Today's date in the system's long format | Wednesday, 21 October 2026 |
| `{date:FORMAT}` | Today's date in a format of your own | `{date:YYYY-MM-DD}` → 2026-10-21 |
| `{time}` | The time now, in the system's short format | 18:45 |
| `{datetime}` | Both | 21 Oct 2026, 18:45 |
| `{clipboard}` | The newest text in Kopi's history | |
| `{input:Label}` | A value typed just before pasting | `Hi {input:Name},` |
| `{cursor}` | Nothing. The cursor ends here | `<b>{cursor}</b>` |

Rules:

- **Unknown names are left as typed.** `{name}` and `{"a": 1}` paste as they are, so most code and JSON need no escaping.
- **Double braces are never placeholders.** `{{date}}` pastes `{{date}}`, so Handlebars, Jinja, Vue and Mustache templates paste as typed. There's no way to paste a single-brace `{date}` itself in this version. That seems rare enough to wait for someone to ask.
- **Names are case-insensitive** and spaces inside the braces are ignored: `{ Date }` works.
- **Format tokens** for `{date:…}`: `YYYY`, `YY`, `MMMM` (October), `MMM` (Oct), `MM`, `M`, `DD`, `D`, `dddd` (Wednesday), `ddd` (Wed), `HH`, `H`, `hh`, `h`, `mm`, `ss`, `A` (AM/PM). Other characters are kept. Text in square brackets is kept as-is, so `[Week of] D MMM` works. Month and day names follow the system language.
- **`{input:Label}`**: the form has one field per distinct label, in the order they first appear, so `{input:Name}` used twice asks once. Return in the last field pastes, Esc goes back without pasting. A label is 1–32 characters. `{input}` alone asks for "Text".
- **`{cursor}`**: only the first one counts. Any others are removed.
- **`{clipboard}`** is the newest clip in Kopi's history that has text, **not** whatever is on the clipboard right now. That way it never pastes something Kopi chose not to keep: a password from a password manager, a concealed or transient copy, or a copy from an excluded app. If capture is paused, it's the newest clip from before the pause. With no text in the history, it pastes nothing.

### Moving the cursor

After the paste, Kopi posts the **Left arrow** key once for each character after `{cursor}`. That's how other apps do it too; no OS lets one app place the cursor in another.

- Characters are counted as the user sees them (grapheme clusters), and a line break counts as one.
- After more than 1,000 characters, the cursor isn't moved, because posting that many keys is slow and visible.
- Some apps change pasted text, such as editors that re-indent pasted code. Then the cursor lands in the wrong place. The editor's help says so, and it's the reason `{cursor}` works best near the end of a snippet.

This needs one addition to the platform layer: `KeySender::send_left(count)`, with `CGEventPost` on macOS and `SendInput` on Windows, as `send_paste` does now.

## Data model

Migration `0005_snippets.sql`:

```sql
CREATE TABLE snippets (
  id            INTEGER PRIMARY KEY,
  uuid          TEXT    NOT NULL UNIQUE,  -- UUIDv7, its identity across computers
  name          TEXT    NOT NULL,         -- 1–64 characters
  keyword       TEXT,                     -- 1–32 characters, no whitespace; not unique
  body          TEXT    NOT NULL,         -- up to 64 KiB, placeholders as typed
  position      INTEGER NOT NULL,         -- order in the tab
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  last_used_at  INTEGER,                  -- this computer only
  use_count     INTEGER NOT NULL DEFAULT 0, -- this computer only
  updated_hlc   INTEGER NOT NULL,
  sync_seq      INTEGER NOT NULL
);
CREATE INDEX snippets_by_sync_seq ON snippets (sync_seq);
```

- The **body is stored as typed**, placeholders and all. It's filled in only when pasted.
- **Not in the search index.** There will be tens or hundreds of snippets, not 100k, so search reads them all and matches in Rust, which also keeps them apart from clip search.
- **Retention, limits and Clear history never touch snippets.** Only deleting one does.
- **`tombstones`** gains the entity `snippet`, keyed by `uuid`. SQLite can't change a `CHECK` constraint in place, so the migration rebuilds that table, keeping its rows.
- **`sync_devices`** gains `snippets_seq` (see [Older versions](#older-versions)), set to 0 for every computer already paired.
- Settings: `sync.kinds` can now include `snippets`, on by default.

Filling in placeholders happens in the panel (TypeScript), not in Rust. The panel already has the system's date formats through `Intl.DateTimeFormat`, it draws the `{input}` form, and its preview needs the same parser to draw the chips. Rust stores and syncs snippets as text and checks lengths. It never parses them. The parser and filler are one pure module, `features/snippets/template.ts`, with unit tests.

## Sync

Snippets sync like **pinboards**: the latest change wins the whole row, by `updated_hlc`.

| Row | Identity | Merge |
| --- | --- | --- |
| Snippet (`name`, `keyword`, `body`, `position`) | `uuid` | **Latest wins** the whole row, by `updated_hlc`. `last_used_at` and `use_count` stay local |
| Deleted snippet | tombstone `('snippet', uuid)` | Deletes an older row and keeps it from coming back. Wins a tie |

- Two people editing the same snippet on two computers at once get the later edit. Snippets are short and personal, so merging the text line by line isn't worth it.
- Reordering writes a new `position` and HLC for each snippet it moves, as reordering pinboards does.
- `Changes` gains a `snippets` list. Received snippets are checked against the same rules as local edits: name, keyword and body lengths, and a valid UUID.
- **What to sync** in Settings → Sync gains **Snippets**, on.

### Older versions

`Changes` and `Hello` are CBOR with unknown fields skipped, so an alpha.8 computer reads a batch with snippets in it without failing. But it **skips the snippets and still moves its cursor past them**, so after it updates, it would never get the snippets it skipped. So:

- `Hello` gains `snippets_from`: how far this side has received snippets from the other, or nothing if it doesn't take them (an older Kopi, or Snippets unticked in What to sync).
- A sender sends snippets only to a computer whose `Hello` has `snippets_from`. It sends the snippets with a `sync_seq` between `snippets_from` and `received_seq` first, then carries snippets in its normal batches.
- The receiver keeps `snippets_seq` in step with `received_seq` while it takes snippets, and leaves it where it is while Snippets is unticked.
- The migration sets `snippets_seq` to 0 for every paired computer, so the first session after updating fetches every snippet the other has.

So a computer that updates gets every snippet, whichever computer updated first, and ticking Snippets again after unticking it fetches what was missed. Snippets are few and small, so sending them all again costs nothing. The protocol version stays 1.

## Privacy

- Snippets are stored in `kopi.db`, beside the history, with the same protections ([PRIVACY_DESIGN.md](PRIVACY_DESIGN.md)). A snippet is something the user chose to write down, so Kopi doesn't scan it for secrets.
- `{clipboard}` reads Kopi's history, never the live clipboard, so it can't paste what the privacy filter kept out.
- Filled-in snippets aren't added to the history.
- Logs have counts and IDs, never a snippet's name or text.
- Deleting a snippet deletes it on every paired computer, and the confirmation says so.

## Testing

- **Template** (`template.test.ts`): every placeholder, unknown names, double braces, case and spaces, each date token and bracketed text, `{input}` labels in order and repeated, more than one `{cursor}`, the character count after `{cursor}` with emoji, combining marks and line breaks, and a 64 KiB body.
- **Storage** (Rust): create, edit, reorder, delete, the length rules, and that retention and Clear history leave snippets alone.
- **Merge** ([`crates/kopi-core/tests/sync.rs`](../crates/kopi-core/tests/sync.rs)): concurrent edits, edit against delete, reorders, and snippets added to the random three-store test, which must still end identical.
- **Older versions**: a store migrated from alpha.8 fetches every snippet in its first session. Syncing with a peer that sends no `snippets_from` sends none, and they arrive once it does. These run over loopback with `discovery: false`, like the existing service tests.
- **By hand**, on macOS and Windows: paste and `{cursor}` in TextEdit, Notes, Mail, Safari and Chrome text fields, VS Code, Slack, Word and Notepad, and a snippet edited on one computer showing up on the other.

## Building it

Two pull requests, both in 0.1.0-alpha.9:

1. **Snippets on one computer**: the table, the tab, the editor, Save as snippet, every placeholder, `send_left` on both platforms, keywords in search.
2. **Snippets sync**: the merge, `snippets_from`, the What to sync option, and the tests against alpha.8.

The website's "Snippets" pinboard examples change to show the Snippets tab, and the README and MVP_SCOPE list snippets as built.

## What changed while building it

- **A brace pair across lines isn't a placeholder**, so `{\n  cursor\n}` in code stays as typed. Spaces inside the braces are still ignored, so a one-line `{ date }` does fill in.
- **Only `{date}` and `{input}` take a value** after a colon. `{time:HH}` and the like paste as typed.
- **Keyword rows in History** show only while no kind or app filter is set, since a snippet has neither.
- **Moving snippets**, by dragging or with Move up and Move down, works while the whole list shows. With a search, the rows aren't next to each other in the real order.
- **⌘N works in every tab.** A snippet made outside the Snippets tab opens that tab when the editor closes, with the new snippet selected.
- **Dropping a text clip on the Snippets tab** does Save as snippet, as dropping one on a pinboard adds it there.
- **Duplicate** puts the copy at the top, named "*name* copy", without the keyword, so the two don't share it.
- **The `{input}` form closes when the panel hides**, since the app the snippet would paste into may have changed.
- **`{clipboard}`** takes text, rich text (its plain text), links (the URL), code and colors. Images and files have no text.
- **Before the arrow keys for `{cursor}`**, Kopi waits 60 ms after the paste, since some apps read the clipboard a moment after ⌘V.
- **Esc in the editor goes back to the list** instead of hiding the panel, and so does ⌘↩.
- **Unticking Snippets in What to sync stops this computer taking them.** Its own still go to the computers that take them, as with the kinds of clip.
- **Ticking or unticking Snippets starts the sessions again**, since a session's `Hello` says whether to send them. Ticking it fetches what was missed once they reconnect, within about 15 seconds.
- **Snippet deletes go only to computers that take snippets.** One that starts taking them gets the deletes it missed along with the snippets.
- **The delete confirmation mentions other computers only while sync is on** with at least one paired computer.
- **An open editor isn't updated** by an edit from another computer. The next change typed is the later edit, so it wins.

## Open questions

- **Should pasting a snippet restore what was on the clipboard before?** It would leave the clipboard as the user had it, but it means waiting for the target app to read the paste, which differs between apps. "Restoring the previous clipboard" is its own item on the Later list, and snippets would use it when it comes.
- **A shortcut that opens the panel straight on the Snippets tab?** Easy to add once people ask for it.
- **Offer to turn a pinboard into snippets?** Useful for anyone who followed the website's "Snippets" board example. It would copy the board's text clips into snippets and leave the board alone.
- **Sort by most used instead of the user's order?** The user's order keeps ⌘1–9 stable, which is why it's the default.
