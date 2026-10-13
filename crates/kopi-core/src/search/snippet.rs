//! The text shown for each result, with the matches marked.

/// Marks the start and end of a match in FTS5's `snippet()`. Unicode keeps these noncharacters
/// for internal use, so copied text won't hold them.
pub(super) const OPEN: char = '\u{FDD0}';
pub(super) const CLOSE: char = '\u{FDD1}';

/// The most FTS5's `snippet()` allows. Trigram tokens are characters, so this is the length.
pub(super) const TOKENS: usize = 64;

/// The length of the text shown when there's no search, and of a `LIKE` match's snippet.
pub(super) const PREVIEW_CHARS: usize = 200;

/// How much of the text before a `LIKE` match to show, when the match is past the preview.
const CONTEXT_CHARS: usize = 16;

/// A run of snippet text. `matched` runs are what the search found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub matched: bool,
}

/// Splits the output of `snippet()` at the [`OPEN`] and [`CLOSE`] marks.
pub(super) fn parse(marked: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut text = String::new();
    let mut matched = false;
    for c in marked.chars() {
        let now = match c {
            OPEN => true,
            CLOSE => false,
            _ => {
                text.push(c);
                continue;
            }
        };
        if now != matched {
            push(&mut spans, std::mem::take(&mut text), matched);
            matched = now;
        }
    }
    push(&mut spans, text, matched);
    spans
}

/// The start of `text`, unmarked.
pub(super) fn preview(text: Option<&str>) -> Vec<Span> {
    let mut spans = Vec::new();
    if let Some(text) = text {
        push(&mut spans, cut(text, 0, PREVIEW_CHARS), false);
    }
    spans
}

/// The text around the first match of any of `terms`, with every match in it marked. It starts
/// at the beginning when the first match is in the preview. Case is
/// ignored for ASCII letters, as SQLite's `LIKE` ignores it. The start of the text if nothing
/// matches, which happens when only the app name did.
pub(super) fn around(text: Option<&str>, terms: &[String]) -> Vec<Span> {
    let Some(text) = text else {
        return Vec::new();
    };
    // ASCII lowercasing keeps every byte offset.
    let lower = text.to_ascii_lowercase();
    let needles: Vec<String> = terms
        .iter()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect();
    let Some(first) = needles.iter().filter_map(|n| lower.find(n.as_str())).min() else {
        return preview(Some(text));
    };

    // Terms are at most 2 characters, so a match that starts in the preview ends in it.
    let start = if text[..first].chars().count() + 2 <= PREVIEW_CHARS {
        0
    } else {
        text[..first]
            .char_indices()
            .rev()
            .nth(CONTEXT_CHARS - 1)
            .map_or(0, |(i, _)| i)
    };
    let end = text[start..]
        .char_indices()
        .nth(PREVIEW_CHARS)
        .map_or(text.len(), |(i, _)| start + i);

    let mut spans = Vec::new();
    let mut at = start;
    if start > 0 {
        push(&mut spans, "…".to_owned(), false);
    }
    // Leftmost match first, the longest at the same place, never overlapping.
    while let Some((s, e)) = needles
        .iter()
        .filter_map(|n| {
            let s = at + lower[at..end].find(n.as_str())?;
            Some((s, s + n.len()))
        })
        .min_by_key(|&(s, e)| (s, std::cmp::Reverse(e)))
    {
        push(&mut spans, text[at..s].to_owned(), false);
        push(&mut spans, text[s..e].to_owned(), true);
        at = e;
    }
    push(&mut spans, text[at..end].to_owned(), false);
    if end < text.len() {
        push(&mut spans, "…".to_owned(), false);
    }
    spans
}

/// `chars` characters of `text` from byte `start`, with `…` when there's more.
fn cut(text: &str, start: usize, chars: usize) -> String {
    match text[start..].char_indices().nth(chars) {
        Some((i, _)) => format!("{}…", &text[start..start + i]),
        None => text[start..].to_owned(),
    }
}

