use std::io::Cursor;
use std::sync::mpsc;
use std::time::Duration;

use kopi_core::storage::{
    ClipKind, MAX_INLINE_BYTES, MAX_TEXT_INDEX_BYTES, NewClip, NewFormat, Platform, SCHEMA_VERSION,
    StorageError, Store,
};
use rusqlite::Connection;

fn hash(kind: ClipKind, payload: &[u8]) -> String {
    let mut h = blake3::Hasher::new();
    h.update(kind.as_str().as_bytes());
    h.update(&[0]);
    h.update(payload);
    h.finalize().to_hex().to_string()
}

fn text_clip(text: &str, app: &str) -> NewClip {
    NewClip {
        kind: ClipKind::Text,
        plain_text: Some(text.to_owned()),
        content_hash: hash(ClipKind::Text, text.as_bytes()),
        source_app_name: Some(app.to_owned()),
        source_app_id: Some(format!("com.example.{app}")),
        metadata: None,
        platform: Platform::MacOs,
        formats: vec![format(
            0,
            "public.utf8-plain-text",
            text.as_bytes().to_vec(),
        )],
        local_only: false,
    }
}

fn format(item_index: u32, name: &str, data: Vec<u8>) -> NewFormat {
    NewFormat {
        item_index,
        format: name.to_owned(),
        data,
    }
}

fn raw(dir: &tempfile::TempDir) -> Connection {
    Connection::open(dir.path().join("kopi.db")).unwrap()
}

fn search(store: &Store, query: &str) -> Vec<i64> {
    store
        .with_reader(|conn| {
            let quoted = format!("\"{}\"", query.replace('"', "\"\""));
            let ids = conn
                .prepare("SELECT rowid FROM clips_fts WHERE clips_fts MATCH ?1 ORDER BY rowid")?
                .query_map([quoted], |row| row.get(0))?
                .collect::<Result<_, _>>()?;
            Ok(ids)
        })
        .unwrap()
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_fn(width, height, |x, y| {
        image::Rgba([(x % 256) as u8, (y % 256) as u8, ((x * y) % 256) as u8, 255])
    });
    let mut out = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

#[test]
fn open_creates_the_layout_and_settings() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let store = Store::open(&data).unwrap();

    assert!(data.join("kopi.db").is_file());
    assert!(data.join("blobs").is_dir());
    assert!(data.join("thumbs").is_dir());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&data).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o700);
    }

    let (journal, page_size, version): (String, i64, u32) = store
        .with_reader(|conn| {
            Ok((
                conn.pragma_query_value(None, "journal_mode", |r| r.get(0))?,
                conn.pragma_query_value(None, "page_size", |r| r.get(0))?,
                conn.pragma_query_value(None, "user_version", |r| r.get(0))?,
            ))
        })
        .unwrap();
    assert_eq!(journal, "wal");
    assert_eq!(page_size, 8192);
    assert_eq!(version, SCHEMA_VERSION);

    let tables: Vec<String> = store
        .with_reader(|conn| {
            let names = conn
                .prepare("SELECT name FROM sqlite_schema WHERE type IN ('table','trigger') ORDER BY name")?
                .query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            Ok(names)
        })
        .unwrap();
    for name in [
        "clips",
        "clip_formats",
        "pinboards",
        "pinboard_items",
        "settings",
        "excluded_apps",
        "clips_fts",
        "clips_ai",
        "clips_ad",
        "clips_au",
    ] {
        assert!(tables.iter().any(|t| t == name), "missing {name}");
    }
}

