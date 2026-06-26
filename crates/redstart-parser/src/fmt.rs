//! The Redstart formatter: one canonical, no-config layout (the Gleam/gofmt
//! model).
//!
//! This is a brace-aware reindenter rather than an AST pretty-printer: it
//! re-derives indentation from bracket depth, trims trailing whitespace, and
//! collapses runs of blank lines — while preserving comments and the author's
//! declaration order. (A full AST pretty-printer can come later; this one is
//! safe by construction — it can never drop a comment or reorder code.)

/// Format Redstart source into the canonical layout.
#[must_use]
pub fn format(src: &str) -> String {
    let mut out = String::new();
    let mut depth: i32 = 0;
    let mut blank_run = 0;

    for raw in src.lines() {
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            // Collapse 2+ consecutive blank lines into one.
            blank_run += 1;
            if blank_run == 1 {
                out.push('\n');
            }
            continue;
        }
        blank_run = 0;

        let (opens, closes) = bracket_delta(trimmed);
        let leading_closers = leading_closers(trimmed);

        let effective = (depth - leading_closers).max(0);
        for _ in 0..effective {
            out.push_str("  ");
        }
        out.push_str(trimmed);
        out.push('\n');

        depth = (depth + opens - closes).max(0);
    }

    // Exactly one trailing newline.
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Number of leading closing brackets on a line (these dedent before printing).
fn leading_closers(line: &str) -> i32 {
    let mut n = 0;
    for c in line.chars() {
        match c {
            '}' | ')' | ']' => n += 1,
            ' ' | '\t' | ',' => {}
            _ => break,
        }
    }
    n
}

/// Net `(opens, closes)` bracket counts on a line, ignoring string literals and
/// line comments so braces inside them don't skew indentation.
fn bracket_delta(line: &str) -> (i32, i32) {
    let mut opens = 0;
    let mut closes = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '/' if chars.peek() == Some(&'/') => break, // line comment
            '{' | '(' | '[' => opens += 1,
            '}' | ')' | ']' => closes += 1,
            _ => {}
        }
    }
    (opens, closes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reindents_nested_blocks() {
        let messy = "entity Pool {\nid: Id<Bytes>\n      balance: BigInt\n}\n";
        let expected = "entity Pool {\n  id: Id<Bytes>\n  balance: BigInt\n}\n";
        assert_eq!(format(messy), expected);
    }

    #[test]
    fn closing_brace_dedents() {
        let src = "handler on T.E(event) {\nlet x = f(a)\n}\n";
        assert_eq!(format(src), "handler on T.E(event) {\n  let x = f(a)\n}\n");
    }

    #[test]
    fn preserves_comments() {
        let src = "entity A {\n// a comment\nid: Id<Bytes>\n}\n";
        assert_eq!(
            format(src),
            "entity A {\n  // a comment\n  id: Id<Bytes>\n}\n"
        );
    }

    #[test]
    fn braces_in_strings_are_ignored() {
        let src = "abi X from \"./a}b{.json\"\nentity A {\nid: Id<Bytes>\n}\n";
        let out = format(src);
        assert!(out.contains("entity A {\n  id: Id<Bytes>\n}"));
    }

    #[test]
    fn collapses_blank_lines_and_trailing() {
        let src = "entity A {\n\n\n  id: Id<Bytes>\n}\n\n\n";
        assert_eq!(format(src), "entity A {\n\n  id: Id<Bytes>\n}\n");
    }

    #[test]
    fn is_idempotent() {
        let src = "handler on T.E(event) {\n  let x = match r {\n    Ok(v) => {\n      f(v)\n    }\n  }\n}\n";
        let once = format(src);
        assert_eq!(format(&once), once);
    }
}
