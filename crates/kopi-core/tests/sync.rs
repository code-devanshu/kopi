//! Merging changes between stores, without a network: what `kopi-sync` does over a session.

use kopi_core::classifier::content_hash;
use kopi_core::storage::sync::Changes;
use kopi_core::storage::{ClipKind, NewClip, NewFormat, Platform, StorageResult, Store};

const PLAIN: &str = "public.utf8-plain-text";

struct Node {
    _dir: tempfile::TempDir,
    store: Store,
    id: String,
}

impl Node {
    fn new(id: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        store
            .set_self_device(id, &key(id), &format!("{id} computer"), Platform::MacOs)
            .unwrap();
        Self {
            _dir: dir,
            store,
            id: id.to_owned(),
        }
    }

    fn copy(&self, text: &str) -> i64 {
        self.store.upsert_clip(text_clip(text)).unwrap().id
    }

    fn id_of(&self, text: &str) -> Option<i64> {
        self.store
            .clip_id_by_hash(&content_hash(ClipKind::Text, text.as_bytes()))
            .unwrap()
    }

    /// Clips, pinboards and pinboard items, in a form that should match across computers.
    fn state(&self) -> (Vec<String>, Vec<String>, Vec<String>) {
        self.store
            .with_reader(|conn| {
                let rows = |sql: &str| -> StorageResult<Vec<String>> {
                    Ok(conn
                        .prepare(sql)?
                        .query_map([], |row| row.get(0))?
                        .collect::<Result<_, _>>()?)
                };
                Ok((
                    rows("SELECT content_hash || ' ' || last_used_at || ' ' || pinned || ' '
                                 || created_at || ' ' || coalesce(source_app_id, '-')
                          FROM clips ORDER BY content_hash")?,
                    rows("SELECT uuid || ' ' || name || ' ' || coalesce(color, '-') || ' ' || position
                          FROM pinboards ORDER BY uuid")?,
                    rows("SELECT b.uuid || ' ' || c.content_hash || ' ' || i.position
                          FROM pinboard_items i JOIN pinboards b ON b.id = i.pinboard_id
                          JOIN clips c ON c.id = i.clip_id ORDER BY 1")?,
                ))
            })
            .unwrap()
    }
}

/// A made-up 32-byte public key, different for each device.
fn key(id: &str) -> Vec<u8> {
    blake3::hash(id.as_bytes()).as_bytes().to_vec()
}

fn text_clip(text: &str) -> NewClip {
    NewClip {
        kind: ClipKind::Text,
        plain_text: Some(text.to_owned()),
        content_hash: content_hash(ClipKind::Text, text.as_bytes()),
        source_app_name: Some("Notes".into()),
        source_app_id: Some("com.apple.Notes".into()),
        metadata: None,
        platform: Platform::MacOs,
        formats: vec![NewFormat {
            item_index: 0,
            format: PLAIN.into(),
            data: text.as_bytes().to_vec(),
        }],
        local_only: false,
    }
}

/// Pairs every node with every other, as pairing and the group would.
fn group(nodes: &[&Node]) {
    for a in nodes {
        for b in nodes {
            if a.id != b.id {
                a.store
                    .pair_device(
                        &b.id,
                        &key(&b.id),
                        &format!("{} computer", b.id),
                        Platform::Windows,
                    )
                    .unwrap();
            }
        }
    }
}

/// Sends everything `from` has that `to` hasn't applied, as a session's catch-up does.
/// Returns how many batches it took.
fn sync(from: &Node, to: &Node) -> usize {
    let mut batches = 0;
    loop {
        let after = to
            .store
            .sync_devices()
            .unwrap()
            .into_iter()
            .find(|d| d.device_id == from.id)
            .map_or(0, |d| d.received_seq);
        let changes = from
            .store
            .changes_since(after, to.store.sync_kinds().unwrap(), &from.id)
            .unwrap();
        if changes.max_seq == after {
            return batches;
        }
        to.store.apply_changes(&from.id, &to.id, changes).unwrap();
        batches += 1;
        assert!(batches < 1000, "sync never settled");
    }
}

/// Syncs every pair both ways until nothing changes.
fn settle(nodes: &[&Node]) {
    for _ in 0..10 {
        let mut batches = 0;
        for a in nodes {
            for b in nodes {
                if a.id != b.id {
                    batches += sync(a, b);
                }
            }
        }
        if batches == 0 {
            return;
        }
    }
    panic!("echoes never stopped");
}

#[test]
fn a_copy_reaches_the_other_computer() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    a.copy("hello from a");
    settle(&[&a, &b]);

    let id = b.id_of("hello from a").expect("synced");
    let clip = b.store.get_clip(id).unwrap().unwrap();
    assert_eq!(clip.plain_text.as_deref(), Some("hello from a"));
    assert_eq!(clip.source_app_name.as_deref(), Some("Notes"));
    assert_eq!(clip.copied_on.as_deref(), Some("a computer"));
    assert_eq!(clip.use_count, 1);
    // On the computer it was copied on, it was copied here.
    let original = a
        .store
        .get_clip(a.id_of("hello from a").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(original.copied_on, None);
    assert_eq!(a.state(), b.state());
}

#[test]
fn large_formats_and_images_sync_whole() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    let text = "x".repeat(100_000);
    a.copy(&text);
    let png = image_png();
    let hash = content_hash(ClipKind::Image, &png);
    a.store
        .upsert_clip(NewClip {
            kind: ClipKind::Image,
            plain_text: None,
            content_hash: hash.clone(),
            source_app_name: None,
            source_app_id: None,
            metadata: Some(serde_json::json!({ "w": 300, "h": 300 })),
            platform: Platform::MacOs,
            formats: vec![NewFormat {
                item_index: 0,
                format: "public.png".into(),
                data: png.clone(),
            }],
            local_only: false,
        })
        .unwrap();
    settle(&[&a, &b]);

    let id = b
        .store
        .clip_id_by_hash(&hash)
        .unwrap()
        .expect("image synced");
    let formats = b.store.clip_formats(id).unwrap();
    assert_eq!(formats.len(), 1);
    assert_eq!(formats[0].data, png);
    let long = b.id_of(&text).expect("long text synced");
    let full: Vec<u8> = b
        .store
        .clip_formats(long)
        .unwrap()
        .into_iter()
        .flat_map(|f| f.data)
        .collect();
    assert_eq!(full.len(), 100_000);
}

#[test]
fn deletes_reach_everyone_and_dont_come_back() {
    let (a, b, c) = (Node::new("a"), Node::new("b"), Node::new("c"));
    group(&[&a, &b, &c]);
    a.copy("secret");
    a.copy("keep me");
    settle(&[&a, &b, &c]);
    assert!(c.id_of("secret").is_some());

    // Deleted on B. C is away, then syncs only with A.
    b.store.delete_clips(&[b.id_of("secret").unwrap()]).unwrap();
    sync(&b, &a);
    assert_eq!(a.id_of("secret"), None);
    sync(&c, &a);
    assert_eq!(a.id_of("secret"), None, "C's copy must not bring it back");
    sync(&a, &c);
    assert_eq!(c.id_of("secret"), None);
    settle(&[&a, &b, &c]);
    assert!(a.id_of("keep me").is_some());
    assert_eq!(a.state(), b.state());
    assert_eq!(a.state(), c.state());

    // Copying it again afterwards brings it back everywhere.
    c.copy("secret");
    settle(&[&a, &b, &c]);
    assert!(a.id_of("secret").is_some());
    assert!(b.id_of("secret").is_some());
}

#[test]
fn a_delete_beats_an_older_pin() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    let id = a.copy("pinned");
    a.store.set_pinned(id, true).unwrap();
    settle(&[&a, &b]);
    b.store.delete_clips(&[b.id_of("pinned").unwrap()]).unwrap();
    settle(&[&a, &b]);
    assert_eq!(a.id_of("pinned"), None);
}

#[test]
fn clear_history_clears_everywhere() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    a.copy("one");
    let pinned = a.copy("pinned");
    a.store.set_pinned(pinned, true).unwrap();
    settle(&[&a, &b]);