#[test]
fn reopening_keeps_data_and_newer_schemas_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let store = Store::open(dir.path()).unwrap();
        store
            .upsert_clip(text_clip("kept across launches", "Notes"))
            .unwrap()
            .id
    };
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        store.get_clip(id).unwrap().unwrap().plain_text.as_deref(),
        Some("kept across launches")
    );
    drop(store);

    raw(&dir)
        .pragma_update(None, "user_version", SCHEMA_VERSION + 1)
        .unwrap();
    match Store::open(dir.path()) {
        Err(StorageError::NewerSchema { found, supported }) => {
            assert_eq!((found, supported), (SCHEMA_VERSION + 1, SCHEMA_VERSION));
        }
        other => panic!("expected NewerSchema, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn copying_the_same_content_again_moves_it_to_the_top() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let first = store
        .upsert_clip_at(text_clip("hello", "Notes"), 1_000)
        .unwrap();
    assert!(first.inserted);
    let again = store
        .upsert_clip_at(text_clip("hello", "Safari"), 2_000)
        .unwrap();
    assert!(!again.inserted);
    assert_eq!((again.id, &again.uuid), (first.id, &first.uuid));

    let clip = store.get_clip(first.id).unwrap().unwrap();
    assert_eq!(clip.kind, ClipKind::Text);
    assert_eq!(clip.created_at, 1_000);
    assert_eq!(clip.last_used_at, 2_000);
    assert_eq!(clip.use_count, 2);
    assert_eq!(clip.source_app_name.as_deref(), Some("Safari"));
    assert_eq!(clip.uuid.len(), 36);

    // The same words as rich text are a different clip.
    let mut rich = text_clip("hello", "Pages");
    rich.kind = ClipKind::RichText;
    rich.content_hash = hash(ClipKind::RichText, b"hello");
    assert!(store.upsert_clip(rich).unwrap().inserted);

    assert!(store.mark_used(first.id, 3_000).unwrap());
    let clip = store.get_clip(first.id).unwrap().unwrap();
    assert_eq!((clip.last_used_at, clip.use_count), (3_000, 3));
    assert!(!store.mark_used(999_999, 3_000).unwrap());
}

#[test]
fn search_index_follows_inserts_updates_and_deletes() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let url = store
        .upsert_clip(text_clip("https://github.com/code-devanshu/kopi", "Safari"))
        .unwrap();
    let other = store
        .upsert_clip(text_clip("let café = brew();", "Zed"))
        .unwrap();

    assert_eq!(search(&store, "ithub"), vec![url.id]);
    assert_eq!(
        search(&store, "cafe"),
        vec![other.id],
        "diacritics are ignored"
    );
    assert_eq!(
        search(&store, "Safari"),
        vec![url.id],
        "the app name is indexed"
    );

    store
        .upsert_clip(text_clip(
            "https://github.com/code-devanshu/kopi",
            "Firefox",
        ))
        .unwrap();
    assert!(search(&store, "Safari").is_empty());
    assert_eq!(search(&store, "Firefox"), vec![url.id]);

    let conn = raw(&dir);
    conn.execute("DELETE FROM clips WHERE id = ?1", [url.id])
        .unwrap();
    assert!(search(&store, "ithub").is_empty());
    // Fails if the index and the clips table disagree.
    conn.execute(
        "INSERT INTO clips_fts(clips_fts, rank) VALUES ('integrity-check', 1)",
        [],
    )
    .unwrap();
}

#[test]
fn plain_text_copies_are_skipped_and_long_text_is_cut() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    let short = store.upsert_clip(text_clip("short", "Notes")).unwrap();
    assert!(store.clip_formats(short.id).unwrap().is_empty());

    let long = "é".repeat(MAX_TEXT_INDEX_BYTES); // twice the limit in bytes
    let clip = store.upsert_clip(text_clip(&long, "Notes")).unwrap();
    let stored = store.get_clip(clip.id).unwrap().unwrap();
    assert_eq!(
        stored.plain_text.as_ref().unwrap().len(),
        MAX_TEXT_INDEX_BYTES
    );
    let formats = store.clip_formats(clip.id).unwrap();
    assert_eq!(formats.len(), 1);
    assert_eq!(
        formats[0].data,
        long.as_bytes(),
        "the full text is kept for paste"
    );
}

