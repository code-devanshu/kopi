//! Measures insert, search and startup with a large history.
//!
//! ```sh
//! cargo run --release -p kopi-core --example storage_bench -- 100000
//! ```
//!
//! Clips are made up from a fixed seed: mostly short text, plus links, code and some long
//! pastes. Each clip is its own transaction, as when copying for real. Searches go through
//! `kopi_core::search`, 50 clips a page with snippets, as the panel asks for them. The words
//! come from a small vocabulary, so most searches match a large share of the history: a worse
//! case than real use.

use std::path::Path;
use std::time::{Duration, Instant};

use kopi_core::search::SearchQuery;
use kopi_core::storage::{ClipKind, NewClip, NewFormat, Platform, Store};

const WORDS: &[&str] = &[
    "the",
    "quick",
    "brown",
    "fox",
    "invoice",
    "meeting",
    "tomorrow",
    "deploy",
    "staging",
    "password",
    "reset",
    "kubernetes",
    "github",
    "pull",
    "request",
    "review",
    "budget",
    "q3",
    "café",
    "naïve",
    "résumé",
    "straße",
    "東京",
    "東京都",
    "emoji",
    "🙂",
    "clipboard",
    "history",
    "search",
    "function",
    "const",
    "return",
    "async",
    "await",
    "error",
    "result",
    "option",
    "vector",
];
const APPS: &[(&str, &str)] = &[
    ("Safari", "com.apple.Safari"),
    ("Notes", "com.apple.Notes"),
    ("Zed", "dev.zed.Zed"),
    ("Terminal", "com.apple.Terminal"),
    ("Slack", "com.tinyspeck.slackmacgap"),
    ("Mail", "com.apple.mail"),
];
const QUERIES: &[&str] = &[
    "ithub",
    "deploy",
    "invoice q3",
    "async fn",
    "cafe",
    "東京都",
    "kubern",
    "pull/",
    "zzzq",
    "the quick",
    "return",
    "staging.",
    "Slack",
    "résumé",
    "x7y",
];
const SHORT_QUERIES: &[&str] = &["a", "x", "#4", "fn", "🙂", "東京", "zq"];
const MIXED_QUERIES: &[&str] = &["invoice q3", "fn handler", "pull 7", "deploy a", "zzzq x"];

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Between `min` and `min + spread - 1` random words.
fn sentence(rng: &mut Rng, min: usize, spread: usize) -> String {
    let words = min + rng.below(spread);
    let mut out = String::new();
    for i in 0..words {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(WORDS[rng.below(WORDS.len())]);
    }
    out
}

fn make_clip(rng: &mut Rng, i: usize) -> NewClip {
    let roll = rng.below(100);
    let (kind, text) = match roll {
        0..=59 => (ClipKind::Text, format!("{} #{i}", sentence(rng, 3, 25))),
        60..=79 => (
            ClipKind::Link,
            format!(
                "https://github.com/org{}/repo-{}/pull/{i}",
                rng.below(500),
                rng.below(5000)
            ),
        ),
        80..=94 => (
            ClipKind::Code,
            format!(
                "async fn handler_{i}() -> Result<(), Error> {{\n    let {} = {}.await?;\n    return Ok(());\n}}",
                WORDS[rng.below(WORDS.len())],
                sentence(rng, 4, 1).replace(' ', "_"),
            ),
        ),
        // Long pastes: logs and documents, 2 to 40 KiB.
        _ => (ClipKind::Text, sentence(rng, 300, 6000)),
    };
    let (app, app_id) = APPS[rng.below(APPS.len())];
    let mut hasher = blake3::Hasher::new();
    hasher.update(kind.as_str().as_bytes());
    hasher.update(&[0]);
    hasher.update(text.as_bytes());
    NewClip {
        kind,
        content_hash: hasher.finalize().to_hex().to_string(),
        source_app_name: Some(app.into()),
        source_app_id: Some(app_id.into()),
        metadata: None,
        platform: Platform::MacOs,
        formats: vec![NewFormat {
            item_index: 0,
            format: "public.utf8-plain-text".into(),
            data: text.clone().into_bytes(),
        }],
        plain_text: Some(text),
        local_only: false,
    }
}