    b.store.clear_history(None, true).unwrap();
    settle(&[&a, &b]);
    assert_eq!(a.id_of("one"), None);
    assert!(a.id_of("pinned").is_some(), "kept clips stay when asked");

    // A copy made after the clear survives it.
    a.copy("after");
    settle(&[&a, &b]);
    assert!(b.id_of("after").is_some());
    assert_eq!(a.state(), b.state());
}

#[test]
fn the_latest_use_and_pin_win() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    a.copy("shared");
    settle(&[&a, &b]);

    let on_b = b.id_of("shared").unwrap();
    b.store.set_pinned(on_b, true).unwrap();
    b.store
        .mark_used(on_b, kopi_core::storage::now_ms() + 10)
        .unwrap();
    settle(&[&a, &b]);
    let on_a = a
        .store
        .get_clip(a.id_of("shared").unwrap())
        .unwrap()
        .unwrap();
    assert!(on_a.pinned);
    assert_eq!(a.state(), b.state());

    let on_a_id = on_a.id;
    a.store.set_pinned(on_a_id, false).unwrap();
    settle(&[&a, &b]);
    assert!(!b.store.get_clip(on_b).unwrap().unwrap().pinned);
}

#[test]
fn pinboards_and_their_clips_sync() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    let one = a.copy("one");
    let two = a.copy("two");
    let board = a.store.create_pinboard("Work", Some("#d08a3a")).unwrap();
    a.store.add_to_pinboard(board.id, &[one, two]).unwrap();
    settle(&[&a, &b]);
    assert_eq!(a.state(), b.state());
    let on_b = b.store.pinboards().unwrap();
    assert_eq!(on_b.len(), 1);
    assert_eq!(on_b[0].name, "Work");
    assert_eq!(on_b[0].clip_count, 2);

    b.store.rename_pinboard(on_b[0].id, "Home").unwrap();
    b.store
        .remove_from_pinboard(on_b[0].id, &[b.id_of("one").unwrap()])
        .unwrap();
    settle(&[&a, &b]);
    let on_a = a.store.pinboard(board.id).unwrap().unwrap();
    assert_eq!(on_a.name, "Home");
    assert_eq!(on_a.clip_count, 1);
    assert_eq!(a.state(), b.state());

    a.store.delete_pinboard(board.id).unwrap();
    settle(&[&a, &b]);
    assert!(b.store.pinboards().unwrap().is_empty());
    assert!(b.id_of("one").is_some(), "its clips stay in the history");
}

