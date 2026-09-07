//! Model manager for Phase 2.
//!
//! Expands Phase 1 stub into a metadata-aware manager that understands
//! `model_id/display_name/local_path/runtime/size/quantization/languages/license/checksum/installed/loaded`.
//! Phase 2 supports an already-installed Whisper model on disk (on-demand download via scripts).
//! Full download UI lands in Phase 4; this architecture makes Parakeet addition trivial.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub size_mb: u64,
    pub is_downloaded: bool,
    /// Extended metadata (Phase 2 additions, kept optional for Phase 1 compat).
    #[serde(default)]
    pub quantization: String,
    #[serde(default)]
    pub runtime: String,
    #[serde(default)]
    pub local_path: Option<PathBuf>,
    #[serde(default)]
    pub languages: Vec<String>,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub checksum: String,
    #[serde(default)]
    pub is_loaded: bool,
}

impl ModelInfo {
    pub fn new_whisper(id: &str, display: &str, size_mb: u64, quant: &str) -> Self {
        Self {
            id: id.into(),
            display_name: display.into(),
            size_mb,
            is_downloaded: false,
            quantization: quant.into(),
            runtime: "whisper.cpp".into(),
            local_path: None,
            languages: vec!["en".into(), "multilingual".into()],
            license: "MIT".into(),
            checksum: String::new(),
            is_loaded: false,
        }
    }
}

pub fn builtin_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "whisper-tiny".into(),
            display_name: "Whisper Tiny (39M)".into(),
            size_mb: 75,
            is_downloaded: false,
            quantization: "q5_0".into(),
            runtime: "whisper.cpp".into(),
            local_path: default_model_path("whisper-tiny"),
            languages: vec!["en".into()],
            license: "MIT".into(),
            checksum: String::new(),
            is_loaded: false,
        },
        ModelInfo {
            id: "whisper-base".into(),
            display_name: "Whisper Base (74M)".into(),
            size_mb: 142,
            is_downloaded: false,
            quantization: "q5_0".into(),
            runtime: "whisper.cpp".into(),
            local_path: default_model_path("whisper-base"),
            languages: vec!["en".into()],
            license: "MIT".into(),
            checksum: String::new(),
            is_loaded: false,
        },
        ModelInfo {
            id: "parakeet-tdt-0.6b".into(),
            display_name: "Parakeet TDT 0.6B".into(),
            size_mb: 600,
            is_downloaded: false,
            quantization: "fp16".into(),
            runtime: "parakeet".into(),
            local_path: default_model_path("parakeet-tdt-0.6b"),
            languages: vec!["en".into()],
            license: "CC-BY-4.0".into(),
            checksum: String::new(),
            is_loaded: false,
        },
    ]
}

fn default_model_path(id: &str) -> Option<PathBuf> {
    dirs_path().map(|p| p.join(format!("{}.bin", id)))
}

fn dirs_path() -> Option<PathBuf> {
    // Use Application Support for models
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join("Library/Application Support/Supertype/models"))
}

/// Model manager: discovers installed models on disk, tracks loaded state.
pub struct ModelManager {
    models_dir: PathBuf,
    catalog: Vec<ModelInfo>,
}

impl ModelManager {
    pub fn new(models_dir: PathBuf) -> Self {
        Self {
            models_dir,
            catalog: builtin_catalog(),
        }
    }

    pub fn with_default_dir() -> Self {
        let dir = dirs_path().unwrap_or_else(|| PathBuf::from("./models"));
        Self::new(dir)
    }

    /// Scan `models_dir` for files and mark `is_downloaded`.
    pub fn discover(&mut self) -> Vec<ModelInfo> {
        for info in &mut self.catalog {
            if let Some(path) = &info.local_path {
                info.is_downloaded = path.exists();
                if info.is_downloaded {
                    if let Ok(meta) = std::fs::metadata(path) {
                        info.size_mb = meta.len() / (1024 * 1024);
                    }
                }
            } else {
                // Fallback: check models_dir/id.bin
                let p = self.models_dir.join(format!("{}.bin", info.id));
                info.is_downloaded = p.exists();
            }
        }
        self.catalog.clone()
    }

    pub fn catalog(&self) -> &[ModelInfo] {
        &self.catalog
    }

    pub fn get(&self, id: &str) -> Option<&ModelInfo> {
        self.catalog.iter().find(|m| m.id == id)
    }

    /// Verify sha256 if expected checksum is known (non-empty).
    pub fn verify_checksum(path: &Path, expected: &str) -> Result<bool, String> {
        if expected.is_empty() {
            return Ok(true);
        }
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let hash = {
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&data);
            hex::encode(hasher.finalize())
        };
        Ok(hash == expected)
    }

    pub fn models_dir(&self) -> &Path {
        &self.models_dir
    }

    /// Suggest download URL for a given model id (on-demand).
    pub fn download_url(id: &str) -> Option<String> {
        match id {
            "whisper-tiny" => Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_0.bin"
                    .into(),
            ),
            "whisper-base" => Some(
                "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_0.bin"
                    .into(),
            ),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn catalog_has_three() {
        assert_eq!(builtin_catalog().len(), 3);
    }

    #[test]
    fn discover_marks_downloaded() {
        let dir = TempDir::new().unwrap();
        let mut mgr = ModelManager::new(dir.path().to_path_buf());
        // Before: nothing downloaded
        mgr.discover();
        assert!(!mgr.get("whisper-tiny").unwrap().is_downloaded);
        // Create fake file in expected location via catalog's local_path override
        // Instead test via direct file in models_dir
        let p = dir.path().join("whisper-tiny.bin");
        std::fs::write(&p, b"fake").unwrap();
        // Need to set catalog local_path to p for discovery
        for m in &mut mgr.catalog {
            if m.id == "whisper-tiny" {
                m.local_path = Some(p.clone());
            }
        }
        mgr.discover();
        assert!(mgr.get("whisper-tiny").unwrap().is_downloaded);
    }

    #[test]
    fn checksum_verify() {
        let dir = TempDir::new().unwrap();
        let p = dir.path().join("model.bin");
        std::fs::write(&p, b"hello").unwrap();
        // Empty expected → true
        assert!(ModelManager::verify_checksum(&p, "").unwrap());
        // Wrong hash → false
        assert!(!ModelManager::verify_checksum(&p, "deadbeef").unwrap());
        // Correct hash
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"hello");
        let hex = hex::encode(h.finalize());
        assert!(ModelManager::verify_checksum(&p, &hex).unwrap());
    }

    #[test]
    fn download_url_known() {
        assert!(ModelManager::download_url("whisper-tiny").is_some());
        assert!(ModelManager::download_url("unknown").is_none());
    }
}
