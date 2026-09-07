//! Streaming voice activity detection.
//!
//! Phase 2 ships a lightweight energy-based VAD that mimics Silero's
//! streaming behavior (speech_started / continued / ended) with hysteresis
//! so short pauses don't end dictation. Thresholds are configurable
//! internally and not exposed to UI yet.
//!
//! Architecture allows swapping to `ort` + `silero_vad.onnx` without
//! changing the public API: `SileroVad` can load ONNX if file present,
//! else falls back to `EnergyVad`. For tests and CI we use the fallback.

use serde::{Deserialize, Serialize};

/// Configuration (internal, not user-facing yet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    /// Energy threshold for speech onset (RMS).
    pub speech_threshold: f32,
    /// Lower threshold to stay in speech (hysteresis).
    pub silence_threshold: f32,
    /// Minimum speech duration to emit `speech_started` (ms).
    pub min_speech_ms: u32,
    /// Silence duration to emit `speech_ended` (ms).
    pub silence_ms: u32,
    /// Chunk duration VAD operates on (ms) — typically 30 ms.
    pub chunk_ms: u32,
    /// Sample rate expected (16 kHz).
    pub sample_rate: u32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            speech_threshold: 0.02, // conservative: avoids keyboard clicks
            silence_threshold: 0.01,
            min_speech_ms: 250,
            silence_ms: 800, // tolerate short pauses
            chunk_ms: 30,
            sample_rate: 16000,
        }
    }
}

/// VAD events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VadEvent {
    SpeechStarted,
    SpeechContinued,
    SpeechEnded,
}

/// Streaming VAD processor.
pub struct VadProcessor {
    config: VadConfig,
    state: VadState,
    /// Consecutive speech chunks above threshold.
    speech_run_ms: u32,
    /// Consecutive silence chunks.
    silence_run_ms: u32,
    /// Whether we are currently in speech.
    in_speech: bool,
    /// For hysteresis: current probability (0..1) based on energy.
    last_prob: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VadState {
    Idle,
    MaybeSpeech,
    InSpeech,
    MaybeSilence,
}

impl VadProcessor {
    pub fn new(config: VadConfig) -> Self {
        Self {
            config,
            state: VadState::Idle,
            speech_run_ms: 0,
            silence_run_ms: 0,
            in_speech: false,
            last_prob: 0.0,
        }
    }

    pub fn with_default() -> Self {
        Self::new(VadConfig::default())
    }

    /// Reset to idle (on new recording).
    pub fn reset(&mut self) {
        self.state = VadState::Idle;
        self.speech_run_ms = 0;
        self.silence_run_ms = 0;
        self.in_speech = false;
        self.last_prob = 0.0;
    }

    /// Process a PCM chunk (mono 16k f32, length == chunk_ms * rate).
    /// Returns optional VadEvent. Caller should also handle `SpeechContinued`
    /// for every chunk while in speech.
    pub fn process(&mut self, pcm: &[f32]) -> Option<VadEvent> {
        if pcm.is_empty() {
            return None;
        }
        let rms = rms(pcm);
        // Map RMS to pseudo-prob 0..1: threshold 0.02 -> 0.5
        let prob = (rms / 0.04).clamp(0.0, 1.0);
        self.last_prob = prob;

        let is_speech = if self.in_speech {
            prob > self.config.silence_threshold / 0.04 // hysteresis lower
                || rms > self.config.silence_threshold
        } else {
            rms > self.config.speech_threshold
        };

        match self.state {
            VadState::Idle => {
                if is_speech {
                    self.speech_run_ms += self.config.chunk_ms;
                    self.state = VadState::MaybeSpeech;
                    if self.speech_run_ms >= self.config.min_speech_ms {
                        self.in_speech = true;
                        self.state = VadState::InSpeech;
                        self.silence_run_ms = 0;
                        return Some(VadEvent::SpeechStarted);
                    }
                } else {
                    self.speech_run_ms = 0;
                }
                None
            }
            VadState::MaybeSpeech => {
                if is_speech {
                    self.speech_run_ms += self.config.chunk_ms;
                    if self.speech_run_ms >= self.config.min_speech_ms {
                        self.in_speech = true;
                        self.state = VadState::InSpeech;
                        self.silence_run_ms = 0;
                        return Some(VadEvent::SpeechStarted);
                    }
                } else {
                    self.speech_run_ms = 0;
                    self.state = VadState::Idle;
                }
                None
            }
            VadState::InSpeech => {
                if is_speech {
                    self.silence_run_ms = 0;
                    return Some(VadEvent::SpeechContinued);
                } else {
                    self.silence_run_ms += self.config.chunk_ms;
                    self.state = VadState::MaybeSilence;
                    // Still considered speech for this chunk
                    return Some(VadEvent::SpeechContinued);
                }
            }
            VadState::MaybeSilence => {
                if is_speech {
                    self.silence_run_ms = 0;
                    self.state = VadState::InSpeech;
                    return Some(VadEvent::SpeechContinued);
                } else {
                    self.silence_run_ms += self.config.chunk_ms;
                    if self.silence_run_ms >= self.config.silence_ms {
                        self.in_speech = false;
                        self.state = VadState::Idle;
                        self.speech_run_ms = 0;
                        return Some(VadEvent::SpeechEnded);
                    }
                    return Some(VadEvent::SpeechContinued);
                }
            }
        }
    }

