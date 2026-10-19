//! `search::search` against a real store.

use kopi_core::search::{
    MAX_CANDIDATES, SearchHit, SearchPage, SearchQuery, SourceApp, search, source_apps,
};
use kopi_core::storage::{ClipKind, NewClip, NewFormat, Platform, Store};
use rusqlite::{Connection, params};

const NOW: i64 = 1_760_000_000_000;
const HOUR: i64 = 60 * 60 * 1000;
const DAY: i64 = 24 * HOUR;

struct Fixture {
    dir: tempfile::TempDir,
    store: Store,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        Self { dir, store }
    }

    /// Stores `text` as copied from `app` at `NOW - ago`.
    fn add(&self, kind: ClipKind, text: &str, app: &str, ago: i64) -> i64 {
        let mut h = blake3::Hasher::new();
        h.update(kind.as_str().as_bytes());
        h.update(&[0]);
        h.update(text.as_bytes());
        let clip = NewClip {
            kind,
            plain_text: (!text.is_empty()).then(|| text.to_owned()),
            content_hash: h.finalize().to_hex().to_string(),
            source_app_name: Some(app.to_owned()),
            source_app_id: Some(format!("com.example.{app}")),
            metadata: None,
            platform: Platform::MacOs,
            formats: vec![NewFormat {
                item_index: 0,
                format: "public.utf8-plain-text".into(),
                data: text.as_bytes().to_vec(),
            }],
            local_only: false,
        };
        self.store.upsert_clip_at(clip, NOW - ago).unwrap().id
    }

    fn text(&self, text: &str, ago: i64) -> i64 {
        self.add(ClipKind::Text, text, "Notes", ago)
    }

    fn run(&self, query: SearchQuery) -> SearchPage {
        search(&self.store, &query).unwrap()
    }

    fn ids(&self, text: &str) -> Vec<i64> {
        ids(&self.run(SearchQuery {
            text: text.into(),
            ..SearchQuery::default()
        }))
    }

    /// A pinboard holding `clips` in that order.
    fn pinboard(&self, clips: &[i64]) -> i64 {
        let conn = Connection::open(self.dir.path().join("kopi.db")).unwrap();
        conn.execute(
            "INSERT INTO pinboards (uuid, name, position, created_at, updated_at)
             VALUES (lower(hex(randomblob(16))), 'Board', 0, 0, 0)",
            [],
        )
        .unwrap();
        let board = conn.last_insert_rowid();
        for (position, clip) in clips.iter().enumerate() {
            conn.execute(
                "INSERT INTO pinboard_items (pinboard_id, clip_id, position, added_at)
                 VALUES (?1, ?2, ?3, 0)",
                params![board, clip, position as i64],
            )
            .unwrap();
        }
        board
    }
}

fn ids(page: &SearchPage) -> Vec<i64> {
    page.hits.iter().map(|h| h.id).collect()
}

/// The snippet as text, with matches in brackets.
fn shown(hit: &SearchHit) -> String {
    hit.snippet
        .iter()
        .map(|s| {
            if s.matched {
                format!("[{}]", s.text)
            } else {
                s.text.clone()
            }
        })
        .collect()
}

#[test]
fn no_text_lists_the_most_recent_first_in_pages() {
    let f = Fixture::new();
    let old = f.text("old", 3 * HOUR);
    let mid = f.text("middle", 2 * HOUR);
    let new = f.text("new", HOUR);
    // Copying again moves it to the top.
    f.text("old", 0);

    let page = f.run(SearchQuery::default());
    assert_eq!(ids(&page), [old, new, mid]);
    assert!(!page.has_more);
    assert_eq!(shown(&page.hits[0]), "old");
    assert_eq!(page.hits[0].kind, ClipKind::Text);
    assert_eq!(page.hits[0].source_app_name.as_deref(), Some("Notes"));
    assert_eq!(page.hits[0].last_used_at, NOW);

    let first = f.run(SearchQuery {
        limit: 2,
        ..SearchQuery::default()
    });
    assert_eq!(ids(&first), [old, new]);
    assert!(first.has_more);
    let second = f.run(SearchQuery {
        limit: 2,
        offset: 2,
        ..SearchQuery::default()
    });
    assert_eq!(ids(&second), [mid]);
    assert!(!second.has_more);

    // Spaces alone are no search.
    assert_eq!(f.ids("  \t "), [old, new, mid]);
}

