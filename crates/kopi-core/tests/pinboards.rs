//! Pins and pinboards against a real store.

use kopi_core::search::{SearchQuery, search};
use kopi_core::storage::{
    ClipKind, MAX_PINBOARD_NAME_CHARS, NewClip, NewFormat, Platform, StorageError, Store,
};

fn open() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    (dir, store)
}

fn add(store: &Store, text: &str) -> i64 {
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
    store.upsert_clip(clip).unwrap().id
}

fn names(store: &Store) -> Vec<String> {
    store
        .pinboards()
        .unwrap()
        .into_iter()
        .map(|b| b.name)
        .collect()
}

/// The clips on a board, in its order.
fn board(store: &Store, id: i64) -> Vec<i64> {
    let query = SearchQuery {
        pinboard_id: Some(id),
        ..SearchQuery::default()
    };
    let page = search(store, &query).unwrap();
    page.hits.iter().map(|h| h.id).collect()
}

#[test]
fn pinning_survives_copying_again() {
    let (_dir, store) = open();
    let id = add(&store, "keep me");
    assert!(!store.get_clip(id).unwrap().unwrap().pinned);

    assert!(store.set_pinned(id, true).unwrap());
    assert!(store.get_clip(id).unwrap().unwrap().pinned);
    // A repeat copy moves it to the top and leaves it pinned.
    assert_eq!(add(&store, "keep me"), id);
    assert!(store.get_clip(id).unwrap().unwrap().pinned);

    assert!(store.set_pinned(id, false).unwrap());
    assert!(!store.get_clip(id).unwrap().unwrap().pinned);
    assert!(!store.set_pinned(id + 100, true).unwrap());
}

#[test]
fn pinboards_are_created_renamed_recolored_and_deleted() {
    let (_dir, store) = open();
    assert!(store.pinboards().unwrap().is_empty());

    let work = store.create_pinboard("  Work ", Some("#d08a3a")).unwrap();
    assert_eq!(work.name, "Work");
    assert_eq!(work.color.as_deref(), Some("#D08A3A"));
    assert_eq!((work.position, work.clip_count), (0, 0));
    assert_eq!(work.uuid.len(), 36);
    let home = store.create_pinboard("Home", None).unwrap();
    assert_eq!(home.position, 1);
    assert_eq!(names(&store), ["Work", "Home"]);

    assert!(store.rename_pinboard(home.id, "Personal").unwrap());
    assert!(store.recolor_pinboard(home.id, Some("#4a78b5")).unwrap());
    let personal = store.pinboard(home.id).unwrap().unwrap();
    assert_eq!(personal.name, "Personal");
    assert_eq!(personal.color.as_deref(), Some("#4A78B5"));
    assert!(personal.updated_at >= home.updated_at);
    assert!(store.recolor_pinboard(home.id, None).unwrap());
    assert_eq!(store.pinboard(home.id).unwrap().unwrap().color, None);

    assert!(!store.rename_pinboard(999, "Nope").unwrap());
    assert!(!store.recolor_pinboard(999, None).unwrap());

    assert!(store.delete_pinboard(work.id).unwrap());
    assert!(!store.delete_pinboard(work.id).unwrap());
    assert_eq!(names(&store), ["Personal"]);
    assert_eq!(store.pinboard(work.id).unwrap(), None);
}

#[test]
fn bad_names_and_colors_are_refused() {
    let (_dir, store) = open();
    let long = "x".repeat(MAX_PINBOARD_NAME_CHARS + 1);
    for name in ["", "   ", long.as_str()] {
        let err = store.create_pinboard(name, None).unwrap_err();
        assert!(matches!(err, StorageError::Invalid(_)), "{name:?}: {err}");
    }
    let err = store.create_pinboard("Work", Some("orange")).unwrap_err();
    assert!(matches!(err, StorageError::Invalid(_)), "{err}");

    let board = store.create_pinboard("Work", None).unwrap();
    assert!(store.rename_pinboard(board.id, " ").is_err());
    assert!(store.recolor_pinboard(board.id, Some("#12345")).is_err());
    assert_eq!(names(&store), ["Work"]);
}