#[test]
fn large_formats_go_to_shared_blob_files() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let big = vec![7u8; MAX_INLINE_BYTES + 1];
    let small = b"{\\rtf1 hi}".to_vec();

    let mut clips = Vec::new();
    for text in ["one", "two"] {
        let mut clip = text_clip(text, "Pages");
        clip.kind = ClipKind::RichText;
        clip.content_hash = hash(ClipKind::RichText, text.as_bytes());
        clip.formats = vec![
            format(0, "public.html", big.clone()),
            format(0, "public.rtf", small.clone()),
        ];
        clips.push(store.upsert_clip(clip).unwrap().id);
    }

    let blob = blake3::hash(&big).to_hex().to_string();
    let path = store.blobs().path(&blob).unwrap();
    assert!(path.ends_with(format!("{}/{}/{blob}.bin", &blob[..2], &blob[2..4])));
    assert!(path.is_file());

    let conn = raw(&dir);
    let (inline, files): (i64, i64) = conn
        .query_row(
            "SELECT count(data), count(blob_hash) FROM clip_formats",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((inline, files), (2, 2));
    let size: i64 = conn
        .query_row(
            "SELECT size_bytes FROM clips WHERE id = ?1",
            [clips[0]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(size as usize, 3 + big.len() + small.len());

    let formats = store.clip_formats(clips[1]).unwrap();
    assert_eq!(
        formats
            .iter()
            .map(|f| f.format.as_str())
            .collect::<Vec<_>>(),
        ["public.html", "public.rtf"]
    );
    assert_eq!(formats[0].data, big);
    assert_eq!(formats[1].data, small);
}

#[test]
fn copying_rich_text_again_replaces_its_formats() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let old_html = vec![1u8; MAX_INLINE_BYTES + 10];
    let new_html = vec![2u8; MAX_INLINE_BYTES + 10];

    let rich = |html: &[u8]| {
        let mut clip = text_clip("styled", "Pages");
        clip.kind = ClipKind::RichText;
        clip.content_hash = hash(ClipKind::RichText, b"styled");
        clip.formats = vec![format(0, "public.html", html.to_vec())];
        clip
    };
    let id = store.upsert_clip(rich(&old_html)).unwrap().id;
    let old_path = store
        .blobs()
        .path(&blake3::hash(&old_html).to_hex())
        .unwrap();
    assert!(old_path.is_file());

    assert!(!store.upsert_clip(rich(&new_html)).unwrap().inserted);
    assert_eq!(store.clip_formats(id).unwrap()[0].data, new_html);
    assert!(!old_path.exists(), "the old blob had no other reference");

    // A plain text clip that is copied again keeps its first formats.
    let mut text = text_clip("plain", "Notes");
    text.formats
        .push(format(0, "public.html", b"<b>first</b>".to_vec()));
    let id = store.upsert_clip(text.clone()).unwrap().id;
    text.formats[1].data = b"<b>second</b>".to_vec();
    store.upsert_clip(text).unwrap();
    assert_eq!(store.clip_formats(id).unwrap()[0].data, b"<b>first</b>");
}

#[test]
fn image_clips_get_a_thumbnail_off_the_calling_thread() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = mpsc::channel();
    let store = Store::open_with_listener(
        dir.path(),
        Box::new(move |done| {
            let _ = tx.send((std::thread::current().name().map(str::to_owned), done));
        }),
    )
    .unwrap();

    let bytes = png(1200, 600);
    assert!(
        bytes.len() > MAX_INLINE_BYTES,
        "the test image should go to a blob file"
    );
    let content_hash = hash(ClipKind::Image, &bytes);
    let clip = NewClip {
        kind: ClipKind::Image,
        plain_text: None,
        content_hash: content_hash.clone(),
        source_app_name: Some("Preview".into()),
        source_app_id: Some("com.apple.Preview".into()),
        metadata: Some(serde_json::json!({ "w": 1200, "h": 600 })),
        platform: Platform::MacOs,
        formats: vec![format(0, "public.png", bytes)],
        local_only: false,
    };
    let id = store.upsert_clip(clip).unwrap().id;

    let (thread, done) = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(thread.as_deref(), Some("kopi-thumbnails"));
    assert_eq!(done.content_hash, content_hash);
    let path = done.result.unwrap();
    assert_eq!(
        store.thumbnail_path(&content_hash).unwrap(),
        Some(path.clone())
    );
    assert!(path.ends_with(format!("{}/{content_hash}@320.png", &content_hash[..2])));
    let thumb = image::open(&path).unwrap();
    assert_eq!((thumb.width(), thumb.height()), (320, 160));

    assert_eq!(
        store.get_clip(id).unwrap().unwrap().metadata,
        Some(serde_json::json!({ "w": 1200, "h": 600 }))
    );

    // It can be asked for again, and a broken image reports an error instead of panicking.
    std::fs::remove_file(&path).unwrap();
    assert!(store.request_thumbnail(id).unwrap());
    rx.recv_timeout(Duration::from_secs(10))
        .unwrap()
        .1
        .result
        .unwrap();
    assert!(path.is_file());

    let junk = b"not a png".to_vec();
    let junk_hash = hash(ClipKind::Image, &junk);
    let mut clip = text_clip("", "Preview");
    clip.kind = ClipKind::Image;
    clip.plain_text = None;
    clip.content_hash = junk_hash.clone();
    clip.formats = vec![format(0, "public.png", junk)];
    store.upsert_clip(clip).unwrap();
    let (_, done) = rx.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(done.content_hash, junk_hash);
    assert!(done.result.is_err());
    assert_eq!(store.thumbnail_path(&junk_hash).unwrap(), None);
}

