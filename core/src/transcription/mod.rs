//! Transcription abstraction for Phase 2.
//!
//! This trait will be the seam behind which Whisper.cpp, Parakeet, etc. live.
//! Phase 1 provided `DummyModel`; Phase 2 adds `WhisperCppModel` with on-disk
//! loading and streaming.

use crate::engine::events::EngineEvent;
use std::path::{Path, PathBuf};

pub mod parakeet;
pub mod whisper;

#[derive(Debug, Clone)]
pub struct TranscriptionOutput {
    pub text: String,
    pub is_final: bool,
    pub confidence: Option<f32>,
}

/// Metadata about a model.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ModelMetadata {
    pub id: String,
    pub display_name: String,
    pub local_path: Option<PathBuf>,
    pub runtime: String,
    pub size_mb: u64,
    pub quantization: String,
    pub languages: Vec<String>,
    pub license: String,
    pub checksum: String,
    pub is_installed: bool,
    pub is_loaded: bool,
}

/// Errors from model operations.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ModelError {
    #[error("model not loaded")]
    NotLoaded,
    #[error("model already loaded: {0}")]
    AlreadyLoaded(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("corrupted model: {0}")]
    Corrupted(String),
    #[error("transcription cancelled")]
    Cancelled,
    #[error("unsupported: {0}")]
    Unsupported(String),
}

pub trait SpeechModel: Send + Sync {
    fn model_id(&self) -> &str;
    fn metadata(&self) -> ModelMetadata;
    fn is_loaded(&self) -> bool;
    fn load(&mut self, path: &Path) -> Result<(), ModelError>;
    fn unload(&mut self);
    fn transcribe(&self, pcm: &[f32]) -> Result<TranscriptionOutput, ModelError>;
    fn transcribe_stream(&self, pcm: &[f32], chunk_ms: usize) -> Vec<TranscriptionOutput>;
    fn cancel(&self);
    fn set_language(&mut self, lang: &str) -> Result<(), ModelError>;
}

/// Dummy model that echoes a fixed transcript; proves plumbing without ML.
pub struct DummyModel {
    pub id: String,
    loaded: bool,
}

impl DummyModel {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            loaded: true,
        }
    }
}

impl SpeechModel for DummyModel {
    fn model_id(&self) -> &str {
        &self.id
    }

    fn metadata(&self) -> ModelMetadata {
        ModelMetadata {
            id: self.id.clone(),
            display_name: format!("Dummy {}", self.id),
            local_path: None,
            runtime: "dummy".into(),
            size_mb: 0,
            quantization: "none".into(),
            languages: vec!["en".into()],
            license: "MIT".into(),
            checksum: "dummy".into(),
            is_installed: true,
            is_loaded: self.loaded,
        }
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn load(&mut self, _path: &Path) -> Result<(), ModelError> {
        self.loaded = true;
        Ok(())
    }

    fn unload(&mut self) {
        self.loaded = false;
    }

    fn transcribe(&self, _pcm: &[f32]) -> Result<TranscriptionOutput, ModelError> {
        if !self.loaded {
            return Err(ModelError::NotLoaded);
        }
        Ok(TranscriptionOutput {
            text: "[dummy transcript]".into(),
            is_final: true,
            confidence: Some(1.0),
        })
    }

    fn transcribe_stream(&self, pcm: &[f32], chunk_ms: usize) -> Vec<TranscriptionOutput> {
        if pcm.is_empty() {
            return vec![];
        }
        let chunk_samples = 16000 * chunk_ms / 1000;
        let n_chunks = (pcm.len() + chunk_samples - 1) / chunk_samples;
        (0..n_chunks)
            .map(|i| TranscriptionOutput {
                text: format!("[dummy chunk {}]", i),
                is_final: i == n_chunks - 1,
                confidence: Some(1.0),
            })
            .collect()
    }

    fn cancel(&self) {}

    fn set_language(&mut self, _lang: &str) -> Result<(), ModelError> {
        Ok(())
    }
}

/// Adapter to convert transcription output into engine events.
pub fn output_to_events(output: TranscriptionOutput) -> Vec<EngineEvent> {
    if output.is_final {
        vec![EngineEvent::FinalTranscript(output.text)]
    } else {
        vec![EngineEvent::PartialTranscript(output.text)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_model_transcribes() {
        let m = DummyModel::new("dummy");
        let out = m.transcribe(&[0.0; 160]).unwrap();
        assert!(out.is_final);
        assert_eq!(output_to_events(out)[0].name(), "final_transcript");
    }

    #[test]
    fn dummy_load_unload() {
        let mut m = DummyModel::new("dummy");
        m.unload();
        assert!(!m.is_loaded());
        assert!(m.transcribe(&[0.0; 160]).is_err());
        m.load(Path::new("/tmp/fake")).unwrap();
        assert!(m.is_loaded());
    }

    #[test]
    fn dummy_stream() {
        let m = DummyModel::new("dummy");
        let pcm = vec![0.0; 32000]; // 2 sec -> 2 chunks of 1000ms
        let outs = m.transcribe_stream(&pcm, 1000);
        assert_eq!(outs.len(), 2);
        assert!(!outs[0].is_final);
        assert!(outs[1].is_final);
    }
}
