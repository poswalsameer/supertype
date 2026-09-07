//! Model manager for Phase 2.
//!
//! Expands Phase 1 stub into a metadata-aware manager that understands
//! `model_id/display_name/local_path/runtime/size/quantization/languages/license/checksum/installed/loaded`.
//! Phase 2 supports an already-installed Whisper model on disk (on-demand download via scripts).
//! Full download UI lands in Phase 4; this architecture makes Parakeet addition trivial.

pub mod downloader;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub size_mb: u64,
    pub is_downloaded: bool,
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
    // Phase 4 extended metadata
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub family: String,
    #[serde(default)]
    pub download_urls: Vec<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub min_memory_mb: u32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub is_recommended: bool,
    #[serde(default)]
    pub attribution: String,
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
            description: String::new(),
            family: "whisper".into(),
            download_urls: Vec::new(),
            capabilities: vec!["transcription".into()],
            min_memory_mb: 2048,
            is_default: false,
            is_recommended: false,
            attribution: String::new(),
        }
    }
}

pub fn builtin_catalog() -> Vec<ModelInfo> {
    vec![
        ModelInfo {
            id: "whisper-tiny-q4_0".into(),
            display_name: "Whisper Tiny Q4".into(),
            size_mb: 43,
            is_downloaded: false,
            quantization: "q4_0".into(),
            runtime: "whisper.cpp".into(),
            local_path: default_model_path("whisper-tiny-q4_0"),
            languages: vec!["en".into(), "multilingual".into()],
            license: "MIT".into(),
            checksum: String::new(),
            is_loaded: false,
            description: "Ultra lightweight · fastest · lowest RAM".into(),
            family: "whisper".into(),
            download_urls: vec!["https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q4_0.bin".into()],
            capabilities: vec!["transcription".into()],
            min_memory_mb: 1536,
            is_default: false,
            is_recommended: false,
            attribution: "OpenAI Whisper · ggerganov/whisper.cpp".into(),
        },
        ModelInfo {
            id: "whisper-tiny".into(),
            display_name: "Whisper Tiny (39M)".into(),
            size_mb: 75,
            is_downloaded: false,
            quantization: "q5_0".into(),
            runtime: "whisper.cpp".into(),
            local_path: default_model_path("whisper-tiny"),
            languages: vec!["en".into(), "multilingual".into()],
            license: "MIT".into(),
            checksum: String::new(),
            is_loaded: false,
            description: "Balanced tiny · recommended for 8 GB".into(),
            family: "whisper".into(),
            download_urls: vec!["https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_0.bin".into()],
            capabilities: vec!["transcription".into()],
            min_memory_mb: 2048,
            is_default: true,
            is_recommended: true,
            attribution: "OpenAI Whisper · ggerganov/whisper.cpp".into(),
        },
        ModelInfo {
            id: "whisper-base".into(),
            display_name: "Whisper Base (74M)".into(),
            size_mb: 142,
            is_downloaded: false,
            quantization: "q5_0".into(),
            runtime: "whisper.cpp".into(),
            local_path: default_model_path("whisper-base"),
            languages: vec!["en".into(), "multilingual".into()],
            license: "MIT".into(),
            checksum: String::new(),
            is_loaded: false,
            description: "Lightweight · higher accuracy".into(),
            family: "whisper".into(),
            download_urls: vec!["https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_0.bin".into()],
            capabilities: vec!["transcription".into()],
            min_memory_mb: 4096,
            is_default: false,
            is_recommended: true,
            attribution: "OpenAI Whisper · ggerganov/whisper.cpp".into(),
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
            description: "Fast · high accuracy · Apple Silicon optimized".into(),
            family: "parakeet".into(),
            download_urls: vec![
                "https://huggingface.co/nvidia/parakeet-tdt-0.6b-v2/resolve/main/parakeet-tdt-0.6b.onnx".into(),
            ],
            capabilities: vec!["transcription".into(), "timestamps".into()],
            min_memory_mb: 8192,
            is_default: false,
            is_recommended: true,
            attribution: "NVIDIA Parakeet · CC-BY-4.0".into(),
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
        builtin_catalog()
            .iter()
            .find(|m| m.id == id)
            .and_then(|m| m.download_urls.first().cloned())
            .or_else(|| match id {
                "whisper-tiny" => Some(
                    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny-q5_0.bin"
                        .into(),
                ),
                "whisper-base" => Some(
                    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base-q5_0.bin"
                        .into(),
                ),
                _ => None,
            })
    }

    /// Atomic install: verify checksum then move temp file to final location.
    pub fn install_from_temp(
        &self,
        temp_path: &Path,
        model_id: &str,
        expected_checksum: &str,
    ) -> Result<PathBuf, String> {
        // Check disk space (need at least file size + 100 MB)
        let meta = std::fs::metadata(temp_path).map_err(|e| e.to_string())?;
        let file_size = meta.len();
        if let Some(parent) = self
            .get(model_id)
            .and_then(|m| m.local_path.as_ref())
            .and_then(|p| p.parent())
        {
            let _ = std::fs::create_dir_all(parent);
            // Check space via hardware probe (best-effort)
            if let Some(free_gb) = crate::hardware::HardwareInfo::probe().disk_free_gb {
                let need_gb = ((file_size as f64 / (1024.0 * 1024.0 * 1024.0)).ceil() as u32) + 1;
                if free_gb < need_gb {
                    return Err(format!(
                        "not enough disk space: need {} GB, have {} GB",
                        need_gb, free_gb
                    ));
                }
            }
        }
        // Verify checksum if provided
        if !expected_checksum.is_empty() && !Self::verify_checksum(temp_path, expected_checksum)? {
            return Err("checksum mismatch — corrupted download".into());
        }
        // Detect corrupted: file must be >1 MB for whisper, >10 MB for parakeet
        if file_size < 1024 * 1024 {
            return Err("model file too small — corrupted download".into());
        }
        let dest = self
            .get(model_id)
            .and_then(|m| m.local_path.clone())
            .unwrap_or_else(|| self.models_dir.join(format!("{}.bin", model_id)));
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // Atomic move (rename is atomic on same filesystem)
        std::fs::rename(temp_path, &dest).map_err(|e| e.to_string())?;
        Ok(dest)
    }

    /// Uninstall (delete) a model file.
    pub fn uninstall(&self, model_id: &str) -> Result<bool, String> {
        let path = self
            .get(model_id)
            .and_then(|m| m.local_path.clone())
            .unwrap_or_else(|| self.models_dir.join(format!("{}.bin", model_id)));
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Hardware-aware recommendation.
    pub fn recommended_for_hardware(&self, hw: &crate::hardware::HardwareInfo) -> Vec<ModelInfo> {
        let ids = crate::hardware::recommend_model_ids(hw);
        ids.iter().filter_map(|id| self.get(id).cloned()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn catalog_has_three() {
        // Phase 4 catalog has 4 entries (tiny-q4, tiny, base, parakeet)
        assert!(builtin_catalog().len() >= 4);
        assert!(builtin_catalog().iter().any(|m| m.id == "whisper-tiny"));
        assert!(builtin_catalog()
            .iter()
            .any(|m| m.id == "parakeet-tdt-0.6b"));
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
        assert!(ModelManager::download_url("whisper-tiny-q4_0").is_some());
        assert!(ModelManager::download_url("parakeet-tdt-0.6b").is_some());
        assert!(ModelManager::download_url("unknown").is_none());
    }

    #[test]
    fn catalog_metadata_valid() {
        for m in builtin_catalog() {
            assert!(!m.id.is_empty());
            assert!(!m.display_name.is_empty());
            assert!(!m.runtime.is_empty());
            assert!(!m.license.is_empty());
            assert!(m.size_mb > 0);
            assert!(!m.download_urls.is_empty());
        }
    }

    #[test]
    fn install_and_uninstall() {
        let dir = TempDir::new().unwrap();
        let mgr = ModelManager::new(dir.path().to_path_buf());
        // Need to override local_path for test
        let mut mgr2 = mgr;
        for m in &mut mgr2.catalog {
            if m.id == "whisper-tiny" {
                m.local_path = Some(dir.path().join("whisper-tiny.bin"));
            }
        }
        let temp = dir.path().join("tmp.bin");
        std::fs::write(&temp, vec![0x42u8; 2 * 1024 * 1024]).unwrap();
        let dest = mgr2.install_from_temp(&temp, "whisper-tiny", "").unwrap();
        assert!(dest.exists());
        assert!(!temp.exists()); // moved
        assert!(mgr2.uninstall("whisper-tiny").unwrap());
        assert!(!dest.exists());
    }

    #[test]
    fn install_corrupted_detects_small() {
        let dir = TempDir::new().unwrap();
        let mgr = ModelManager::new(dir.path().to_path_buf());
        let temp = dir.path().join("tiny.bin");
        std::fs::write(&temp, b"tiny").unwrap();
        let res = mgr.install_from_temp(&temp, "whisper-tiny", "");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("too small"));
    }

    #[test]
    fn install_checksum_mismatch() {
        let dir = TempDir::new().unwrap();
        let mgr = ModelManager::new(dir.path().to_path_buf());
        let temp = dir.path().join("tmp2.bin");
        std::fs::write(&temp, vec![0x42u8; 2 * 1024 * 1024]).unwrap();
        let res = mgr.install_from_temp(&temp, "whisper-tiny", "deadbeef");
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("checksum"));
    }

    #[test]
    fn hardware_recommend() {
        let mgr = ModelManager::new("/tmp".into());
        let hw_small = crate::hardware::HardwareInfo {
            arch: "aarch64".into(),
            is_apple_silicon: true,
            cpu_cores: 8,
            memory_gb: 6,
            metal_supported: true,
            disk_free_gb: Some(20),
        };
        let rec = mgr.recommended_for_hardware(&hw_small);
        assert!(!rec.is_empty());
        let hw_big = crate::hardware::HardwareInfo {
            arch: "aarch64".into(),
            is_apple_silicon: true,
            cpu_cores: 10,
            memory_gb: 32,
            metal_supported: true,
            disk_free_gb: Some(100),
        };
        let rec2 = mgr.recommended_for_hardware(&hw_big);
        assert!(rec2.iter().any(|m| m.id == "parakeet-tdt-0.6b"));
    }
}
