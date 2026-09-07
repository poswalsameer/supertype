//! Deterministic local formatter (Phase 3).
//! No LLM. All transforms are pure functions and tested.

use std::collections::HashMap;

/// Format raw ASR output into final insertion text.
/// Steps: trim → collapse whitespace → spoken punctuation → spacing → capitalization → dictionary.
pub fn format_transcript(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    // 1. Collapse repeated whitespace but preserve single spaces first; newlines handled via spoken commands
    let collapsed = collapse_whitespace(trimmed);
    // 2. Spoken punctuation
    let punctuated = apply_spoken_punctuation(&collapsed);
    // 3. Spacing normalization around punctuation
    let spaced = normalize_punctuation_spacing(&punctuated);
    // 4. Capitalization
    let capitalized = capitalize_sentences(&spaced);
    // 5. Final collapse to clean up spacing introduced
    collapse_whitespace(&capitalized)
}

/// Replace dictionary entries (phrase -> replacement) in text.
/// Case-insensitive exact phrase match.
pub fn apply_dictionary(text: &str, dict: &HashMap<String, String>) -> String {
    if dict.is_empty() || text.is_empty() {
        return text.to_string();
    }
    let mut out = text.to_string();
    for (phrase, replacement) in dict {
        // Simple case-insensitive replace via lowercased search
        let lower_out = out.to_lowercase();
        let lower_phrase = phrase.to_lowercase();
        let mut result = String::new();
        let mut last = 0;
        let mut search_start = 0;
        while let Some(pos) = lower_out[search_start..].find(&lower_phrase) {
            let abs_pos = search_start + pos;
            result.push_str(&out[last..abs_pos]);
            result.push_str(replacement);
            last = abs_pos + phrase.len();
            search_start = abs_pos + phrase.len();
        }
        result.push_str(&out[last..]);
        out = result;
    }
    out
}

fn collapse_whitespace(s: &str) -> String {
    // Collapse whitespace but preserve up to one blank line (\n\n) for new paragraph.
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = false;
    let mut consecutive_newlines = 0;
    for ch in s.chars() {
        if ch == '\n' {
            consecutive_newlines += 1;
            if consecutive_newlines <= 2 {
                out.push('\n');
            }
            last_was_space = false;
        } else if ch.is_whitespace() {
            if !last_was_space && consecutive_newlines == 0 {
                out.push(' ');
                last_was_space = true;
            }
        } else {
            out.push(ch);
            last_was_space = false;
            consecutive_newlines = 0;
        }
    }
    // Trim spaces around newlines and overall
    out.split('\n')
        .map(|part| part.trim())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn apply_spoken_punctuation(s: &str) -> String {
    // Work on lowercased token matching but preserve original case for non-punct words.
    // We tokenise by whitespace, replacing spoken commands with symbols.
    // Longer phrases first to avoid partial.
    let replacements: Vec<(&str, &str)> = vec![
        ("new paragraph", "\n\n"),
        ("new line", "\n"),
        ("question mark", "?"),
        ("exclamation mark", "!"),
        ("exclamation point", "!"),
        ("open quote", "\""),
        ("close quote", "\""),
        ("open parenthesis", "("),
        ("close parenthesis", ")"),
        ("open bracket", "["),
        ("close bracket", "]"),
        ("comma", ","),
        ("period", "."),
        ("dot", "."),
        ("colon", ":"),
        ("semicolon", ";"),
        ("dash", " - "),
        ("hyphen", "-"),
        ("ellipsis", "..."),
    ];
    // Case-insensitive matching: build lower version for search
    let mut result = s.to_string();
    let mut lower = result.to_lowercase();
    for (phrase, repl) in replacements {
        // Replace " phrase " boundaries to avoid "comma operator" false positive?
        // We do conservative: replace only when phrase is isolated by word boundaries (space/punct/start/end).
        // Simple: replace all occurrences case-insensitively when surrounded by non-alpha or ends.
        let mut i = 0;
        while let Some(pos) = lower[i..].find(phrase) {
            let abs = i + pos;
            // Check word boundaries: char before and after is not alphanumeric
            let before_ok = if abs == 0 {
                true
            } else {
                let c = lower[..abs].chars().last().unwrap_or(' ');
                !c.is_alphanumeric()
            };
            let after_end = abs + phrase.len();
            let after_ok = if after_end >= lower.len() {
                true
            } else {
                let c = lower[after_end..].chars().next().unwrap_or(' ');
                !c.is_alphanumeric()
            };
            if before_ok && after_ok {
                result.replace_range(abs..after_end, repl);
                lower.replace_range(abs..after_end, repl);
                i = abs + repl.len();
            } else {
                i = abs + phrase.len();
            }
        }
    }
    result
}

fn normalize_punctuation_spacing(s: &str) -> String {
    // Remove space before , . ? ! : ; ) ] " and ensure single space after them when followed by alphanumeric
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' && i + 1 < chars.len() && is_punct_without_space_before(chars[i + 1]) {
            // skip space before punct
            i += 1;
            continue;
        }
        out.push(c);
        if is_punct_needs_space_after(c) && i + 1 < chars.len() && chars[i + 1].is_alphanumeric() {
            out.push(' ');
        }
        if (c == '(' || c == '"') && i + 1 < chars.len() && chars[i + 1] == ' ' {
            // skip space after opening '(' or opening quote
            // Only if next non-space is alphanumeric (likely opening)
            let mut j = i + 1;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            if j < chars.len() && chars[j].is_alphanumeric() {
                i = j - 1;
            }
        }
        i += 1;
    }
    out.replace("  ", " ")
}

