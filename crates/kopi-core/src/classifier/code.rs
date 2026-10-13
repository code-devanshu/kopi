//! Source code: whether text reads as code, and a guess at its language.
//!
//! A heuristic, tuned to keep prose out more than to catch every snippet. Code that slips
//! through is still stored and searchable as text. A paragraph shown as code would look broken.

use serde_json::Value;

/// Only the start of a long text is looked at.
const SAMPLE_BYTES: usize = 16 * 1024;
/// Longer JSON isn't parsed to check it.
const MAX_JSON_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Code {
    /// `None` when no language stands out.
    pub lang: Option<&'static str>,
}

/// `Some` if `text` reads as code.
pub(super) fn detect(text: &str) -> Option<Code> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(code) = structured(text) {
        return Some(code);
    }
    let sample = sample(text);
    let raw: Vec<&str> = sample.lines().filter(|l| !l.trim().is_empty()).collect();
    let lines: Vec<&str> = raw.iter().map(|l| l.trim()).collect();
    let is_code = match lines.as_slice() {
        [] => false,
        [line] if command(line) => {
            return Some(Code {
                lang: Some("shell"),
            });
        }
        [line] => strong_line(line),
        lines => {
            // A line indented under a code line that opens a block is code too: the body of
            // `def f(x):` or `if x {`.
            let indent = |l: &str| l.len() - l.trim_start().len();
            let in_block = |i: usize| {
                i > 0
                    && indent(raw[i]) > indent(raw[i - 1])
                    && lines[i - 1].ends_with([':', '{'])
                    && code_line(lines[i - 1])
            };
            let code = (0..lines.len())
                .filter(|&i| in_block(i) || code_line(lines[i]))
                .count();
            let prose = (0..lines.len())
                .filter(|&i| !in_block(i) && prose_line(lines[i]))
                .count();
            code >= 2 && code * 2 >= lines.len() && code > prose
        }
    };
    is_code.then(|| Code {
        lang: language(sample, &lines),
    })
}

fn sample(text: &str) -> &str {
    if text.len() <= SAMPLE_BYTES {
        return text;
    }
    let mut end = SAMPLE_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Formats that are code by their shape: a shebang script, JSON, markup and SQL.
fn structured(text: &str) -> Option<Code> {
    let code = |lang| Some(Code { lang });
    let first_line = text.lines().next().unwrap_or_default();
    if let Some(interpreter) = first_line.strip_prefix("#!")
        && interpreter.trim_start().starts_with('/')
    {
        return code(shebang(interpreter));
    }

    let json_shaped = (text.starts_with('{') && text.ends_with('}'))
        || (text.starts_with('[') && text.ends_with(']'));
    if json_shaped && text.len() <= MAX_JSON_BYTES {
        let non_empty = match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(map)) => !map.is_empty(),
            Ok(Value::Array(items)) => !items.is_empty(),
            _ => false,
        };
        if non_empty {
            return code(Some("json"));
        }
    }

    if text.starts_with('<') && text.ends_with('>') {
        let head = text.get(..text.len().min(64)).unwrap_or(text);
        let head = head.to_ascii_lowercase();
        if head.starts_with("<?xml") {
            return code(Some("xml"));
        }
        if head.starts_with("<!doctype html") || head.starts_with("<html") {
            return code(Some("html"));
        }
        let tag: String = head[1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let closed = text.contains("</") || text.contains("/>");
        if closed && tag.starts_with(|c: char| c.is_ascii_alphabetic()) {
            let lang = if HTML_TAGS.contains(&tag.as_str()) {
                "html"
            } else {
                "xml"
            };
            return code(Some(lang));
        }
    }

    // Uppercase keywords, or any case for a statement that ends with `;`.
    let any_case = text.ends_with(';');
    let is = |word: &str, list: &[&str]| {
        list.iter().any(|k| {
            if any_case {
                word.eq_ignore_ascii_case(k)
            } else {
                word == *k
            }
        })
    };
    let first_word = text.split_whitespace().next().unwrap_or_default();
    if is(first_word, SQL_STARTS)
        && text
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .skip(1)
            .any(|w| is(w, SQL_CLAUSES))
    {
        return code(Some("sql"));
    }
    None
}

fn shebang(interpreter: &str) -> Option<&'static str> {
    let mut words = interpreter.split_whitespace();
    let program = words.next()?.rsplit('/').next()?;
    let program = if program == "env" {
        words.find(|w| !w.starts_with('-'))?
    } else {
        program
    };
    match program {
        p if p.starts_with("python") => Some("python"),
        "bash" | "sh" | "zsh" | "fish" => Some("shell"),
        "node" | "deno" | "bun" => Some("javascript"),
        "ruby" => Some("ruby"),
        "perl" => Some("perl"),
        _ => None,
    }
}

