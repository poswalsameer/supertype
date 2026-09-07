//! Audio subsystem for Phase 2.
//!
//! Privacy invariant: raw PCM must never be persisted to disk.
//! This module documents the rule and provides the streaming pipeline:
//! Microphone -> AVAudioEngine tap (Swift) -> `AudioRingProducer::push_batch`
//! -> `AudioRingConsumer` -> `resample_to_mono_16k` -> `VadProcessor` -> ASR
//! No file I/O in the real-time path.

pub mod pipeline;
pub mod resample;
pub mod ring;
pub mod vad;

// Nothing to persist.

/// Marker used in tests to prove no audio tables exist.
pub const AUDIO_NEVER_PERSISTED: &str =
    "Raw microphone audio is never persisted to disk by design.";

pub use pipeline::AudioPipeline;
pub use resample::{normalize_in_place, resample_to_mono_16k, TARGET_SAMPLE_RATE};
pub use ring::{new_pair, AudioRingConsumer, AudioRingProducer};
pub use vad::{VadConfig, VadEvent, VadProcessor};
