//! Retention, clearing history and the orphan sweep against a real store. Each rule in
//! `docs/DATA_MODEL.md#retention-and-cleanup` has a test here.

use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;

use kopi_core::retention::{Limits, Retention};
use kopi_core::storage::{
    ClipKind, DEFAULT_MAX_TOTAL_BYTES, NewClip, NewFormat, Platform, StorageError, Store,
};

const DAY: i64 = 24 * 60 * 60 * 1000;
const NOW: i64 = 1_800_000_000_000;

const UNLIMITED: Limits = Limits {
    max_items: None,
    max_age_days: None,
    max_total_bytes: None,
};

fn open() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    (dir, store)
}

fn hash(kind: &str, data: &[u8]) -> String {
    let mut h = blake3::Hasher::new();
    h.update(kind.as_bytes());
    h.update(b"\0");
    h.update(data);
    h.finalize().to_hex().to_string()
}

/// A text clip last used at `at`. Its size is the text's length.
fn add_at(store: &Store, text: &str, at: i64) -> i64 {
    add_with(store, text, Vec::new(), at)
}

/// A text clip with extra formats, such as one big enough for a blob file.
fn add_with(store: &Store, text: &str, formats: Vec<NewFormat>, at: i64) -> i64 {
    let clip = NewClip {
        kind: ClipKind::Text,
        plain_text: Some(text.to_owned()),
        content_hash: hash("text", text.as_bytes()),
        source_app_name: None,
        source_app_id: None,
        metadata: None,
        platform: Platform::MacOs,
        formats,
    };
    store.upsert_clip_at(clip, at).unwrap().id
}

fn big_format(byte: u8) -> NewFormat {
    NewFormat {
        item_index: 0,
        format: "public.rtf".into(),
        data: vec![byte; 100 * 1024],
    }
}

fn ids(store: &Store) -> Vec<i64> {
    store
        .with_reader(|conn| {
            Ok(conn
                .prepare("SELECT id FROM clips ORDER BY id")?
                .query_map([], |row| row.get(0))?
                .collect::<Result<_, _>>()?)
        })
        .unwrap()
}

fn count_files(dir: &Path) -> usize {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .map(|e| {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                count_files(&e.path())
            } else {
                1
            }
        })
        .sum()
}

#[test]
fn limits_default_to_two_gib_and_round_trip() {
    let (_dir, store) = open();
    assert_eq!(
        store.retention_limits().unwrap(),
        Limits {
            max_items: None,
            max_age_days: None,
            max_total_bytes: Some(DEFAULT_MAX_TOTAL_BYTES),
        }
    );
    assert_eq!(DEFAULT_MAX_TOTAL_BYTES, 2 * 1024 * 1024 * 1024);

    let limits = Limits {
        max_items: Some(500),
        max_age_days: Some(30),
        max_total_bytes: None,
    };
    store.set_retention_limits(&limits).unwrap();
    assert_eq!(store.retention_limits().unwrap(), limits);

    let zero = Limits {
        max_items: Some(0),
        ..limits
    };
    assert!(matches!(
        store.set_retention_limits(&zero),
        Err(StorageError::Invalid(_))
    ));
    assert_eq!(store.retention_limits().unwrap(), limits);
}

#[test]
fn count_limit_removes_the_least_recently_used_first() {
    let (_dir, store) = open();
    let a = add_at(&store, "a", NOW - 4000);
    let b = add_at(&store, "b", NOW - 3000);
    let c = add_at(&store, "c", NOW - 2000);
    let d = add_at(&store, "d", NOW - 1000);
    // Pasting `a` makes it the most recent.
    store.mark_used(a, NOW).unwrap();

    let limits = Limits {
        max_items: Some(2),
        ..UNLIMITED
    };
    let removed = store.apply_retention(&limits, NOW).unwrap();
    assert_eq!(sorted(removed), vec![b, c]);
    assert_eq!(ids(&store), vec![a, d]);

    // Nothing more to do.
    assert!(store.apply_retention(&limits, NOW).unwrap().is_empty());
}