const HTML_TAGS: &[&str] = &[
    "a", "article", "aside", "b", "body", "br", "button", "code", "div", "em", "footer", "form",
    "h1", "h2", "h3", "h4", "h5", "h6", "head", "header", "i", "img", "input", "label", "li",
    "link", "main", "meta", "nav", "ol", "option", "p", "pre", "script", "section", "select",
    "span", "strong", "style", "table", "td", "template", "th", "tr", "ul",
];

const SQL_STARTS: &[&str] = &[
    "SELECT", "INSERT", "UPDATE", "DELETE", "CREATE", "ALTER", "DROP", "WITH", "PRAGMA",
];
const SQL_CLAUSES: &[&str] = &[
    "FROM", "INTO", "SET", "TABLE", "INDEX", "VIEW", "WHERE", "VALUES", "TRIGGER",
];

/// Keywords that start a line of code. Lowercase only: prose starts its lines with "If" and
/// "For", code with `if` and `for`.
const KEYWORDS: &[&str] = &[
    "async",
    "await",
    "case",
    "catch",
    "class",
    "const",
    "def",
    "elif",
    "else",
    "enum",
    "export",
    "extern",
    "fn",
    "for",
    "from",
    "func",
    "function",
    "if",
    "impl",
    "import",
    "interface",
    "let",
    "match",
    "mod",
    "package",
    "private",
    "protected",
    "pub",
    "public",
    "return",
    "static",
    "struct",
    "switch",
    "trait",
    "try",
    "type",
    "unsafe",
    "use",
    "val",
    "var",
    "while",
];

/// Operators that hardly appear outside code.
const OPERATORS: &[&str] = &[
    "=>", "::", "==", "!=", "&&", "||", ":=", "+=", "-=", "();", "){", ") {", "->",
];

/// Commands that make a one-line shell snippet.
const COMMANDS: &[&str] = &[
    "brew",
    "cargo",
    "cd",
    "chmod",
    "chown",
    "curl",
    "docker",
    "gh",
    "git",
    "grep",
    "kubectl",
    "ls",
    "mkdir",
    "npm",
    "npx",
    "pip",
    "pip3",
    "pnpm",
    "python3",
    "rm",
    "rustup",
    "scp",
    "ssh",
    "sudo",
    "tar",
    "wget",
    "xcodebuild",
    "yarn",
];

fn keyword_start(line: &str) -> bool {
    KEYWORDS.iter().any(|k| {
        line.strip_prefix(k)
            .is_some_and(|rest| rest.starts_with([' ', '(', ':', '{', '<']))
    })
}

fn has_operator(line: &str) -> bool {
    OPERATORS.iter().any(|op| line.contains(op))
}

/// A whole line that's a call: `foo()`, `console.log("hi");`.
fn call(line: &str) -> bool {
    let Some(open) = line.find('(') else {
        return false;
    };
    let name = &line[..open];
    !name.is_empty()
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '.'))
        && (line.ends_with(')') || line.ends_with(");"))
}

/// A shell command: `$ anything`, or a known command with arguments.
fn command(line: &str) -> bool {
    if line
        .strip_prefix("$ ")
        .is_some_and(|rest| !rest.trim().is_empty())
    {
        return true;
    }
    let mut words = line.split_whitespace();
    let first = words.next().unwrap_or_default();
    COMMANDS.contains(&first) && words.next().is_some() && !line.ends_with(['.', '!', '?'])
}

/// One line, on its own, that's code. Needs more than a line among others.
fn strong_line(line: &str) -> bool {
    (line.ends_with(';') && (line.contains('=') || line.contains('(') || keyword_start(line)))
        || has_operator(line)
        || call(line)
        || (keyword_start(line) && line.contains(['(', '{', '=', ';', '<', '[']))
}

/// A line that counts as code among others.
fn code_line(line: &str) -> bool {
    line.ends_with([';', '{', '}', '(', '['])
        || line
            .chars()
            .all(|c| matches!(c, '}' | ')' | ']' | ';' | ','))
        || (keyword_start(line) && line.contains(['(', ')', '{', '}', '=', ':', ';', '<', '[']))
        || has_operator(line)
        || line.contains(" = ")
        || call(line)
        || [
            "//", "/*", "*/", "#[", "#include", "#define", "#import", "$ ",
        ]
        .iter()
        .any(|p| line.starts_with(p))
        || (line.starts_with('<') && line.ends_with('>'))
}