#[test]
fn deleting_clips_removes_their_rows_files_and_thumbnails() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = mpsc::channel();
    let store = Store::open_with_listener(
        dir.path(),
        Box::new(move |done| {
            let _ = tx.send(done);
        }),
    )
    .unwrap();

    // Two clips share one blob file.
    let big = vec![9u8; MAX_INLINE_BYTES + 1];
    let mut shared = Vec::new();
    for text in ["first", "second"] {
        let mut clip = text_clip(text, "Pages");
        clip.kind = ClipKind::RichText;
        clip.content_hash = hash(ClipKind::RichText, text.as_bytes());
        clip.formats = vec![format(0, "public.html", big.clone())];
        shared.push(store.upsert_clip(clip).unwrap().id);
    }
    let blob = store.blobs().path(&blake3::hash(&big).to_hex()).unwrap();

    let bytes = png(40, 20);
    let image_hash = hash(ClipKind::Image, &bytes);
    let image = store
        .upsert_clip(NewClip {
            kind: ClipKind::Image,
            plain_text: None,
            content_hash: image_hash.clone(),
            source_app_name: None,
            source_app_id: None,
            metadata: None,
            platform: Platform::MacOs,
            formats: vec![format(0, "public.png", bytes)],
            local_only: false,
        })
        .unwrap()
        .id;
    let thumb = rx
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .result
        .unwrap();

    let kept = store.upsert_clip(text_clip("kept", "Notes")).unwrap().id;
    let conn = raw(&dir);
    conn.execute_batch(&format!(
        "UPDATE clips SET pinned = 1 WHERE id = {image};
         INSERT INTO pinboards (uuid, name, position, created_at, updated_at)
         VALUES ('p', 'Board', 0, 0, 0);
         INSERT INTO pinboard_items (pinboard_id, clip_id, position, added_at)
         VALUES (1, {image}, 0, 0), (1, {kept}, 1, 0);"
    ))
    .unwrap();

    assert_eq!(store.clip_id_by_hash(&image_hash).unwrap(), Some(image));

    // Unknown ids are skipped. The blob is still used by the second clip.
    assert_eq!(
        store.delete_clips(&[shared[0], 999_999]).unwrap(),
        [shared[0]]
    );
    assert!(blob.is_file());
    assert_eq!(search(&store, "first"), Vec::<i64>::new());
    assert_eq!(search(&store, "second"), [shared[1]]);

    // Pinned clips and clips on a pinboard go too, when asked for by id.
    assert_eq!(
        store.delete_clips(&[image, shared[1], image]).unwrap(),
        [image, shared[1]]
    );
    assert!(!blob.exists());
    assert!(!thumb.exists());
    assert_eq!(store.thumbnail_path(&image_hash).unwrap(), None);
    assert_eq!(store.get_clip(image).unwrap(), None);
    assert_eq!(store.clip_id_by_hash(&image_hash).unwrap(), None);

    let count = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap();
    assert_eq!(count("SELECT count(*) FROM clips"), 1);
    assert_eq!(count("SELECT count(*) FROM clip_formats"), 0);
    assert_eq!(count("SELECT clip_id FROM pinboard_items"), kept);
    assert_eq!(store.delete_clips(&[]).unwrap(), Vec::<i64>::new());
}