#[test]
fn pinboards_are_reordered() {
    let (_dir, store) = open();
    let ids: Vec<i64> = ["A", "B", "C", "D"]
        .iter()
        .map(|name| store.create_pinboard(name, None).unwrap().id)
        .collect();
    let [a, b, c, d] = ids[..] else {
        unreachable!()
    };

    store.reorder_pinboards(&[c, a, b, d]).unwrap();
    assert_eq!(names(&store), ["C", "A", "B", "D"]);

    // Left-out boards keep their order after the given ones, and unknown or repeated ids are
    // skipped.
    store.reorder_pinboards(&[d, 999, d]).unwrap();
    assert_eq!(names(&store), ["D", "C", "A", "B"]);
    let positions: Vec<i64> = store
        .pinboards()
        .unwrap()
        .iter()
        .map(|b| b.position)
        .collect();
    assert_eq!(positions, [0, 1, 2, 3]);

    // A new board goes last, and a gap left by a delete doesn't matter.
    store.delete_pinboard(c).unwrap();
    store.create_pinboard("E", None).unwrap();
    assert_eq!(names(&store), ["D", "A", "B", "E"]);
}

#[test]
fn clips_are_added_to_and_removed_from_pinboards() {
    let (_dir, store) = open();
    let one = add(&store, "one");
    let two = add(&store, "two");
    let three = add(&store, "three");
    let work = store.create_pinboard("Work", None).unwrap().id;
    let home = store.create_pinboard("Home", None).unwrap().id;

    // Newly added clips go on top, the first of them topmost.
    assert_eq!(store.add_to_pinboard(work, &[one]).unwrap(), Some(1));
    assert_eq!(store.add_to_pinboard(work, &[two, three]).unwrap(), Some(2));
    assert_eq!(board(&store, work), [two, three, one]);

    // Clips already on it and unknown clips are skipped.
    assert_eq!(store.add_to_pinboard(work, &[one, 999]).unwrap(), Some(0));
    assert_eq!(store.add_to_pinboard(999, &[one]).unwrap(), None);

    // A clip can be on several boards.
    store.add_to_pinboard(home, &[one]).unwrap();
    assert_eq!(store.clip_pinboards(one).unwrap(), [work, home]);
    assert_eq!(store.clip_pinboards(two).unwrap(), [work]);
    let counts: Vec<i64> = store
        .pinboards()
        .unwrap()
        .iter()
        .map(|b| b.clip_count)
        .collect();
    assert_eq!(counts, [3, 1]);

    assert_eq!(
        store
            .remove_from_pinboard(work, &[three, three, 999])
            .unwrap(),
        1
    );
    assert_eq!(board(&store, work), [two, one]);
    assert!(
        store.get_clip(three).unwrap().is_some(),
        "still in the history"
    );

    // Deleting a board keeps its clips. Deleting a clip takes it off every board.
    store.delete_pinboard(home).unwrap();
    assert!(store.get_clip(one).unwrap().is_some());
    assert_eq!(store.clip_pinboards(one).unwrap(), [work]);
    store.delete_clips(&[two]).unwrap();
    assert_eq!(board(&store, work), [one]);
    assert_eq!(store.pinboard(work).unwrap().unwrap().clip_count, 1);
}

#[test]
fn pinned_and_pinboard_clips_are_kept() {
    let (_dir, store) = open();
    let loose = add(&store, "loose");
    let pinned = add(&store, "pinned");
    let boarded = add(&store, "boarded");
    store.set_pinned(pinned, true).unwrap();
    let board = store.create_pinboard("Work", None).unwrap().id;
    store.add_to_pinboard(board, &[boarded]).unwrap();

    assert!(!store.is_kept(loose).unwrap());
    assert!(store.is_kept(pinned).unwrap());
    assert!(store.is_kept(boarded).unwrap());
    assert!(!store.is_kept(999).unwrap());

    // Unpinning, or taking it off its last board, lets retention have it again.
    store.set_pinned(pinned, false).unwrap();
    store.remove_from_pinboard(board, &[boarded]).unwrap();
    assert!(!store.is_kept(pinned).unwrap());
    assert!(!store.is_kept(boarded).unwrap());
}