fn is_punct_without_space_before(c: char) -> bool {
    matches!(c, ',' | '.' | '?' | '!' | ':' | ';' | ')' | ']' | '"')
}
fn is_punct_needs_space_after(c: char) -> bool {
    matches!(c, ',' | '.' | '?' | '!' | ':' | ';')
}

fn capitalize_sentences(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(s.len());
    let mut capitalize_next = true;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if capitalize_next && c.is_alphabetic() {
            out.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            out.push(c);
        }
        if matches!(c, '.' | '?' | '!' | '\n') {
            capitalize_next = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn trims() {
        assert_eq!(format_transcript("  hello  "), "Hello");
    }

    #[test]
    fn comma_period() {
        assert_eq!(
            format_transcript("hello comma world period"),
            "Hello, world."
        );
    }

    #[test]
    fn question_exclamation() {
        assert_eq!(
            format_transcript("is this a question question mark"),
            "Is this a question?"
        );
        assert_eq!(format_transcript("wow exclamation mark"), "Wow!");
    }

    #[test]
    fn new_line_paragraph() {
        assert_eq!(format_transcript("hello new line world"), "Hello\nWorld");
        assert_eq!(
            format_transcript("hello new paragraph world"),
            "Hello\n\nWorld"
        );
    }

    #[test]
    fn quotes_parens() {
        assert_eq!(
            format_transcript("open quote hello close quote"),
            "\"Hello\""
        );
        assert_eq!(
            format_transcript("open parenthesis hello close parenthesis"),
            "(Hello)"
        );
    }

    #[test]
    fn spacing_and_collapse() {
        assert_eq!(format_transcript("hello   comma   world  "), "Hello, world");
        assert_eq!(format_transcript("  hello   world  "), "Hello world");
    }

    #[test]
    fn capitalization() {
        assert_eq!(
            format_transcript("hello period world period"),
            "Hello. World."
        );
    }

    #[test]
    fn comma_operator_not_comma() {
        // "comma operator" should still become ", operator" is acceptable per spec,
        // but ensure "comma" inside word not replaced: "command" should stay
        assert_eq!(format_transcript("command"), "Command");
        assert_eq!(format_transcript("hello comma operator"), "Hello, operator");
    }

    #[test]
    fn dictionary_apply() {
        let mut dict = HashMap::new();
        dict.insert("wisp er".into(), "Wispr".into());
        assert_eq!(
            apply_dictionary("hello wisp er world", &dict),
            "hello Wispr world"
        );
        assert_eq!(apply_dictionary("WISP ER", &dict), "Wispr");
    }

    #[test]
    fn empty() {
        assert_eq!(format_transcript(""), "");
        assert_eq!(format_transcript("   "), "");
    }

    #[test]
    fn complex() {
        assert_eq!(
            format_transcript("hello comma world period new line this is a test question mark"),
            "Hello, world.\nThis is a test?"
        );
    }
}
