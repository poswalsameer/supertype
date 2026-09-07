/// Deterministic formatting stub for Phase 1.
/// Phase 3 will implement spoken-punctuation normalization.
pub fn format_transcript(raw: &str) -> String {
    raw.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trims() {
        assert_eq!(format_transcript("  hello  "), "hello");
    }
}
