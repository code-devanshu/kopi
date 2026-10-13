//! Turning the search box into terms. What was typed never reaches SQLite as query syntax.

/// Typing past this many bytes searches only for the start.
pub(super) const MAX_TEXT_BYTES: usize = 1024;

/// Words past this many are ignored.
pub(super) const MAX_TERMS: usize = 16;

/// The trigram index can't match fewer characters than this.
const MIN_INDEXED_CHARS: usize = 3;

/// The words of a search, split by what can use the index.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Terms {
    /// Words of 3 or more characters.
    pub long: Vec<String>,
    /// Words of 1 or 2 characters.
    pub short: Vec<String>,
}

impl Terms {
    /// Splits `text` on whitespace. Control characters are dropped, and a repeated word (case
    /// aside) is kept once.
    pub fn parse(text: &str) -> Self {
        let mut end = text.len().min(MAX_TEXT_BYTES);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        let mut terms = Self::default();
        let mut seen = Vec::new();
        for word in text[..end].split_whitespace() {
            let word: String = word.chars().filter(|c| !c.is_control()).collect();
            let folded = word.to_lowercase();
            if word.is_empty() || seen.contains(&folded) {
                continue;
            }
            if seen.len() == MAX_TERMS {
                break;
            }
            seen.push(folded);
            if word.chars().count() >= MIN_INDEXED_CHARS {
                terms.long.push(word);
            } else {
                terms.short.push(word);
            }
        }
        terms
    }

    /// The long words as FTS5 string literals, all of which must match.
    pub fn fts_query(&self) -> String {
        let quoted: Vec<String> = self
            .long
            .iter()
            .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
            .collect();
        quoted.join(" ")
    }
}

/// A `LIKE … ESCAPE '\'` pattern that matches `term` anywhere.
pub(super) fn like_pattern(term: &str) -> String {
    let mut pattern = String::with_capacity(term.len() + 2);
    pattern.push('%');
    for c in term.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_words_by_length() {
        let cases: &[(&str, &[&str], &[&str])] = &[
            ("", &[], &[]),
            ("   \t\n ", &[], &[]),
            ("deploy", &["deploy"], &[]),
            ("  deploy  staging ", &["deploy", "staging"], &[]),
            ("invoice q3", &["invoice"], &["q3"]),
            ("a", &[], &["a"]),
            ("fn x", &[], &["fn", "x"]),
            ("東京", &[], &["東京"]),
            ("東京都", &["東京都"], &[]),
            ("🙂🙂🙂", &["🙂🙂🙂"], &[]),
            ("é", &[], &["é"]),
            ("café", &["café"], &[]),
            ("Deploy deploy DEPLOY", &["Deploy"], &[]),
            ("ab AB", &[], &["ab"]),
            ("a\0b", &[], &["ab"]),
            ("\0\0", &[], &[]),
            ("x\u{7}yz", &["xyz"], &[]),
            ("https://github.com/org", &["https://github.com/org"], &[]),
        ];
        for (text, long, short) in cases {
            let terms = Terms::parse(text);
            assert_eq!(terms.long, *long, "{text:?}");
            assert_eq!(terms.short, *short, "{text:?}");
        }
    }

    #[test]
    fn long_text_is_cut() {
        let words: Vec<String> = (0..40).map(|i| format!("word{i}")).collect();
        let terms = Terms::parse(&words.join(" "));
        assert_eq!(terms.long.len(), MAX_TERMS);
        assert_eq!(terms.long[0], "word0");

        let text = "é".repeat(MAX_TEXT_BYTES);
        let terms = Terms::parse(&text);
        assert_eq!(terms.long, ["é".repeat(MAX_TEXT_BYTES / 2)]);
    }

    #[test]
    fn fts_literals_are_escaped() {
        let cases: &[(&str, &str)] = &[
            ("deploy", r#""deploy""#),
            ("deploy staging", r#""deploy" "staging""#),
            (r#"say "hi" now"#, r#""say" """hi""" "now""#),
            ("\"\"\"", "\"\"\"\"\"\"\"\""),
            ("NEAR(a b)", r#""NEAR(a""#),
            ("abc*", r#""abc*""#),
            ("^abc", r#""^abc""#),
            ("col:abc", r#""col:abc""#),
            ("{a b}:abc", r#""b}:abc""#),
            ("AND OR NOT", r#""AND" "NOT""#),
            ("-abc +abc", r#""-abc" "+abc""#),
        ];
        for (text, want) in cases {
            assert_eq!(Terms::parse(text).fts_query(), *want, "{text:?}");
        }
    }

    #[test]
    fn like_patterns_are_escaped() {
        let cases: &[(&str, &str)] = &[
            ("ab", "%ab%"),
            ("5%", r"%5\%%"),
            ("a_", r"%a\_%"),
            (r"\n", r"%\\n%"),
            ("é", "%é%"),
        ];
        for (term, want) in cases {
            assert_eq!(like_pattern(term), *want, "{term:?}");
        }
    }
}