#[test]
fn age_limit_removes_clips_last_used_too_long_ago() {
    let (_dir, store) = open();
    let old = add_at(&store, "old", NOW - 8 * DAY);
    let recent = add_at(&store, "recent", NOW - 6 * DAY);
    let reused = add_at(&store, "reused", NOW - 30 * DAY);
    store.mark_used(reused, NOW - DAY).unwrap();

    let limits = Limits {
        max_age_days: Some(7),
        ..UNLIMITED
    };
    assert_eq!(store.apply_retention(&limits, NOW).unwrap(), vec![old]);
    assert_eq!(ids(&store), vec![recent, reused]);
}

#[test]
fn size_limit_removes_the_oldest_until_it_fits() {
    let (_dir, store) = open();
    let a = add_at(&store, &"a".repeat(400), NOW - 4000);
    let b = add_at(&store, &"b".repeat(300), NOW - 3000);
    let c = add_at(&store, &"c".repeat(200), NOW - 2000);
    let d = add_at(&store, &"d".repeat(100), NOW - 1000);

    let limits = Limits {
        max_total_bytes: Some(650),
        ..UNLIMITED
    };
    // d + c + b = 600 fits; a would make it 1000.
    assert_eq!(store.apply_retention(&limits, NOW).unwrap(), vec![a]);

    // Kept clips count towards the size, though they're never removed.
    store.set_pinned(b, true).unwrap();
    let limits = Limits {
        max_total_bytes: Some(450),
        ..UNLIMITED
    };
    // b (300, kept) + d (100) = 400 fits; c would make it 600.
    assert_eq!(store.apply_retention(&limits, NOW).unwrap(), vec![c]);
    assert_eq!(ids(&store), vec![b, d]);
}

#[test]
fn limits_never_remove_pinned_clips_or_clips_on_a_pinboard() {
    let (_dir, store) = open();
    let pinned = add_at(&store, &"p".repeat(1000), NOW - 100 * DAY);
    let boarded = add_at(&store, &"q".repeat(1000), NOW - 100 * DAY);
    let loose = add_at(&store, &"l".repeat(1000), NOW - 100 * DAY);
    store.set_pinned(pinned, true).unwrap();
    let board = store.create_pinboard("Keep", None).unwrap();
    store.add_to_pinboard(board.id, &[boarded]).unwrap();

    let limits = Limits {
        max_items: Some(1),
        max_age_days: Some(1),
        max_total_bytes: Some(1),
    };
    assert_eq!(store.apply_retention(&limits, NOW).unwrap(), vec![loose]);
    assert_eq!(ids(&store), vec![pinned, boarded]);

    // Unpinned, it's history again.
    store.set_pinned(pinned, false).unwrap();
    assert_eq!(store.apply_retention(&limits, NOW).unwrap(), vec![pinned]);
}

#[test]
fn removed_clips_take_their_unshared_blob_files_with_them() {
    let (dir, store) = open();
    let blobs = dir.path().join("blobs");
    // Two clips share one blob file; a third has its own.
    let a = add_with(&store, "a", vec![big_format(1)], NOW - 3000);
    let b = add_with(&store, "b", vec![big_format(1)], NOW - 2000);
    let c = add_with(&store, "c", vec![big_format(2)], NOW - 1000);
    assert_eq!(count_files(&blobs), 2);

    let limits = |n| Limits {
        max_items: Some(n),
        ..UNLIMITED
    };
    assert_eq!(store.apply_retention(&limits(2), NOW).unwrap(), vec![a]);
    assert_eq!(count_files(&blobs), 2, "b still uses the shared file");
    assert_eq!(store.apply_retention(&limits(1), NOW).unwrap(), vec![b]);
    assert_eq!(count_files(&blobs), 1);
    assert_eq!(store.clip_formats(c).unwrap()[0].data, vec![2; 100 * 1024]);
}