#[test]
fn words_match_anywhere_ignoring_case_and_accents() {
    let f = Fixture::new();
    let link = f.add(
        ClipKind::Link,
        "https://github.com/code-devanshu/kopi",
        "Safari",
        0,
    );
    let cafe = f.text("Meet at the Café Zürich", HOUR);
    let tokyo = f.text("東京都の天気", 2 * HOUR);

    assert_eq!(f.ids("ithub"), [link]);
    assert_eq!(f.ids("GITHUB.COM/code"), [link]);
    assert_eq!(f.ids("cafe"), [cafe]);
    assert_eq!(f.ids("ZURICH"), [cafe]);
    assert_eq!(f.ids("東京都"), [tokyo]);
    assert_eq!(f.ids("zzzq"), [] as [i64; 0]);
}

#[test]
fn every_word_must_match_in_any_order() {
    let f = Fixture::new();
    let both = f.text("deploy to staging tonight", 0);
    let reversed = f.text("staging is ready, deploy it", HOUR);
    let one = f.text("deploy to production", 2 * HOUR);

    let mut found = f.ids("staging deploy");
    found.sort();
    assert_eq!(found, [both, reversed]);
    assert_eq!(f.ids("deploy production"), [one]);
    assert_eq!(f.ids("deploy nowhere"), [] as [i64; 0]);
}

#[test]
fn short_words_fall_back_to_like() {
    let f = Fixture::new();
    let code = f.add(ClipKind::Code, "fn main() {}", "Zed", 0);
    let percent = f.text("growth of 50% in Q3", HOUR);
    let other = f.text("nothing to see", 2 * HOUR);

    assert_eq!(f.ids("fn"), [code]);
    assert_eq!(f.ids("FN"), [code]);
    assert_eq!(f.ids("q3"), [percent]);
    // Wildcards are literal.
    assert_eq!(f.ids("0%"), [percent]);
    assert_eq!(f.ids("%"), [percent]);
    assert_eq!(f.ids("_"), [] as [i64; 0]);
    // One letter, everywhere it appears, newest first.
    assert_eq!(f.ids("n"), [code, percent, other]);

    let page = f.run(SearchQuery {
        text: "q3".into(),
        ..SearchQuery::default()
    });
    assert_eq!(shown(&page.hits[0]), "growth of 50% in [Q3]");
}

#[test]
fn short_and_long_words_together() {
    let f = Fixture::new();
    let q3 = f.text("invoice for Q3", 0);
    let q4 = f.text("invoice for Q4", HOUR);

    assert_eq!(f.ids("invoice q3"), [q3]);
    assert_eq!(f.ids("q4 invoice"), [q4]);
    assert_eq!(f.ids("invoice q5"), [] as [i64; 0]);
}

#[test]
fn the_app_name_is_searched_too() {
    let f = Fixture::new();
    let slack = f.add(ClipKind::Text, "see you tomorrow", "Slack", 0);
    f.add(ClipKind::Text, "see you later", "Mail", HOUR);
    let image = f.add(ClipKind::Image, "", "Slack", 2 * HOUR);

    let page = f.run(SearchQuery {
        text: "slack".into(),
        ..SearchQuery::default()
    });
    let hit = |id| page.hits.iter().find(|h| h.id == id).unwrap();
    assert_eq!(page.hits.len(), 2);
    // Nothing in the text matched, so it starts at the beginning.
    assert_eq!(shown(hit(slack)), "see you tomorrow");
    assert_eq!(hit(image).snippet, []);
    assert_eq!(hit(image).kind, ClipKind::Image);
}

#[test]
fn snippets_mark_the_matches() {
    let f = Fixture::new();
    let long = format!(
        "{} the deploy key is in the vault {}",
        "lorem ipsum ".repeat(40),
        "dolor sit ".repeat(40)
    );
    f.text(&long, 0);
    f.text("Deploy, then deploy again", HOUR);

    let page = f.run(SearchQuery {
        text: "deploy".into(),
        ..SearchQuery::default()
    });
    let snippets: Vec<String> = page.hits.iter().map(shown).collect();
    assert!(
        snippets.contains(&"[Deploy], then [deploy] again".to_owned()),
        "{snippets:?}"
    );
    let cut = snippets.iter().find(|s| s.starts_with('…')).unwrap();
    assert!(cut.contains("the [deploy] key"), "{cut}");
    assert!(cut.ends_with('…'), "{cut}");
    assert!(cut.chars().count() < 100, "{cut}");
}

