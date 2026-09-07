//! Resampling / format normalization to 16 kHz mono f32.
//!
//! The microphone may deliver 44.1 or 48 kHz stereo f32. ASR requires 16 kHz mono.
//! This module is format-agnostic: caller passes input rate + channel count.
//! Future ASR engines can request different target rates without changing capture.

/// Target sample rate for Whisper and most local ASR.
pub const TARGET_SAMPLE_RATE: u32 = 16000;

/// Convert interleaved or planar input to mono 16 kHz.
/// `input` is mono or stereo interleaved if `channels==2`.
/// Uses linear interpolation (fast, no alloc beyond output Vec).
/// For production high quality, Phase 2 could swap to `rubato` FFT without changing API.
pub fn resample_to_mono_16k(input: &[f32], input_rate: u32, channels: usize, out: &mut Vec<f32>) {
    if input.is_empty() {
        out.clear();
        return;
    }
    // 1. Mixdown to mono if needed
    // For interleaved stereo, average pairs.
    let mono: Vec<f32> = if channels == 1 {
        input.to_vec()
    } else if channels == 2 {
        input.chunks_exact(2).map(|c| (c[0] + c[1]) * 0.5).collect()
    } else {
        // For >2 channels, average all
        let mut mono = Vec::with_capacity(input.len() / channels);
        for chunk in input.chunks(channels) {
            let sum: f32 = chunk.iter().sum();
            mono.push(sum / channels as f32);
        }
        mono
    };

    if input_rate == TARGET_SAMPLE_RATE {
        *out = mono;
        return;
    }

    // 2. Linear resampling
    // ratio = out_rate / in_rate
    let ratio = TARGET_SAMPLE_RATE as f64 / input_rate as f64;
    let out_len = ((mono.len() as f64) * ratio).ceil() as usize;
    out.clear();
    out.reserve(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 / ratio;
        let idx0 = src_pos.floor() as usize;
        let frac = (src_pos - idx0 as f64) as f32;
        let s0 = mono.get(idx0).copied().unwrap_or(0.0);
        let s1 = mono.get(idx0 + 1).copied().unwrap_or(s0);
        out.push(s0 * (1.0 - frac) + s1 * frac);
    }
}

/// Normalize to [-1,1] if needed and apply simple peak normalization.
/// Input is expected to be already -1..1 but we clamp.
pub fn normalize_in_place(samples: &mut [f32]) {
    // Clamp and apply light gain if very quiet
    let mut peak: f32 = 0.0;
    for &s in samples.iter() {
        peak = peak.max(s.abs());
    }
    if peak > 0.0 && peak < 0.1 {
        let gain = 0.1 / peak;
        let gain = gain.min(4.0);
        for s in samples.iter_mut() {
            *s = (*s * gain).clamp(-1.0, 1.0);
        }
    } else {
        for s in samples.iter_mut() {
            *s = s.clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_48k_to_16k_sine() {
        // Generate 100ms 440 Hz sine at 48 kHz mono
        let sr_in = 48000;
        let len = (sr_in as f32 * 0.1) as usize;
        let input: Vec<f32> = (0..len)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr_in as f32).sin())
            .collect();
        let mut out = Vec::new();
        resample_to_mono_16k(&input, sr_in, 1, &mut out);
        // 100ms @16k = 1600 samples (±1)
        assert!(
            (out.len() as i32 - 1600).abs() <= 1,
            "out len {} != 1600",
            out.len()
        );
        // Peak preserved roughly
        let peak: f32 = out.iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!(peak > 0.9, "peak {}", peak);
    }

    #[test]
    fn resample_stereo_to_mono() {
        let input = vec![1.0, -1.0, 0.5, 0.5]; // 2 frames stereo interleaved
        let mut out = Vec::new();
        resample_to_mono_16k(&input, 16000, 2, &mut out);
        assert_eq!(out.len(), 2);
        assert!((out[0] - 0.0).abs() < 1e-5); // (1 + -1)/2
        assert!((out[1] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn resample_same_rate_passthrough() {
        let input = vec![0.1, 0.2, 0.3];
        let mut out = Vec::new();
        resample_to_mono_16k(&input, 16000, 1, &mut out);
        assert_eq!(out, input);
    }

    #[test]
    fn normalize_clamp_and_boost() {
        let mut samples = vec![2.0, -2.0, 0.05];
        normalize_in_place(&mut samples);
        assert!(samples[0] <= 1.0 && samples[1] >= -1.0);
    }

    #[test]
    fn audio_format_conversion_integration() {
        // Simulate native 48k stereo → mono 16k pipeline
        let sr = 48000;
        let frames = 480; // 10ms @48k stereo interleaved = 960 samples
        let input: Vec<f32> = (0..frames * 2)
            .map(|i| (i as f32 * 0.001).sin() * 0.5)
            .collect();
        let mut out = Vec::new();
        resample_to_mono_16k(&input, sr, 2, &mut out);
        normalize_in_place(&mut out);
        // 10ms @16k = 160 samples
        assert!((out.len() as i32 - 160).abs() <= 2);
        for &s in &out {
            assert!(s >= -1.0 && s <= 1.0);
        }
    }
}
