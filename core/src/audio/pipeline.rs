//! Audio pipeline orchestration.
//!
//! The real-time tap pushes native-rate PCM into `AudioRingProducer` (lock-free).
//! This pipeline, running on a background Tokio task, drains the ring,
//! resamples to 16 kHz mono, normalizes, chunks for VAD, and emits
//! `VadEvent`s that the engine translates to `EngineEvent::Speech*`.

use crate::audio::resample::{normalize_in_place, resample_to_mono_16k, TARGET_SAMPLE_RATE};
use crate::audio::vad::{VadEvent, VadProcessor};

/// Pipeline that owns a consumer of the ring and VAD state.
/// It does not own the producer — that lives behind the FFI handle.
pub struct AudioPipeline {
    vad: VadProcessor,
    // Input format of the tap (detected from AVAudioEngine)
    input_rate: u32,
    input_channels: usize,
    // Scratch buffers (pre-allocated, reused to avoid alloc)
    resampled_buf: Vec<f32>,
    // Accumulation for chunking (30 ms @16k = 480 samples)
    pending: Vec<f32>,
}

impl AudioPipeline {
    pub fn new(input_rate: u32, input_channels: usize) -> Self {
        Self {
            vad: VadProcessor::with_default(),
            input_rate,
            input_channels,
            resampled_buf: Vec::with_capacity(4096),
            pending: Vec::with_capacity(4096),
        }
    }

    pub fn with_default() -> Self {
        Self::new(16000, 1)
    }

    pub fn update_input_format(&mut self, rate: u32, channels: usize) {
        self.input_rate = rate;
        self.input_channels = channels;
    }

    pub fn reset(&mut self) {
        self.vad.reset();
        self.pending.clear();
    }

    pub fn vad(&self) -> &VadProcessor {
        &self.vad
    }

    pub fn vad_mut(&mut self) -> &mut VadProcessor {
        &mut self.vad
    }

    /// Process a batch of native PCM (mono or interleaved stereo) pushed from Swift.
    /// Returns VAD events emitted for this batch.
    pub fn process_batch(&mut self, native_pcm: &[f32]) -> Vec<VadEvent> {
        if native_pcm.is_empty() {
            return Vec::new();
        }
        // Resample to 16k mono
        resample_to_mono_16k(
            native_pcm,
            self.input_rate,
            self.input_channels,
            &mut self.resampled_buf,
        );
        normalize_in_place(&mut self.resampled_buf);
        self.pending.extend_from_slice(&self.resampled_buf);

        let mut events = Vec::new();
        let chunk_samples = (TARGET_SAMPLE_RATE * self.vad.config().chunk_ms / 1000) as usize;
        while self.pending.len() >= chunk_samples {
            let chunk: Vec<f32> = self.pending.drain(..chunk_samples).collect();
            if let Some(ev) = self.vad.process(&chunk) {
                events.push(ev);
            }
        }
        events
    }

    /// Drain any leftover partial chunk as silence-padded for final flush.
    pub fn flush(&mut self) -> Vec<VadEvent> {
        if self.pending.is_empty() {
            return Vec::new();
        }
        let chunk_samples = (TARGET_SAMPLE_RATE * self.vad.config().chunk_ms / 1000) as usize;
        // Pad with zeros
        let mut chunk = std::mem::take(&mut self.pending);
        chunk.resize(chunk_samples, 0.0);
        if let Some(ev) = self.vad.process(&chunk) {
            vec![ev]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_speech_detection() {
        let mut pipe = AudioPipeline::new(16000, 1);
        // 300 ms tone at 16k mono
        let tone: Vec<f32> = (0..4800)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin() * 0.3)
            .collect();
        let events = pipe.process_batch(&tone);
        assert!(events.contains(&VadEvent::SpeechStarted));
    }

    #[test]
    fn pipeline_resample_stereo_48k() {
        let mut pipe = AudioPipeline::new(48000, 2);
        // 100ms @48k stereo interleaved
        let frames = 4800;
        let input: Vec<f32> = (0..frames * 2)
            .map(|i| (i as f32 * 0.01).sin() * 0.3)
            .collect();
        let events = pipe.process_batch(&input);
        // Should not panic and should produce some VAD decision (maybe speech)
        let _ = events;
        // Ensure pending correctly chunked
        assert!(pipe.pending.len() < 480);
    }

    #[test]
    fn pipeline_silence_no_start() {
        let mut pipe = AudioPipeline::new(16000, 1);
        let silence = vec![0.0; 16000]; // 1 sec silence
        let events = pipe.process_batch(&silence);
        assert!(!events.contains(&VadEvent::SpeechStarted));
    }
}