#[test]
fn hashes_must_be_blake3_hex() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();

    for bad in ["", "../../etc/passwd", &"A".repeat(64), &"g".repeat(64)] {
        assert!(matches!(
            store.blobs().path(bad),
            Err(StorageError::InvalidHash(_))
        ));
        assert!(matches!(
            store.thumbnail_path(bad),
            Err(StorageError::InvalidHash(_))
        ));
    }
    let mut clip = text_clip("x", "Notes");
    clip.content_hash = "not-a-hash".into();
    assert!(matches!(
        store.upsert_clip(clip),
        Err(StorageError::InvalidHash(_))
    ));
}

#[test]
fn password_managers_are_excluded_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let apps = store.excluded_apps(Platform::MacOs).unwrap();
    for id in [
        "com.apple.keychainaccess",
        "com.apple.Passwords",
        "com.1password.1password",
        "com.bitwarden.desktop",
        "org.keepassxc.keepassxc",
        "com.dashlane.Dashlane",
        "in.sinew.Enpass-Desktop",
    ] {
        let app = apps.iter().find(|a| a.app_id == id);
        assert!(app.is_some_and(|a| a.is_default), "{id} not seeded");
    }
    assert!(apps.iter().all(|a| a.created_at > 0));

    let apps = store.excluded_apps(Platform::Windows).unwrap();
    for id in [
        "1password.exe",
        "Bitwarden.exe",
        "KeePass.exe",
        "KeePassXC.exe",
    ] {
        let app = apps.iter().find(|a| a.app_id.eq_ignore_ascii_case(id));
        assert!(app.is_some_and(|a| a.is_default), "{id} not seeded");
    }
    assert!(!apps.iter().any(|a| a.app_id.starts_with("com.")));
}

#[test]
fn a_removed_default_stays_removed() {
    let dir = tempfile::tempdir().unwrap();
    {
        let store = Store::open(dir.path()).unwrap();
        assert!(
            store
                .remove_excluded_app(Platform::MacOs, "COM.BITWARDEN.DESKTOP")
                .unwrap()
        );
        assert!(
            !store
                .remove_excluded_app(Platform::MacOs, "com.bitwarden.desktop")
                .unwrap()
        );
    }
    let store = Store::open(dir.path()).unwrap();
    let apps = store.excluded_apps(Platform::MacOs).unwrap();
    assert!(!apps.iter().any(|a| a.app_id == "com.bitwarden.desktop"));
}

#[test]
fn user_excluded_apps_are_added_once() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    assert!(
        store
            .add_excluded_app(Platform::MacOs, "com.example.Vault", "Vault")
            .unwrap()
    );
    assert!(
        !store
            .add_excluded_app(Platform::MacOs, "com.example.vault", "Vault")
            .unwrap()
    );
    assert!(
        !store
            .add_excluded_app(Platform::MacOs, "com.apple.passwords", "Passwords")
            .unwrap()
    );
    let apps = store.excluded_apps(Platform::MacOs).unwrap();
    let vault: Vec<_> = apps.iter().filter(|a| a.app_name == "Vault").collect();
    assert_eq!(vault.len(), 1);
    assert!(!vault[0].is_default);

    // The same ID on the other platform is a different app.
    assert!(
        store
            .add_excluded_app(Platform::Windows, "com.example.Vault", "Vault")
            .unwrap()
    );
}