#[test]
fn removed_image_clips_take_their_thumbnails_with_them() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = mpsc::channel();
    let store = Store::open_with_listener(dir.path(), Box::new(move |done| tx.send(done).unwrap()))
        .unwrap();
    let png = png();
    let clip = NewClip {
        kind: ClipKind::Image,
        plain_text: None,
        content_hash: hash("image", &png),
        source_app_name: None,
        source_app_id: None,
        metadata: None,
        platform: Platform::MacOs,
        formats: vec![NewFormat {
            item_index: 0,
            format: "public.png".into(),
            data: png,
        }],
    };
    let hash = clip.content_hash.clone();
    let id = store.upsert_clip_at(clip, NOW - 10 * DAY).unwrap().id;
    let done = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert!(done.result.is_ok());
    assert!(store.thumbnail_path(&hash).unwrap().is_some());

    let limits = Limits {
        max_age_days: Some(1),
        ..UNLIMITED
    };
    assert_eq!(store.apply_retention(&limits, NOW).unwrap(), vec![id]);
    assert!(store.thumbnail_path(&hash).unwrap().is_none());
}

#[test]
fn orphan_sweep_deletes_files_no_clip_uses() {
    let (dir, store) = open();
    let kept = add_with(&store, "kept", vec![big_format(1)], NOW);
    // A blob written before a crash, with no row.
    let orphan = store.blobs().put(&[9; 100 * 1024]).unwrap();
    // A half-written blob from an earlier run, and a file Kopi didn't make.
    let shard = dir.path().join("blobs/ab/cd");
    fs::create_dir_all(&shard).unwrap();
    fs::write(shard.join("abcd.tmp-1-0"), b"partial").unwrap();
    fs::write(shard.join("notes.txt"), b"not ours").unwrap();
    // A thumbnail of a deleted clip, one of an old size, and one that's still in use.
    let used = hash("text", b"kept");
    let gone = "e".repeat(64);
    let thumbs = dir.path().join("thumbs");
    for name in [
        format!("ee/{gone}@320.png"),
        format!("{}/{used}@256.png", &used[..2]),
        format!("{}/{used}@320.png", &used[..2]),
    ] {
        let path = thumbs.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"png").unwrap();
    }

    assert_eq!(store.sweep_orphans().unwrap(), 4);
    assert!(!store.blobs().path(&orphan).unwrap().exists());
    assert!(!shard.join("abcd.tmp-1-0").exists());
    assert!(shard.join("notes.txt").exists());
    assert!(store.thumbnail_path(&gone).unwrap().is_none());
    assert!(store.thumbnail_path(&used).unwrap().is_some());
    assert_eq!(
        store.clip_formats(kept).unwrap()[0].data,
        vec![1; 100 * 1024]
    );

    assert_eq!(store.sweep_orphans().unwrap(), 0);
}

#[test]
fn clear_last_hour_keeps_older_clips_and_kept_ones() {
    let (_dir, store) = open();
    let now = kopi_core::storage::now_ms();
    let old = add_at(&store, "old", now - 2 * 60 * 60 * 1000);
    let recent = add_at(&store, "recent", now - 60 * 1000);
    let pinned = add_at(&store, "pinned", now - 60 * 1000);
    store.set_pinned(pinned, true).unwrap();

    let removed = store
        .clear_history(Some(now - 60 * 60 * 1000), true)
        .unwrap();
    assert_eq!(removed, vec![recent]);
    assert_eq!(ids(&store), vec![old, pinned]);
}

#[test]
fn clear_everything_can_include_kept_clips() {
    let (dir, store) = open();
    let a = add_at(&store, "a", NOW);
    let pinned = add_with(&store, "pinned", vec![big_format(3)], NOW);
    store.set_pinned(pinned, true).unwrap();

    assert_eq!(store.clear_history(None, true).unwrap(), vec![a]);
    assert_eq!(ids(&store), vec![pinned]);

    assert_eq!(store.clear_history(None, false).unwrap(), vec![pinned]);
    assert!(ids(&store).is_empty());
    assert_eq!(count_files(&dir.path().join("blobs")), 0);
}