#[test]
fn an_item_brings_its_clip_back() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    let old = a.copy("old");
    settle(&[&a, &b]);
    // B's limits removed it, which isn't a delete.
    b.store
        .apply_retention(
            &kopi_core::storage::Limits {
                max_items: Some(1),
                max_age_days: None,
                max_total_bytes: None,
            },
            kopi_core::storage::now_ms(),
        )
        .unwrap();
    b.copy("newer");
    b.store
        .apply_retention(
            &kopi_core::storage::Limits {
                max_items: Some(1),
                max_age_days: None,
                max_total_bytes: None,
            },
            kopi_core::storage::now_ms(),
        )
        .unwrap();
    assert_eq!(b.id_of("old"), None);

    let board = a.store.create_pinboard("Keep", None).unwrap();
    a.store.add_to_pinboard(board.id, &[old]).unwrap();
    settle(&[&a, &b]);
    let id = b.id_of("old").expect("the pinboard needs it");
    assert!(b.store.is_kept(id).unwrap());
}

#[test]
fn local_only_files_and_unwanted_kinds_stay_put() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    a.store
        .upsert_clip(NewClip {
            local_only: true,
            ..text_clip("windows said no")
        })
        .unwrap();
    let path = "/Users/me/report.pdf";
    a.store
        .upsert_clip(NewClip {
            kind: ClipKind::File,
            plain_text: Some("report.pdf".into()),
            content_hash: content_hash(ClipKind::File, path.as_bytes()),
            source_app_name: None,
            source_app_id: None,
            metadata: None,
            platform: Platform::MacOs,
            formats: vec![NewFormat {
                item_index: 0,
                format: "public.file-url".into(),
                data: format!("file://{path}").into_bytes(),
            }],
            local_only: false,
        })
        .unwrap();
    b.store
        .set_sync_kinds(kopi_core::settings::SYNCABLE_KINDS.without(ClipKind::Link))
        .unwrap();
    a.store
        .upsert_clip(NewClip {
            kind: ClipKind::Link,
            plain_text: Some("https://kopi.app".into()),
            content_hash: content_hash(ClipKind::Link, b"https://kopi.app"),
            source_app_name: None,
            source_app_id: None,
            metadata: Some(serde_json::json!({ "host": "kopi.app" })),
            platform: Platform::MacOs,
            formats: vec![NewFormat {
                item_index: 0,
                format: PLAIN.into(),
                data: b"https://kopi.app".to_vec(),
            }],
            local_only: false,
        })
        .unwrap();
    a.copy("fine");
    settle(&[&a, &b]);
    let (clips, _, _) = b.state();
    assert_eq!(clips.len(), 1, "only the plain text clip: {clips:?}");
    assert!(b.id_of("fine").is_some());
}

