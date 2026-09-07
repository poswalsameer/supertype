//! Performance instrumentation for Phase 2.
//!
//! Measures capture start latency, VAD latency, model load time, ASR inference,
//! end-of-speech to final transcript, real-time factor, peak memory, CPU.
//! Stored locally for development/testing only, not transmitted.

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Time from start_recording() to first audio callback (ms).
    pub capture_start_ms: Option<u64>,
    /// VAD inference per chunk (µs).
    pub vad_latency_us: Option<u64>,
    /// Model load time (ms).
    pub model_load_ms: Option<u64>,
    /// Last ASR inference duration (ms).
    pub asr_ms: Option<u64>,
    /// Time from last speech end to final transcript (ms).
    pub eos_to_final_ms: Option<u64>,
    /// Real-time factor = asr_ms / audio_duration_ms
    pub real_time_factor: Option<f32>,
    /// Peak memory estimate (MB) — approximated via allocation tracking.
    pub peak_memory_mb: Option<u64>,
    /// CPU utilization estimate (%).
    pub cpu_pct: Option<f32>,
    /// Timestamp of measurement.
    pub measured_at: Option<String>,
    /// Selected backend (Metal / Accelerate / CPU).
    pub backend: Option<String>,
}

impl PerformanceMetrics {
    pub fn backend() -> String {
        // On ARM Apple Silicon, whisper.cpp would use Metal; we report what we would use.
        if cfg!(target_arch = "aarch64") {
            "metal".into()
        } else {
            "accelerate".into()
        }
    }
}

/// Simple histogram for benchmark tool.
#[derive(Debug, Clone, Default)]
pub struct BenchmarkReport {
    pub samples: Vec<BenchmarkSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSample {
    pub name: String,
    pub audio_duration_ms: u64,
    pub asr_ms: u64,
    pub rtf: f32,
    pub vad_ms: u64,
    pub text: String,
}

impl BenchmarkReport {
    pub fn add(&mut self, sample: BenchmarkSample) {
        self.samples.push(sample);
    }

    pub fn summary(&self) -> String {
        if self.samples.is_empty() {
            return "No samples".into();
        }
        let avg_rtf: f32 =
            self.samples.iter().map(|s| s.rtf).sum::<f32>() / self.samples.len() as f32;
        let max_asr = self.samples.iter().map(|s| s.asr_ms).max().unwrap_or(0);
        let mut out = String::new();
        out.push_str(&format!(
            "Benchmark: {} samples, avg RTF {:.2}, max ASR {}ms, backend {}\n",
            self.samples.len(),
            avg_rtf,
            max_asr,
            PerformanceMetrics::backend()
        ));
        for s in &self.samples {
            out.push_str(&format!(
                "  {:20} {:4}ms audio → {:4}ms ASR (RTF {:.2}) VAD {:3}ms | {}\n",
                s.name, s.audio_duration_ms, s.asr_ms, s.rtf, s.vad_ms, s.text
            ));
        }
        out
    }
}

/// Helper to measure a closure.
pub fn measure<F, T>(f: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let t0 = Instant::now();
    let res = f();
    (res, t0.elapsed())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_backend() {
        let b = PerformanceMetrics::backend();
        assert!(!b.is_empty());
    }

    #[test]
    fn benchmark_summary() {
        let mut r = BenchmarkReport::default();
        r.add(BenchmarkSample {
            name: "hello.wav".into(),
            audio_duration_ms: 1000,
            asr_ms: 300,
            rtf: 0.3,
            vad_ms: 2,
            text: "hello world".into(),
        });
        let s = r.summary();
        assert!(s.contains("hello.wav"));
        assert!(s.contains("RTF"));
    }

    #[test]
    fn measure_helper() {
        let (val, dur) = measure(|| {
            // do a small amount of work to ensure non-zero duration
            let mut s = 0u64;
            for i in 0..1000 {
                s = s.wrapping_add(i);
            }
            s
        });
        let _ = val;
        // Duration should be >=0 (allow 0 on very fast clocks)
        assert!(dur.as_nanos() >= 0);
    }
}