/// A line of four or more words with nothing code-like in it.
fn prose_line(line: &str) -> bool {
    !code_line(line) && line.split_whitespace().count() >= 4
}

/// Markers of each language, with weights. A language needs a score of at least
/// [`MIN_SCORE`] and must beat the next best.
const LANGUAGES: &[(&str, &[(&str, u32)])] = &[
    (
        "rust",
        &[
            ("fn ", 2),
            ("let mut ", 3),
            ("impl ", 3),
            ("pub fn ", 3),
            ("pub struct ", 3),
            ("println!", 3),
            ("use std::", 3),
            ("&mut ", 2),
            ("#[", 2),
            ("::", 1),
            ("Some(", 1),
            ("Ok(", 1),
        ],
    ),
    (
        "python",
        &[
            ("def ", 3),
            ("elif ", 3),
            ("__init__", 3),
            ("self.", 2),
            ("print(", 2),
            ("lambda ", 2),
            ("):\n", 2),
            ("None", 1),
            ("True", 1),
            ("import ", 1),
        ],
    ),
    (
        "javascript",
        &[
            ("console.", 3),
            ("document.", 3),
            ("require(", 3),
            ("===", 3),
            ("export default", 3),
            ("function ", 2),
            ("=> ", 2),
            ("const ", 2),
            ("undefined", 2),
            ("let ", 1),
        ],
    ),
    (
        "go",
        &[
            ("package ", 3),
            (":= ", 3),
            ("fmt.", 3),
            ("err != nil", 3),
            ("func ", 2),
            ("chan ", 2),
        ],
    ),
    (
        "swift",
        &[
            ("import SwiftUI", 5),
            ("import Foundation", 5),
            ("some View", 5),
            ("@State", 4),
            ("guard ", 3),
            ("func ", 2),
            ("let ", 1),
            ("var ", 1),
        ],
    ),
    (
        "c",
        &[("#include", 4), ("printf(", 3), ("int main", 3), ("->", 1)],
    ),
    (
        "cpp",
        &[
            ("std::", 2),
            ("cout", 3),
            ("nullptr", 3),
            ("template<", 3),
            ("#include <iostream>", 3),
        ],
    ),
    (
        "shell",
        &[
            ("$ ", 3),
            ("sudo ", 3),
            ("fi\n", 3),
            ("then\n", 2),
            ("echo ", 2),
            ("export ", 1),
        ],
    ),
];

/// Markers that make JavaScript TypeScript. They add to the JavaScript score.
const TYPESCRIPT: &[(&str, u32)] = &[
    (": string", 3),
    (": number", 3),
    (": boolean", 3),
    ("export type ", 3),
    ("as const", 3),
    ("interface ", 2),
    ("readonly ", 2),
];

const MIN_SCORE: u32 = 3;

fn language(sample: &str, lines: &[&str]) -> Option<&'static str> {
    if css(lines) {
        return Some("css");
    }
    let score = |markers: &[(&str, u32)]| -> u32 {
        markers
            .iter()
            .filter(|(marker, _)| sample.contains(marker))
            .map(|(_, weight)| weight)
            .sum()
    };
    let mut scores: Vec<(&'static str, u32)> = LANGUAGES
        .iter()
        .map(|(lang, markers)| (*lang, score(markers)))
        .collect();
    let typescript = score(TYPESCRIPT);
    if typescript > 0 {
        let javascript = scores
            .iter()
            .find(|(lang, _)| *lang == "javascript")
            .map_or(0, |(_, s)| *s);
        scores.push(("typescript", javascript + typescript));
    }
    scores.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
    match scores.as_slice() {
        [(lang, best), rest @ ..]
            if *best >= MIN_SCORE && rest.first().is_none_or(|r| r.1 < *best) =>
        {
            Some(lang)
        }
        _ => None,
    }
}