#[test]
fn forged_hashes_and_strangers_data_are_refused() {
    let (a, b) = (Node::new("a"), Node::new("b"));
    group(&[&a, &b]);
    a.copy("real");
    let mut changes = a
        .store
        .changes_since(0, b.store.sync_kinds().unwrap(), &a.id)
        .unwrap();
    let clip = changes.clips.first_mut().unwrap();
    clip.formats[0].data = b"swapped".to_vec();
    let applied = b.store.apply_changes(&a.id, &b.id, changes).unwrap();
    assert_eq!(applied.rejected, 1);
    assert!(
        b.store
            .with_reader(|c| Ok(
                c.query_row("SELECT count(*) FROM clips", [], |r| r.get::<_, i64>(0))?
            ))
            .unwrap()
            == 0
    );

    // A device can't change another's key.
    let mut changes = Changes::default();
    changes.devices.push(kopi_core::storage::sync::SyncDevice {
        device_id: "a".into(),
        public_key: key("mallory"),
        name: "a computer".into(),
        os: "macos".into(),
        removed: false,
        updated_hlc: i64::MAX / 2,
    });
    b.store.apply_changes(&a.id, &b.id, changes).unwrap();
    let a_on_b = b
        .store
        .sync_devices()
        .unwrap()
        .into_iter()
        .find(|d| d.device_id == "a")
        .unwrap();
    assert_eq!(a_on_b.public_key, key("a"));
}

#[test]
fn group_changes_spread() {
    let (a, b, c) = (Node::new("a"), Node::new("b"), Node::new("c"));
    // C pairs only with B.
    a.store
        .pair_device("b", &key("b"), "b computer", Platform::Windows)
        .unwrap();
    b.store
        .pair_device("a", &key("a"), "a computer", Platform::MacOs)
        .unwrap();
    b.store
        .pair_device("c", &key("c"), "c computer", Platform::MacOs)
        .unwrap();
    c.store
        .pair_device("b", &key("b"), "b computer", Platform::Windows)
        .unwrap();
    sync(&b, &a);
    let names = |n: &Node| {
        n.store
            .sync_devices()
            .unwrap()
            .into_iter()
            .filter(|d| !d.removed)
            .map(|d| d.device_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&a), ["a", "b", "c"]);

    a.store.remove_device("c").unwrap();
    sync(&a, &b);
    assert_eq!(names(&b), ["a", "b"]);
}

#[test]
fn random_changes_in_any_order_converge() {
    for seed in 1..=25 {
        random_run(seed);
    }
}

