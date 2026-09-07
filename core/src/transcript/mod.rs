//! Transcript lifecycle: Audio → raw ASR → formatted → insertion.
//! Separates raw, normalized, final to allow Phase 4 local LLM extension.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawTranscript {
    pub text: String,
    pub is_final: bool,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FormattedTranscript {
    pub raw: String,
    pub formatted: String,
    pub confidence: Option<f32>,
}

impl FormattedTranscript {
    pub fn from_raw(raw: RawTranscript, dict: &std::collections::HashMap<String, String>) -> Self {
        let formatted_raw = crate::formatting::format_transcript(&raw.text);
        let formatted = crate::formatting::apply_dictionary(&formatted_raw, dict);
        Self {
            raw: raw.text,
            formatted,
            confidence: raw.confidence,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptRecord {
    pub id: Option<i64>,
    pub raw: String,
    pub formatted: String,
    pub model_id: String,
    pub duration_ms: Option<i64>,
    pub bundle_id: Option<String>,
    pub app_name: Option<String>,
    pub created_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn lifecycle_separation() {
        let raw = RawTranscript {
            text: "hello comma world period".into(),
            is_final: true,
            confidence: Some(0.99),
        };
        let dict = HashMap::new();
        let fmt = FormattedTranscript::from_raw(raw.clone(), &dict);
        assert_eq!(fmt.raw, "hello comma world period");
        assert_eq!(fmt.formatted, "Hello, world.");
        assert_eq!(fmt.confidence, Some(0.99));
    }

    #[test]
    fn lifecycle_with_dict() {
        let raw = RawTranscript {
            text: "hello wisp er".into(),
            is_final: true,
            confidence: None,
        };
        let mut dict = HashMap::new();
        dict.insert("wisp er".into(), "Wispr".into());
        let fmt = FormattedTranscript::from_raw(raw, &dict);
        assert_eq!(fmt.formatted, "Hello Wispr");
    }
}