/// Appends `text`, joining it to the last span when that's the same kind.
fn push(spans: &mut Vec<Span>, text: String, matched: bool) {
    if text.is_empty() {
        return;
    }
    match spans.last_mut() {
        Some(last) if last.matched == matched => last.text.push_str(&text),
        _ => spans.push(Span { text, matched }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spans as text, with matches in brackets.
    fn show(spans: &[Span]) -> String {
        spans
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
    fn parses_marks() {
        let cases: &[(&str, &str)] = &[
            ("", ""),
            ("plain", "plain"),
            ("a \u{FDD0}b\u{FDD1} c", "a [b] c"),
            ("\u{FDD0}all\u{FDD1}", "[all]"),
            (
                "…x \u{FDD0}one\u{FDD1} and \u{FDD0}two\u{FDD1}…",
                "…x [one] and [two]…",
            ),
            ("\u{FDD0}ab\u{FDD1}\u{FDD0}cd\u{FDD1}", "[abcd]"),
            ("\u{FDD0}\u{FDD1}x", "x"),
            ("x\u{FDD1}y", "xy"),
            ("x\u{FDD0}y", "x[y]"),
        ];
        for (marked, want) in cases {
            assert_eq!(show(&parse(marked)), *want, "{marked:?}");
        }
        assert_eq!(
            parse("a \u{FDD0}b\u{FDD1}"),
            [
                Span {
                    text: "a ".into(),
                    matched: false
                },
                Span {
                    text: "b".into(),
                    matched: true
                },
            ]
        );
    }

    #[test]
    fn previews() {
        assert_eq!(preview(None), []);
        assert_eq!(preview(Some("")), []);
        assert_eq!(show(&preview(Some("short"))), "short");
        let long = "é".repeat(PREVIEW_CHARS + 1);
        assert_eq!(
            show(&preview(Some(&long))),
            format!("{}…", "é".repeat(PREVIEW_CHARS))
        );
        let exact = "x".repeat(PREVIEW_CHARS);
        assert_eq!(show(&preview(Some(&exact))), exact);
    }

    #[test]
    fn marks_short_matches() {
        let t = |terms: &[&str]| terms.iter().map(|t| (*t).to_owned()).collect::<Vec<_>>();
        let cases: &[(&str, &[&str], &str)] = &[
            ("fn main", &["fn"], "[fn] main"),
            ("Fn main FN", &["fn"], "[Fn] main [FN]"),
            ("a b a", &["a"], "[a] b [a]"),
            ("abab", &["ab", "a"], "[abab]"),
            ("xab", &["a", "ab"], "x[ab]"),
            ("q3 report", &["q3", "x"], "[q3] report"),
            ("東京 tower", &["東京"], "[東京] tower"),
            ("ÉCOLE école", &["é"], "ÉCOLE [é]cole"),
            ("100%", &["0%"], "10[0%]"),
            ("no match here", &["zz"], "no match here"),
            ("", &["a"], ""),
        ];
        for (text, terms, want) in cases {
            assert_eq!(show(&around(Some(text), &t(terms))), *want, "{text:?}");
        }
        assert_eq!(around(None, &t(&["a"])), []);
    }

    #[test]
    fn short_matches_keep_some_context() {
        let text = format!("{}fn{}", "x".repeat(300), "y".repeat(300));
        let shown = show(&around(Some(&text), &["fn".to_owned()]));
        let want = format!(
            "…{}[fn]{}…",
            "x".repeat(CONTEXT_CHARS),
            "y".repeat(PREVIEW_CHARS - CONTEXT_CHARS - 2)
        );
        assert_eq!(shown, want);

        // In the preview: from the start.
        let text = format!("{}fn{}", "x".repeat(PREVIEW_CHARS - 2), "y".repeat(10));
        let shown = show(&around(Some(&text), &["fn".to_owned()]));
        assert_eq!(shown, format!("{}[fn]…", "x".repeat(PREVIEW_CHARS - 2)));
        let text = format!("{}fn", "x".repeat(PREVIEW_CHARS - 1));
        let shown = show(&around(Some(&text), &["fn".to_owned()]));
        assert!(shown.starts_with('…') && shown.ends_with("[fn]"), "{shown}");

        let text = format!("ab{}", "é".repeat(PREVIEW_CHARS));
        let shown = show(&around(Some(&text), &["ab".to_owned()]));
        assert_eq!(shown, format!("[ab]{}…", "é".repeat(PREVIEW_CHARS - 2)));
    }
}