#[test]
fn clearing_leaves_no_trace_of_the_text_on_disk() {
    let (dir, store) = open();
    let secret = "correct-horse-battery-staple-7f3a";
    // Enough rows that the database spans many pages.
    for i in 0..200 {
        add_at(
            &store,
            &format!("filler clip {i} {}", "x".repeat(200)),
            NOW - 1000,
        );
    }
    add_at(&store, secret, NOW);
    assert!(
        contains(dir.path(), secret),
        "the secret is on disk to begin with"
    );

    store.clear_history(None, false).unwrap();
    assert!(!contains(dir.path(), secret));
    let wal = dir.path().join("kopi.db-wal");
    assert_eq!(fs::metadata(wal).map(|m| m.len()).unwrap_or(0), 0);
}

#[test]
fn the_search_index_purges_deleted_text() {
    let (_dir, store) = open();
    let secure_delete: i64 = store
        .with_reader(|conn| {
            Ok(conn.query_row(
                "SELECT v FROM clips_fts_config WHERE k = 'secure-delete'",
                [],
                |row| row.get(0),
            )?)
        })
        .unwrap();
    assert_eq!(secure_delete, 1);
}

#[test]
fn retention_runs_at_startup_and_at_most_once_an_interval() {
    let dir = tempfile::tempdir().unwrap();
    let store = Arc::new(Store::open(dir.path()).unwrap());
    store
        .set_retention_limits(&Limits {
            max_items: Some(1),
            ..UNLIMITED
        })
        .unwrap();
    let now = kopi_core::storage::now_ms();
    let a = add_at(&store, "a", now - 2000);
    let b = add_at(&store, "b", now - 1000);
    let orphan = store.blobs().put(&[7; 100 * 1024]).unwrap();

    let (tx, rx) = mpsc::channel();
    let interval = Duration::from_millis(400);
    let retention = Retention::spawn(
        Arc::clone(&store),
        interval,
        Box::new(move |result| tx.send(result.unwrap()).unwrap()),
    )
    .unwrap();
    let next = || rx.recv_timeout(Duration::from_secs(5)).unwrap();

    // Startup: the sweep and a run.
    assert_eq!(next(), vec![a]);
    assert!(!store.blobs().path(&orphan).unwrap().exists());

    // Clips arriving within the interval share one run, once it's up.
    let c = add_at(&store, "c", now);
    retention.clips_added();
    let d = add_at(&store, "d", now + 1);
    retention.clips_added();
    assert_eq!(sorted(next()), vec![b, c]);
    assert!(rx.recv_timeout(interval * 2).is_err(), "only one run");
    assert_eq!(ids(&store), vec![d]);

    // Long after the last run, a new clip runs it straight away.
    let e = add_at(&store, "e", now + 2);
    retention.clips_added();
    assert_eq!(rx.recv_timeout(interval / 2).unwrap(), vec![d]);

    // Lowering a limit runs it now.
    store
        .set_retention_limits(&Limits {
            max_age_days: Some(1),
            ..UNLIMITED
        })
        .unwrap();
    let old = add_at(&store, "old", now - 2 * DAY);
    retention.run_now();
    assert_eq!(next(), vec![old]);
    assert_eq!(ids(&store), vec![e]);
}

fn sorted(mut ids: Vec<i64>) -> Vec<i64> {
    ids.sort_unstable();
    ids
}

/// Whether any file in the data folder holds `needle`.
fn contains(dir: &Path, needle: &str) -> bool {
    ["kopi.db", "kopi.db-wal"].iter().any(|name| {
        fs::read(dir.join(name))
            .is_ok_and(|bytes| bytes.windows(needle.len()).any(|w| w == needle.as_bytes()))
    })
}

fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(4, 4, image::Rgba([200, 30, 30, 255]));
    let mut out = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}