fn percentiles(mut samples: Vec<Duration>) -> String {
    samples.sort();
    let at = |p: f64| samples[((samples.len() - 1) as f64 * p) as usize];
    format!(
        "p50 {:>8.3} ms  p95 {:>8.3} ms  p99 {:>8.3} ms  max {:>8.3} ms",
        ms(at(0.50)),
        ms(at(0.95)),
        ms(at(0.99)),
        ms(*samples.last().unwrap())
    )
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn mib(path: &Path) -> f64 {
    std::fs::metadata(path).map_or(0.0, |m| m.len() as f64 / 1024.0 / 1024.0)
}

/// One search as the panel runs it, returning how many clips it found.
fn search(store: &Store, query: &SearchQuery) -> usize {
    kopi_core::search::search(store, query).unwrap().hits.len()
}

/// Runs each query 20 times and prints the timings.
fn bench_searches(store: &Store, name: &str, queries: &[&str], filter: &SearchQuery) {
    let query = |text: &str| SearchQuery {
        text: text.into(),
        ..filter.clone()
    };
    for q in queries {
        search(store, &query(q));
    }
    println!("\nSearch: {name} (20 runs per query)");
    let mut all = Vec::new();
    for q in queries {
        let mut samples = Vec::new();
        let mut rows = 0;
        for _ in 0..20 {
            let t = Instant::now();
            rows = search(store, &query(q));
            samples.push(t.elapsed());
        }
        all.extend(samples.iter().copied());
        samples.sort();
        println!(
            "  {:<12} {:>2} rows  p50 {:>8.3} ms  max {:>8.3} ms",
            format!("{q:?}"),
            rows,
            ms(samples[10]),
            ms(samples[19])
        );
    }
    println!("  all queries: {}", percentiles(all));
}

fn main() {
    let count: usize = std::env::args()
        .nth(1)
        .map_or(100_000, |n| n.parse().expect("a clip count"));
    let dir = tempfile::tempdir().unwrap();
    println!(
        "Kopi storage benchmark: {count} clips in {}",
        dir.path().display()
    );

    // Insert.
    let store = Store::open(dir.path()).unwrap();
    let mut rng = Rng(0x5eed_c0ffee);
    let mut samples = Vec::with_capacity(count);
    let mut text_bytes = 0usize;
    let start = Instant::now();
    let base = 1_760_000_000_000i64;
    for i in 0..count {
        let clip = make_clip(&mut rng, i);
        text_bytes += clip.plain_text.as_ref().map_or(0, String::len);
        let t = Instant::now();
        store.upsert_clip_at(clip, base + i as i64).unwrap();
        samples.push(t.elapsed());
    }
    let total = start.elapsed();
    println!("\nInsert, one transaction per clip");
    println!(
        "  total {:.2} s, {:.0} clips/s",
        total.as_secs_f64(),
        count as f64 / total.as_secs_f64()
    );
    println!("  {}", percentiles(samples));
    println!(
        "  plain text {:.1} MiB",
        text_bytes as f64 / 1024.0 / 1024.0
    );

    // Duplicates: copy something already in the history again.
    let mut rng = Rng(0x5eed_c0ffee);
    let mut samples = Vec::new();
    for i in 0..1000 {
        let clip = make_clip(&mut rng, i);
        let t = Instant::now();
        let done = store
            .upsert_clip_at(clip, base + (count + i) as i64)
            .unwrap();
        samples.push(t.elapsed());
        assert!(!done.inserted);
    }
    println!("\nCopy again (bump an existing clip), 1,000 times");
    println!("  {}", percentiles(samples));

    // Search, 50 clips a page with snippets, as the panel asks for them.
    let none = SearchQuery::default();
    bench_searches(&store, "3+ characters", QUERIES, &none);
    bench_searches(
        &store,
        "3+ characters, code only",
        QUERIES,
        &SearchQuery {
            kind: Some(ClipKind::Code),
            ..none.clone()
        },
    );
    bench_searches(
        &store,
        "3+ characters, from Slack only",
        QUERIES,
        &SearchQuery {
            source_app_id: Some("com.tinyspeck.slackmacgap".into()),
            ..none.clone()
        },
    );
    bench_searches(&store, "1-2 characters", SHORT_QUERIES, &none);
    bench_searches(
        &store,
        "1-2 characters, code only",
        SHORT_QUERIES,
        &SearchQuery {
            kind: Some(ClipKind::Code),
            ..none.clone()
        },
    );
    bench_searches(&store, "mixed", MIXED_QUERIES, &none);
    bench_searches(
        &store,
        "page 3 (offset 100)",
        &["deploy", "github", "fn"],
        &SearchQuery {
            offset: 100,
            ..none.clone()
        },
    );

    // Size on disk, after a checkpoint so the WAL is folded in.
    drop(store);
    {
        let conn = rusqlite::Connection::open(dir.path().join("kopi.db")).unwrap();
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
            .unwrap();
        let fts: Result<i64, _> = conn.query_row(
            "SELECT sum(pgsize) FROM dbstat WHERE name LIKE 'clips_fts%'",
            [],
            |r| r.get(0),
        );
        println!("\nOn disk");
        println!("  kopi.db {:.1} MiB", mib(&dir.path().join("kopi.db")));
        if let Ok(fts) = fts {
            println!(
                "  of which the search index {:.1} MiB",
                fts as f64 / 1024.0 / 1024.0
            );
        }
    }

    // Startup: open the existing store and load the first screen.
    let mut opens = Vec::new();
    let mut firsts = Vec::new();
    for _ in 0..10 {
        let t = Instant::now();
        let store = Store::open(dir.path()).unwrap();
        opens.push(t.elapsed());
        search(&store, &SearchQuery::default());
        firsts.push(t.elapsed());
    }
    println!("\nStartup (warm page cache, 10 runs)");
    println!("  open + migrate check   {}", percentiles(opens));
    println!("  + 50 most recent clips {}", percentiles(firsts));
}
