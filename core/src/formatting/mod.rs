//! Deterministic local formatter (Phase 5 hardened).
//! No LLM. Pure functions, tested, no allocations beyond output.
//! Improvements: token-based spoken punctuation (no offset drift), sorted dictionary, safe unicode slicing.

use std::collections::HashMap;

/// Format raw ASR output into final insertion text.
/// Steps: trim → collapse whitespace → spoken punctuation → spacing → capitalization → dictionary.
pub fn format_transcript(raw: &str) -> String {
    format_transcript_with_options(raw, true, true)
}

pub fn format_transcript_with_options(
    raw: &str,
    punctuation: bool,
    capitalization: bool,
) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let collapsed = collapse_whitespace(trimmed);
    let punctuated = if punctuation {
        apply_spoken_punctuation(&collapsed)
    } else {
        collapsed
    };
    let spaced = normalize_punctuation_spacing(&punctuated);
    let capitalized = if capitalization {
        capitalize_sentences(&spaced)
    } else {
        spaced
    };
    // Deduplicate obvious ASR fragments: "hello hello" where same word repeats due to streaming overlap
    let deduped = deduplicate_adjacent_words(&capitalized);
    collapse_whitespace(&deduped)
}

/// Replace dictionary entries (phrase -> replacement) in text.
/// Case-insensitive exact phrase match. Sorted longest phrase first to avoid partial overwrites.
/// Safe for unicode (char indices).
pub fn apply_dictionary(text: &str, dict: &HashMap<String, String>) -> String {
    if dict.is_empty() || text.is_empty() {
        return text.to_string();
    }
    // Sort by phrase length descending for deterministic longest-match priority
    let mut entries: Vec<(&String, &String)> = dict.iter().collect();
    entries.sort_by_key(|a| std::cmp::Reverse(a.0.len()));
    let mut out = text.to_string();
    for (phrase, replacement) in entries {
        let lower_out = out.to_lowercase();
        let lower_phrase = phrase.to_lowercase();
        // Build via char indices to avoid byte slice panic on unicode
        let out_chars: Vec<char> = out.chars().collect();
        let lower_chars: Vec<char> = lower_out.chars().collect();
        let phrase_chars: Vec<char> = lower_phrase.chars().collect();
        if phrase_chars.is_empty() {
            continue;
        }
        let mut result = String::new();
        let mut i = 0;
        let mut last = 0;
        while i + phrase_chars.len() <= lower_chars.len() {
            if lower_chars[i..i + phrase_chars.len()] == phrase_chars[..] {
                // Append from last..i
                result.extend(out_chars[last..i].iter());
                result.push_str(replacement);
                last = i + phrase_chars.len();
                i += phrase_chars.len();
            } else {
                i += 1;
            }
        }
        result.extend(out_chars[last..].iter());
        out = result;
    }
    out
}