/// Every line is a selector that opens a block, a closing brace, or a `property: value;`.
fn css(lines: &[&str]) -> bool {
    let property = |line: &str| {
        line.strip_suffix(';')
            .and_then(|l| l.split_once(':'))
            .is_some_and(|(name, value)| {
                !name.is_empty()
                    && name.chars().all(|c| c.is_ascii_lowercase() || c == '-')
                    && !value.trim().is_empty()
            })
    };
    lines.iter().any(|l| l.ends_with('{'))
        && lines.iter().any(|l| property(l))
        && !lines.iter().any(|l| keyword_start(l))
        && lines
            .iter()
            .all(|l| l.ends_with('{') || *l == "}" || property(l))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_and_language() {
        // (text, None if not code, Some(lang) if code)
        let cases: &[(&str, Option<Option<&str>>)] = &[
            // Rust
            (
                "fn main() {\n    let x = 5;\n    println!(\"{x}\");\n}",
                Some(Some("rust")),
            ),
            (
                "impl Store {\n    pub fn open(dir: &Path) -> Result<Self> {\n        todo!()\n    }\n}",
                Some(Some("rust")),
            ),
            ("use std::collections::HashMap;", Some(Some("rust"))),
            // Python
            (
                "def greet(name):\n    print(f\"Hello, {name}\")\n    return name.upper()",
                Some(Some("python")),
            ),
            (
                "class Point:\n    def __init__(self, x, y):\n        self.x = x\n        self.y = y",
                Some(Some("python")),
            ),
            // JavaScript and TypeScript
            (
                "const add = (a, b) => a + b;\nconsole.log(add(1, 2));",
                Some(Some("javascript")),
            ),
            ("console.log(\"hello\")", Some(Some("javascript"))),
            (
                "interface User {\n  name: string;\n  age: number;\n}",
                Some(Some("typescript")),
            ),
            // Go and Swift
            (
                "package main\n\nfunc main() {\n\tx := 1\n\tfmt.Println(x)\n}",
                Some(Some("go")),
            ),
            (
                "import SwiftUI\n\nstruct ContentView: View {\n    var body: some View {\n        Text(\"Hi\")\n    }\n}",
                Some(Some("swift")),
            ),
            // C and C++
            (
                "#include <stdio.h>\nint main(void) {\n    printf(\"hi\\n\");\n    return 0;\n}",
                Some(Some("c")),
            ),
            (
                "#include <iostream>\nint main() {\n    std::cout << \"hi\";\n}",
                Some(Some("cpp")),
            ),
            // CSS
            (
                ".button {\n  color: red;\n  padding: 4px 8px;\n}",
                Some(Some("css")),
            ),
            // Structured
            ("{\"name\": \"kopi\", \"version\": 1}", Some(Some("json"))),
            ("[1, 2, 3]", Some(Some("json"))),
            ("<div class=\"card\"><p>Hi</p></div>", Some(Some("html"))),
            ("<!DOCTYPE html>\n<html></html>", Some(Some("html"))),
            ("<?xml version=\"1.0\"?><plist></plist>", Some(Some("xml"))),
            ("<dict><key>a</key></dict>", Some(Some("xml"))),
            ("SELECT id, name FROM users WHERE id = 1", Some(Some("sql"))),
            ("select * from clips;", Some(Some("sql"))),
            ("#!/usr/bin/env python3\nprint('hi')", Some(Some("python"))),
            ("#!/bin/bash\necho hi", Some(Some("shell"))),
            // Shell
            ("git commit -m \"Fix the build\"", Some(Some("shell"))),
            ("$ cargo test -p kopi-core", Some(Some("shell"))),
            ("cargo build --release", Some(Some("shell"))),
            // Code without a clear language
            ("x = foo(bar);", Some(None)),
            ("def double(x):\n    return x * 2", Some(Some("python"))),
            ("if (a == b) {\n  run();\n}", Some(None)),
            // Prose and other text
            ("Hello world", None),
            ("Thanks, see you tomorrow!", None),
            (
                "Hi team,\nThe release is ready. Please test it (on macOS 14 and later) before Friday.\nThanks!",
                None,
            ),
            (
                "Use the form below to sign up.\nIf you have questions, reply to this email.",
                None,
            ),
            ("let me know what you think", None),
            ("for example: apples and pears", None),
            ("- apples\n- pears\n- plums", None),
            ("# Notes\n\nSome thoughts on the design.", None),
            ("1. Open the app\n2. Click Settings", None),
            ("Shopping list:\n    milk\n    eggs", None),
            ("Meeting at 10:30 in room 4", None),
            ("I love you <3", None),
            ("{}", None),
            ("[citation needed]", None),
            ("DELETE this later", None),
            ("Select the files from the folder", None),
            ("git is a version control system.", None),
            ("python is fun", None),
            ("", None),
            ("   \n  ", None),
        ];
        let wrong: Vec<_> = cases
            .iter()
            .map(|(text, want)| (text, want, detect(text).map(|c| c.lang)))
            .filter(|(_, want, got)| got != *want)
            .collect();
        assert!(wrong.is_empty(), "(text, want, got): {wrong:#?}");
    }

    #[test]
    fn long_text_is_sampled() {
        let line = "    let mut value = compute(input);\n";
        let text = format!("fn main() {{\n{}}}", line.repeat(2000));
        assert!(text.len() > SAMPLE_BYTES);
        assert_eq!(detect(&text), Some(Code { lang: Some("rust") }));
    }
}
