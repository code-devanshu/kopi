//! Snippets against a real store.

use kopi_core::storage::{
    ClipKind, Limits, MAX_SNIPPET_BODY_BYTES, MAX_SNIPPET_NAME_CHARS, NewClip, NewFormat, Platform,
    SnippetEdit, StorageError, Store,
};

fn open() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    (dir, store)
}

fn edit<'a>(name: &'a str, keyword: Option<&'a str>, body: &'a str) -> SnippetEdit<'a> {
    SnippetEdit {
        name,
        keyword,
        body,
    }
}

fn add(store: &Store, name: &str, keyword: Option<&str>, body: &str) -> i64 {
    store.create_snippet(edit(name, keyword, body)).unwrap().id
}

fn names(snippets: Vec<kopi_core::storage::Snippet>) -> Vec<String> {
    snippets.into_iter().map(|s| s.name).collect()
}

fn add_clip(store: &Store, text: &str, at: i64) -> i64 {
    let mut h = blake3::Hasher::new();
    h.update(b"text\0");
    h.update(text.as_bytes());
    let clip = NewClip {
        kind: ClipKind::Text,
        plain_text: Some(text.to_owned()),
        content_hash: h.finalize().to_hex().to_string(),
        source_app_name: None,
        source_app_id: None,
        metadata: None,
        platform: Platform::MacOs,
        formats: vec![NewFormat {
            item_index: 0,
            format: "public.utf8-plain-text".into(),
            data: text.as_bytes().to_vec(),
        }],
        local_only: false,
    };
    store.upsert_clip_at(clip, at).unwrap().id
}

fn tombstones(store: &Store) -> Vec<(String, String)> {
    store
        .with_reader(|conn| {
            Ok(conn
                .prepare("SELECT entity, key FROM tombstones ORDER BY entity, key")?
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<Result<_, _>>()?)
        })
        .unwrap()
}

#[test]
fn a_new_snippet_goes_at_the_top() {
    let (_dir, store) = open();
    assert!(store.snippets().unwrap().is_empty());
    add(&store, "Address", Some("addr"), "1 Infinite Loop");
    let sig = store
        .create_snippet(edit("  Signature ", Some(" sig "), "Thanks,\nDevanshu\n"))
        .unwrap();
    assert_eq!(sig.name, "Signature");
    assert_eq!(sig.keyword.as_deref(), Some("sig"));
    assert_eq!(sig.body, "Thanks,\nDevanshu\n");
    assert_eq!(sig.use_count, 0);
    assert_eq!(sig.last_used_at, None);
    assert_eq!(store.snippet(sig.id).unwrap(), Some(sig.clone()));
    assert_eq!(names(store.snippets().unwrap()), ["Signature", "Address"]);
}

#[test]
fn edits_are_checked() {
    let (_dir, store) = open();
    let invalid = |e: SnippetEdit<'_>| {
        assert!(matches!(
            store.create_snippet(e),
            Err(StorageError::Invalid(_))
        ));
    };
    invalid(edit("", None, "body"));
    invalid(edit(" \t", None, "body"));
    invalid(edit(&"n".repeat(MAX_SNIPPET_NAME_CHARS + 1), None, "body"));
    invalid(edit("Name", Some("two words"), "body"));
    invalid(edit("Name", None, &"b".repeat(MAX_SNIPPET_BODY_BYTES + 1)));
    // An empty body and no keyword are fine.
    let id = add(&store, "Empty", Some("   "), "");
    let snippet = store.snippet(id).unwrap().unwrap();
    assert_eq!((snippet.keyword, snippet.body.as_str()), (None, ""));
    assert!(matches!(
        store.update_snippet(id, edit("", None, "")),
        Err(StorageError::Invalid(_))
    ));
}

#[test]
fn an_edit_replaces_the_fields_and_a_no_op_writes_nothing() {
    let (_dir, store) = open();
    let id = add(&store, "Reply", None, "Thanks!");
    let before = store.sync_seq().unwrap();
    assert!(
        store
            .update_snippet(id, edit("Reply", None, "Thanks!"))
            .unwrap()
    );
    assert_eq!(store.sync_seq().unwrap(), before);

    assert!(
        store
            .update_snippet(id, edit("Quick reply", Some("ty"), "Thanks, {input:Name}!"))
            .unwrap()
    );
    assert!(store.sync_seq().unwrap() > before);
    let snippet = store.snippet(id).unwrap().unwrap();
    assert_eq!(snippet.name, "Quick reply");
    assert_eq!(snippet.keyword.as_deref(), Some("ty"));
    assert_eq!(snippet.body, "Thanks, {input:Name}!");

    assert!(!store.update_snippet(id + 100, edit("x", None, "")).unwrap());
}

#[test]
fn reordering_puts_the_given_ids_first() {
    let (_dir, store) = open();
    let c = add(&store, "C", None, "");
    let b = add(&store, "B", None, "");
    let a = add(&store, "A", None, "");
    assert_eq!(names(store.snippets().unwrap()), ["A", "B", "C"]);
    store.reorder_snippets(&[c, 999, a]).unwrap();
    assert_eq!(names(store.snippets().unwrap()), ["C", "A", "B"]);
    let before = store.sync_seq().unwrap();
    store.reorder_snippets(&[c, a, b]).unwrap();
    assert_eq!(store.sync_seq().unwrap(), before, "nothing moved");
    // A new one still goes on top.
    add(&store, "D", None, "");
    assert_eq!(names(store.snippets().unwrap()), ["D", "C", "A", "B"]);
}