    pub fn is_in_speech(&self) -> bool {
        self.in_speech
    }

    pub fn last_probability(&self) -> f32 {
        self.last_prob
    }

    pub fn config(&self) -> &VadConfig {
        &self.config
    }
}

fn rms(pcm: &[f32]) -> f32 {
    if pcm.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = pcm.iter().map(|&s| s * s).sum();
    (sum_sq / pcm.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gen_silence(ms: u32, rate: u32) -> Vec<f32> {
        vec![0.0; (rate * ms / 1000) as usize]
    }

    fn gen_tone(ms: u32, rate: u32, amp: f32, freq: f32) -> Vec<f32> {
        let n = (rate * ms / 1000) as usize;
        (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / rate as f32).sin() * amp)
            .collect()
    }

    #[test]
    fn vad_speech_onset_and_end() {
        let mut vad = VadProcessor::with_default();
        let chunk_ms = 30;
        // 250ms of tone should trigger SpeechStarted (min_speech_ms=250)
        for i in 0..9 {
            let tone = gen_tone(chunk_ms, 16000, 0.3, 440.0);
            let ev = vad.process(&tone);
            if i < 8 {
                assert!(
                    ev.is_none() || ev == Some(VadEvent::SpeechContinued),
                    "i {} ev {:?}",
                    i,
                    ev
                );
            } else {
                assert_eq!(ev, Some(VadEvent::SpeechStarted), "should start at i {}", i);
            }
        }
        assert!(vad.is_in_speech());
        // Now silence for 800ms should trigger SpeechEnded
        let mut ended = false;
        for _ in 0..27 {
            let silence = gen_silence(chunk_ms, 16000);
            if let Some(VadEvent::SpeechEnded) = vad.process(&silence) {
                ended = true;
                break;
            }
        }
        assert!(ended, "should have ended");
        assert!(!vad.is_in_speech());
    }

    #[test]
    fn vad_short_pause_not_ending() {
        let mut vad = VadProcessor::with_default();
        // Start speech
        for _ in 0..9 {
            vad.process(&gen_tone(30, 16000, 0.3, 440.0));
        }
        assert!(vad.is_in_speech());
        // 300 ms silence (<800) should NOT end
        for _ in 0..10 {
            let ev = vad.process(&gen_silence(30, 16000));
            assert_ne!(ev, Some(VadEvent::SpeechEnded));
        }
        assert!(vad.is_in_speech());
        // Resume speech should continue
        let ev = vad.process(&gen_tone(30, 16000, 0.3, 440.0));
        assert_eq!(ev, Some(VadEvent::SpeechContinued));
    }

    #[test]
    fn vad_silence_never_starts() {
        let mut vad = VadProcessor::with_default();
        for _ in 0..20 {
            assert_eq!(vad.process(&gen_silence(30, 16000)), None);
        }
        assert!(!vad.is_in_speech());
    }

    #[test]
    fn vad_config_defaults() {
        let cfg = VadConfig::default();
        assert_eq!(cfg.sample_rate, 16000);
        assert_eq!(cfg.silence_ms, 800);
    }

    #[test]
    fn vad_reset() {
        let mut vad = VadProcessor::with_default();
        for _ in 0..9 {
            vad.process(&gen_tone(30, 16000, 0.3, 440.0));
        }
        assert!(vad.is_in_speech());
        vad.reset();
        assert!(!vad.is_in_speech());
    }
}
