//! Transcription abstraction for Phase 1.
//!
//! This trait will be the seam behind which Whisper.cpp, Parakeet, etc. live.
//! Phase 1 provides the trait + a dummy implementation for wiring tests.
//! No real inference happens yet.
use crate::engine::events::EngineEvent;

pub trait SpeechModel: Send + Sync {
    fn model_id(&self) -> &str;
    /// Transcribe a PCM chunk (16k mono f32). Phase 2 implements real inference.
    fn transcribe(&self, _pcm: &[f32]) -> Result<TranscriptionOutput, String>;
}

#[derive(Debug, Clone)]
pub struct TranscriptionOutput {
    pub text: String,
    pub is_final: bool,
    pub confidence: Option<f32>,
}

/// Dummy model that echoes a fixed transcript; proves plumbing without ML.
pub struct DummyModel {
    pub id: String,
}

impl SpeechModel for DummyModel {
    fn model_id(&self) -> &str {
        &self.id
    }

    fn transcribe(&self, _pcm: &[f32]) -> Result<TranscriptionOutput, String> {
        Ok(TranscriptionOutput {
            text: "[dummy transcript]".into(),
            is_final: true,
            confidence: Some(1.0),
        })
    }
}

/// Adapter to convert transcription output into engine events (future use).
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
        let m = DummyModel { id: "dummy".into() };
        let out = m.transcribe(&[0.0; 160]).unwrap();
        assert!(out.is_final);
        assert_eq!(output_to_events(out)[0].name(), "final_transcript");
    }
}
