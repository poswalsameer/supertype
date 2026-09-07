/// Model manager abstraction stub for Phase 1.
/// Phase 4 will expand this into catalog/download/verify/load/unload/switch.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub size_mb: u64,
    pub is_downloaded: bool,
}

pub fn builtin_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "whisper-tiny".into(),
            display_name: "Whisper Tiny (39M)".into(),
            size_mb: 75,
            is_downloaded: false,
        },
        ModelInfo {
            id: "whisper-base".into(),
            display_name: "Whisper Base (74M)".into(),
            size_mb: 142,
            is_downloaded: false,
        },
        ModelInfo {
            id: "parakeet-tdt-0.6b".into(),
            display_name: "Parakeet TDT 0.6B".into(),
            size_mb: 600,
            is_downloaded: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_has_three() {
        assert_eq!(builtin_catalog().len(), 3);
    }
}