#[test]
fn filters_apply_to_every_kind_of_search() {
    let f = Fixture::new();
    let text = f.add(ClipKind::Text, "release notes fn", "Notes", 0);
    let code = f.add(ClipKind::Code, "fn release() {}", "Zed", HOUR);
    let link = f.add(
        ClipKind::Link,
        "https://example.com/release",
        "Safari",
        2 * HOUR,
    );

    let by_kind = |text: &str, kind| {
        ids(&f.run(SearchQuery {
            text: text.into(),
            kind: Some(kind),
            ..SearchQuery::default()
        }))
    };
    assert_eq!(by_kind("", ClipKind::Code), [code]);
    assert_eq!(by_kind("release", ClipKind::Link), [link]);
    assert_eq!(by_kind("fn", ClipKind::Text), [text]);
    assert_eq!(by_kind("release", ClipKind::Image), [] as [i64; 0]);

    let by_app = |text: &str, app: &str| {
        ids(&f.run(SearchQuery {
            text: text.into(),
            source_app_id: Some(app.into()),
            ..SearchQuery::default()
        }))
    };
    assert_eq!(by_app("", "com.example.Safari"), [link]);
    assert_eq!(by_app("release", "com.example.Zed"), [code]);
    assert_eq!(by_app("fn", "com.example.Notes"), [text]);
    assert_eq!(by_app("release", "com.example.Nope"), [] as [i64; 0]);

    // A pinboard lists in its own order, and searches within it.
    let board = f.pinboard(&[link, text]);
    let in_board = |text: &str| {
        ids(&f.run(SearchQuery {
            text: text.into(),
            pinboard_id: Some(board),
            ..SearchQuery::default()
        }))
    };
    assert_eq!(in_board(""), [link, text]);
    assert_eq!(in_board("release"), [text, link]);
    assert_eq!(in_board("fn"), [text]);
    let both = f.run(SearchQuery {
        pinboard_id: Some(board),
        kind: Some(ClipKind::Text),
        ..SearchQuery::default()
    });
    assert_eq!(ids(&both), [text]);

    // Pinned clips, on every path.
    assert!(f.store.set_pinned(code, true).unwrap());
    assert!(f.store.set_pinned(text, true).unwrap());
    let pinned = |text: &str| {
        ids(&f.run(SearchQuery {
            text: text.into(),
            pinned: true,
            ..SearchQuery::default()
        }))
    };
    assert_eq!(pinned(""), [text, code]);
    assert_eq!(pinned("release"), [text, code]);
    assert_eq!(pinned("fn"), [text, code]);
    assert_eq!(pinned("notes"), [text]);
    assert_eq!(pinned("Safari"), [] as [i64; 0]);
}

#[test]
fn ranking_blends_the_match_with_recency() {
    let f = Fixture::new();
    // The same word, a day apart: the newer comes first.
    let older = f.text("budget review", DAY);
    let newer = f.text("budget meeting", 0);
    assert_eq!(f.ids("budget"), [newer, older]);

    // A clip that is mostly the word beats a long text that mentions it, a little later.
    let f = Fixture::new();
    let exact = f.text("kubernetes", HOUR);
    let mention = f.text(
        &format!(
            "{} kubernetes {}",
            "log line ".repeat(200),
            "more ".repeat(200)
        ),
        0,
    );
    assert_eq!(f.ids("kubernetes"), [exact, mention]);
}

#[test]
fn index_results_page() {
    let f = Fixture::new();
    let all: Vec<i64> = (0..7)
        .map(|i| f.text(&format!("report {i}"), i * HOUR))
        .collect();
    let page = |offset, limit| {
        f.run(SearchQuery {
            text: "report".into(),
            offset,
            limit,
            ..SearchQuery::default()
        })
    };
    let first = page(0, 3);
    assert_eq!(ids(&first), all[..3]);
    assert!(first.has_more);
    let last = page(6, 3);
    assert_eq!(ids(&last), all[6..]);
    assert!(!last.has_more);
    let past = page(10, 3);
    assert!(past.hits.is_empty());
    assert!(!past.has_more);
    // Zero means the default size, and the size is capped.
    assert_eq!(page(0, 0).hits.len(), 7);
    assert_eq!(page(0, 100_000).hits.len(), 7);
}