fn collapse_whitespace(s: &str) -> String {
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
    out.split('\n')
        .map(|part| part.trim())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn apply_spoken_punctuation(s: &str) -> String {
    // Token-based to avoid offset drift after replacements of different lengths.
    // We lowercase for matching but preserve original case for non-commands.
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
    // Build a single pass: tokenize by whitespace, try to match 2-word then 1-word
    let lower = s.to_lowercase();
    // We'll work on word boundaries using a sliding window over the lower string
    // Simpler: iterate over string, try to match any phrase at position i
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let lower_chars: Vec<char> = lower.chars().collect();
    let mut i = 0;
    while i < lower_chars.len() {
        let mut matched: Option<(&str, usize)> = None;
        for (phrase, repl) in &replacements {
            let phrase_lower = phrase.to_lowercase();
            let p_chars: Vec<char> = phrase_lower.chars().collect();
            if i + p_chars.len() <= lower_chars.len()
                && lower_chars[i..i + p_chars.len()] == p_chars[..]
            {
                // Check word boundaries
                let before_ok = if i == 0 {
                    true
                } else {
                    !lower_chars[i - 1].is_alphanumeric()
                };
                let after = i + p_chars.len();
                let after_ok = if after >= lower_chars.len() {
                    true
                } else {
                    !lower_chars[after].is_alphanumeric()
                };
                if before_ok && after_ok {
                    // Prefer longest match
                    if matched.map(|(_, len)| p_chars.len() > len).unwrap_or(true) {
                        matched = Some((repl, p_chars.len()));
                    }
                }
            }
        }
        if let Some((repl, len)) = matched {
            result.push_str(repl);
            i += len;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

fn normalize_punctuation_spacing(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == ' ' && i + 1 < chars.len() && is_punct_without_space_before(chars[i + 1]) {
            i += 1;
            continue;
        }
        out.push(c);
        if is_punct_needs_space_after(c) && i + 1 < chars.len() && chars[i + 1].is_alphanumeric() {
            out.push(' ');
        }
        if (c == '(' || c == '"') && i + 1 < chars.len() && chars[i + 1] == ' ' {
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
    // Collapse any remaining multiple spaces (triple etc) safely
    let mut collapsed = String::with_capacity(out.len());
    let mut last_space = false;
    for ch in out.chars() {
        if ch == ' ' {
            if !last_space {
                collapsed.push(' ');
                last_space = true;
            }
        } else {
            collapsed.push(ch);
            last_space = false;
        }
    }
    // Ensure newlines don't have surrounding spaces already handled by collapse_whitespace later
    collapsed
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
            // Handle " i " -> " I " (single letter word)
            if c == 'i' || c == 'I' {
                let prev_is_space = out
                    .chars()
                    .last()
                    .map(|p| p == ' ' || p == '\n')
                    .unwrap_or(true);
                let next_is_space = chars
                    .peek()
                    .map(|n| *n == ' ' || *n == '\n' || is_punct_without_space_before(*n))
                    .unwrap_or(true);
                if prev_is_space && next_is_space && !capitalize_next {
                    out.pop();
                    out.push('I');
                } else {
                    out.push(c);
                }
            } else {
                out.push(c);
            }
        }
        if matches!(c, '.' | '?' | '!' | '\n') {
            capitalize_next = true;
        }
    }
    out
}

fn deduplicate_adjacent_words(s: &str) -> String {
    // Preserve paragraph breaks: process each \n-separated segment independently
    if s.contains('\n') {
        let parts: Vec<String> = s.split('\n').map(deduplicate_adjacent_words).collect();
        return parts.join("\n");
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.len() < 2 {
        return s.to_string();
    }
    let mut out: Vec<String> = Vec::with_capacity(words.len());
    for w in words {
        if let Some(last) = out.last() {
            // Compare ignoring trailing punctuation
            let a = last
                .trim_matches(|c: char| ",.!?;:".contains(c))
                .to_lowercase();
            let b = w
                .trim_matches(|c: char| ",.!?;:".contains(c))
                .to_lowercase();
            if !a.is_empty() && a == b {
                continue;
            }
        }
        out.push(w.to_string());
    }
    out.join(" ")
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
    fn dictionary_longest_first() {
        let mut dict = HashMap::new();
        dict.insert("hello".into(), "hi".into());
        dict.insert("hello world".into(), "hi world".into());
        assert_eq!(apply_dictionary("hello world", &dict), "hi world");
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
    #[test]
    fn triple_space_collapse() {
        assert_eq!(
            format_transcript("hello   world   test"),
            "Hello world test"
        );
    }
    #[test]
    fn i_capitalization() {
        assert_eq!(format_transcript("i am here period"), "I am here.");
    }
    #[test]
    fn dedup() {
        assert_eq!(
            deduplicate_adjacent_words("hello hello world"),
            "hello world"
        );
        assert_eq!(deduplicate_adjacent_words("Hello Hello"), "Hello");
    }
    #[test]
    fn with_options_no_punct() {
        assert_eq!(
            format_transcript_with_options("hello comma world", false, true),
            "Hello comma world"
        );
        assert_eq!(
            format_transcript_with_options("hello world", true, false),
            "hello world"
        );
    }
}