fn random_run(seed: u64) {
    let nodes = [Node::new("a"), Node::new("b"), Node::new("c")];
    let refs: Vec<&Node> = nodes.iter().collect();
    group(&refs);
    let mut rng = 0x2545_f491_4f6c_dd1du64 ^ seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let mut next = move |n: u64| {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng % n
    };
    let words = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];
    for round in 0..40 {
        let node = &nodes[next(3) as usize];
        let word = words[next(words.len() as u64) as usize];
        match next(9) {
            0 | 1 => {
                node.copy(word);
            }
            2 => {
                if let Some(id) = node.id_of(word) {
                    node.store.delete_clips(&[id]).unwrap();
                }
            }
            3 => {
                if let Some(id) = node.id_of(word) {
                    node.store.set_pinned(id, next(2) == 0).unwrap();
                }
            }
            4 => {
                let boards = node.store.pinboards().unwrap();
                match (boards.first(), node.id_of(word)) {
                    (Some(board), Some(id)) => {
                        node.store.add_to_pinboard(board.id, &[id]).unwrap();
                    }
                    (None, _) => {
                        node.store
                            .create_pinboard(&format!("board {round}"), None)
                            .unwrap();
                    }
                    _ => {}
                }
            }
            5 => {
                if let Some(board) = node.store.pinboards().unwrap().first() {
                    if next(3) == 0 {
                        node.store.delete_pinboard(board.id).unwrap();
                    } else {
                        node.store
                            .rename_pinboard(board.id, &format!("renamed {round}"))
                            .unwrap();
                    }
                }
            }
            6 => {
                // The last few milliseconds, or everything, with or without kept clips.
                let since = (next(2) == 0).then(|| kopi_core::storage::now_ms() - next(4) as i64);
                node.store.clear_history(since, next(2) == 0).unwrap();
            }
            _ => {
                if let Some(id) = node.id_of(word) {
                    node.store
                        .mark_used(id, kopi_core::storage::now_ms())
                        .unwrap();
                }
            }
        }
        // Now and then, two computers that happen to be on together sync.
        if next(3) == 0 {
            let (x, y) = (next(3) as usize, next(3) as usize);
            if x != y {
                sync(&nodes[x], &nodes[y]);
            }
        }
    }
    settle(&refs);
    assert_eq!(nodes[0].state(), nodes[1].state(), "seed {seed}");
    assert_eq!(nodes[0].state(), nodes[2].state(), "seed {seed}");
}

/// A real 300×300 PNG, over the 64 KiB inline limit.
fn image_png() -> Vec<u8> {
    let mut pixels = image::RgbImage::new(300, 300);
    for (x, y, p) in pixels.enumerate_pixels_mut() {
        *p = image::Rgb([
            (x * 7 % 256) as u8,
            (y * 13 % 256) as u8,
            ((x ^ y) % 256) as u8,
        ]);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    pixels.write_to(&mut out, image::ImageFormat::Png).unwrap();
    let out = out.into_inner();
    assert!(out.len() > 64 * 1024, "{}", out.len());
    out
}

#[test]
fn history_from_before_sync_is_ready_to_send() {
    let dir = tempfile::tempdir().unwrap();
    {
        // A database as Kopi 0.1.0-alpha.7 left it: schema 3, with history.
        let conn = rusqlite::Connection::open(dir.path().join("kopi.db")).unwrap();
        for sql in [
            include_str!("../migrations/0001_init.sql"),
            include_str!("../migrations/0002_default_excluded_apps.sql"),
            include_str!("../migrations/0003_windows_excluded_apps.sql"),
        ] {
            conn.execute_batch(sql).unwrap();
        }
        conn.pragma_update(None, "user_version", 3).unwrap();
        for (i, text) in ["first", "second", "third"].iter().enumerate() {
            conn.execute(
                "INSERT INTO clips (uuid, kind, plain_text, content_hash, size_bytes, created_at,
                                    last_used_at, pinned)
                 VALUES (?1, 'text', ?2, ?3, 5, ?4, ?4, ?5)",
                rusqlite::params![
                    format!("u{i}"),
                    text,
                    content_hash(ClipKind::Text, text.as_bytes()),
                    1_700_000_000_000i64 + i as i64,
                    i == 1,
                ],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO pinboards (uuid, name, position, created_at, updated_at)
             VALUES ('0190a000-0000-7000-8000-000000000000', 'Old', 0, 1, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pinboard_items (pinboard_id, clip_id, position, added_at) VALUES (1, 1, 0, 2)",
            [],
        )
        .unwrap();
    }

    let a = Node {
        store: Store::open(dir.path()).unwrap(),
        _dir: dir,
        id: "a".into(),
    };
    a.store
        .set_self_device("a", &key("a"), "a computer", Platform::MacOs)
        .unwrap();
    let b = Node::new("b");
    group(&[&a, &b]);
    settle(&[&a, &b]);
    assert_eq!(a.state(), b.state());
    let (clips, boards, items) = b.state();
    assert_eq!((clips.len(), boards.len(), items.len()), (3, 1, 1));
    // The clock carries on after the newest timestamp.
    let id = a.copy("fourth");
    assert!(a.store.get_clip(id).unwrap().unwrap().last_used_at > 1_700_000_000_002);
}