#[test]
fn typed_syntax_is_searched_for_literally() {
    let f = Fixture::new();
    let quoted = f.text(r#"she said "hello there""#, 0);
    let syntax = f.text("NEAR(a b) AND col:x * ^start {a b}:c", HOUR);

    assert_eq!(f.ids(r#""hello"#), [quoted]);
    assert_eq!(f.ids(r#"said "hello there""#), [quoted]);
    assert_eq!(f.ids("NEAR(a"), [syntax]);
    assert_eq!(f.ids("col:x"), [syntax]);
    assert_eq!(f.ids("^start"), [syntax]);
    assert_eq!(f.ids("}:c"), [syntax]);
    for text in [
        "\"",
        "\"\"\"",
        "*",
        "***",
        "AND",
        "OR NOT",
        "NEAR(",
        "a:b:c",
        "(((",
        "\0\0\0",
        "a\0bc",
        "'; DROP TABLE clips; --",
        "%%%",
        "\\\\\\",
        "\u{FDD0}\u{FDD1}x",
    ] {
        search(
            &f.store,
            &SearchQuery {
                text: text.into(),
                ..SearchQuery::default()
            },
        )
        .unwrap_or_else(|e| panic!("{text:?}: {e}"));
    }
    assert_eq!(f.run(SearchQuery::default()).hits.len(), 2);
}

#[test]
fn only_the_newest_matches_are_ranked() {
    let f = Fixture::new();
    let extra = 30;
    let total = MAX_CANDIDATES + extra;
    // Oldest first, so newer clips have higher ids, as when copying for real. The same length
    // keeps the order down to recency.
    let mut clips: Vec<i64> = (0..total)
        .rev()
        .map(|i| f.text(&format!("note {i:04}"), i as i64 * 60_000))
        .collect();
    clips.reverse();
    f.add(ClipKind::Text, "unrelated", "Mail", 0);

    let page = |offset, limit| {
        f.run(SearchQuery {
            text: "note".into(),
            offset,
            limit,
            ..SearchQuery::default()
        })
    };
    let first = page(0, 50);
    assert_eq!(ids(&first), clips[..50]);
    assert!(first.has_more);
    assert_eq!(shown(&first.hits[0]), "[note] 0000");
    assert_eq!(shown(&first.hits[49]), "[note] 0049");

    let middle = page(500, 50);
    assert_eq!(ids(&middle), clips[500..550]);

    // The oldest matches past the cap aren't reached.
    let last = page(MAX_CANDIDATES as u32 - 20, 50);
    assert_eq!(ids(&last), clips[MAX_CANDIDATES - 20..MAX_CANDIDATES]);
    assert!(!last.has_more);

    // A short word still finds them, through the recent clips. "unrelated" is first.
    assert_eq!(
        ids(&f.run(SearchQuery {
            text: "e".into(),
            offset: total as u32,
            ..SearchQuery::default()
        })),
        [clips[total - 1]]
    );
}

#[test]
fn source_apps_most_recent_first() {
    let f = Fixture::new();
    assert_eq!(source_apps(&f.store, 10).unwrap(), []);

    f.add(ClipKind::Text, "a", "Mail", 3 * HOUR);
    f.add(ClipKind::Text, "b", "Notes", 2 * HOUR);
    f.add(ClipKind::Text, "c", "Mail", HOUR);
    let unknown = f.add(ClipKind::Text, "d", "Ghost", 0);
    let conn = Connection::open(f.dir.path().join("kopi.db")).unwrap();
    conn.execute(
        "UPDATE clips SET source_app_id = NULL WHERE id = ?1",
        [unknown],
    )
    .unwrap();
    // The newest clip's name wins when an app's name changed.
    let renamed = f.add(ClipKind::Text, "e", "Notes", 30 * 60 * 1000);
    conn.execute(
        "UPDATE clips SET source_app_name = 'Notes 2' WHERE id = ?1",
        [renamed],
    )
    .unwrap();

    let app = |id: &str, name: &str| SourceApp {
        id: format!("com.example.{id}"),
        name: Some(name.to_owned()),
    };
    assert_eq!(
        source_apps(&f.store, 10).unwrap(),
        [app("Notes", "Notes 2"), app("Mail", "Mail")]
    );
    assert_eq!(source_apps(&f.store, 1).unwrap(), [app("Notes", "Notes 2")]);
}