#[test]
fn deleting_leaves_a_tombstone() {
    let (_dir, store) = open();
    let id = add(&store, "Gone", None, "");
    let uuid = store.snippet(id).unwrap().unwrap().uuid;
    assert!(store.delete_snippet(id).unwrap());
    assert!(!store.delete_snippet(id).unwrap());
    assert_eq!(store.snippet(id).unwrap(), None);
    assert_eq!(tombstones(&store), [("snippet".to_owned(), uuid)]);
}

#[test]
fn use_is_counted_without_a_change_to_sync() {
    let (_dir, store) = open();
    let id = add(&store, "Sig", None, "");
    let before = store.sync_seq().unwrap();
    assert!(store.mark_snippet_used(id, 5_000).unwrap());
    assert!(store.mark_snippet_used(id, 6_000).unwrap());
    let snippet = store.snippet(id).unwrap().unwrap();
    assert_eq!((snippet.use_count, snippet.last_used_at), (2, Some(6_000)));
    assert_eq!(store.sync_seq().unwrap(), before);
    assert!(!store.mark_snippet_used(id + 1, 5_000).unwrap());
}

#[test]
fn search_puts_an_exact_keyword_first() {
    let (_dir, store) = open();
    add(&store, "Signature for sig-ups", None, "Welcome aboard");
    add(
        &store,
        "Address",
        Some("addr"),
        "1 Infinite Loop, Cupertino",
    );
    add(&store, "Signature", Some("SIG"), "Thanks,\nDevanshu");
    assert_eq!(
        names(store.find_snippets("").unwrap()),
        ["Signature", "Address", "Signature for sig-ups"]
    );
    assert_eq!(
        names(store.find_snippets(" sig ").unwrap()),
        ["Signature", "Signature for sig-ups"]
    );
    assert_eq!(
        names(store.find_snippets("cupertino loop").unwrap()),
        ["Address"]
    );
    assert!(store.find_snippets("loop nowhere").unwrap().is_empty());

    assert_eq!(
        names(store.snippets_with_keyword("Sig").unwrap()),
        ["Signature"]
    );
    assert!(store.snippets_with_keyword("si").unwrap().is_empty());
    assert!(store.snippets_with_keyword("  ").unwrap().is_empty());
}

#[test]
fn history_limits_and_clear_history_leave_snippets_alone() {
    let (_dir, store) = open();
    add(&store, "Keep me", None, "body");
    add_clip(&store, "old clip", 1_000);
    let limits = Limits {
        max_items: Some(0),
        max_age_days: Some(1),
        max_total_bytes: Some(0),
    };
    store.apply_retention(&limits, 10 * 86_400_000).unwrap();
    store.clear_history(None, false).unwrap();
    assert_eq!(names(store.snippets().unwrap()), ["Keep me"]);
}

#[test]
fn the_newest_text_clip_is_for_clipboard_placeholders() {
    let (_dir, store) = open();
    assert_eq!(store.newest_text_clip().unwrap(), None);
    let old = add_clip(&store, "older", 1_000);
    let new = add_clip(&store, "newer", 2_000);
    assert_eq!(store.newest_text_clip().unwrap(), Some(new));
    store.mark_used(old, 3_000).unwrap();
    assert_eq!(store.newest_text_clip().unwrap(), Some(old));
}

#[test]
fn a_database_from_alpha_8_gains_snippets_and_keeps_its_tombstones() {
    let dir = tempfile::tempdir().unwrap();
    {
        // As Kopi 0.1.0-alpha.8 left it: schema 4, with a delete and a paired computer.
        let conn = rusqlite::Connection::open(dir.path().join("kopi.db")).unwrap();
        for sql in [
            include_str!("../migrations/0001_init.sql"),
            include_str!("../migrations/0002_default_excluded_apps.sql"),
            include_str!("../migrations/0003_windows_excluded_apps.sql"),
            include_str!("../migrations/0004_sync.sql"),
        ] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 4).unwrap();
        conn.execute(
            "INSERT INTO tombstones (entity, key, hlc, sync_seq) VALUES ('clip', 'abc', 7, 3)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sync_devices (device_id, public_key, name, os, updated_hlc, sync_seq,
                                       received_seq)
             VALUES ('b', x'01', 'Other Mac', 'macos', 1, 1, 40)",
            [],
        )
        .unwrap();
    }
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(tombstones(&store), [("clip".to_owned(), "abc".to_owned())]);
    let (received, snippets): (i64, i64) = store
        .with_reader(|conn| {
            Ok(conn.query_row(
                "SELECT received_seq, snippets_seq FROM sync_devices WHERE device_id = 'b'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?)
        })
        .unwrap();
    assert_eq!((received, snippets), (40, 0));
    let id = add(&store, "New", None, "");
    store.delete_snippet(id).unwrap();
    assert_eq!(tombstones(&store).len(), 2);
}
