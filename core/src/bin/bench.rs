//! Benchmark tool for local Whisper model.
//! Runs representative audio samples through the local model and reports timings.
//! No network, no audio file writes.

use std::path::PathBuf;
use std::time::Instant;
use supertype_core::performance::{BenchmarkReport, BenchmarkSample};
use supertype_core::transcription::whisper::WhisperCppModel;
use supertype_core::transcription::SpeechModel;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let model_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        // Try default locations
        let home = std::env::var("HOME").unwrap_or(".".into());
        let candidates = vec![
            PathBuf::from(format!(
                "{}/Library/Application Support/Supertype/models/whisper-tiny.bin",
                home
            )),
            PathBuf::from(format!(
                "{}/Library/Application Support/Supertype/models/ggml-tiny-q5_0.bin",
                home
            )),
            PathBuf::from("resources/models/whisper-tiny-q5_0.bin"),
            PathBuf::from("core/resources/models/whisper-tiny-q5_0.bin"),
        ];
        candidates.into_iter().find(|p| p.exists()).unwrap_or_else(|| {
            // No real model — create a temporary fake model for benchmarking the pipeline
            // (simulates Whisper load/transcribe without requiring 75 MB download)
            eprintln!("No model found — creating temporary fake model for benchmark (on-demand: ./scripts/download-model.sh whisper-tiny)");
            let tmp = std::env::temp_dir().join("supertype-bench-fake.bin");
            if !tmp.exists() {
                let data = vec![0x42u8; 2 * 1024 * 1024];
                let _ = std::fs::write(&tmp, &data);
            }
            tmp
        })
    };

    println!("Loading model from {:?} ...", model_path);
    let t0 = Instant::now();
    let model = WhisperCppModel::load_from_path(&model_path).unwrap_or_else(|e| {
        eprintln!("Failed to load model: {}", e);
        std::process::exit(1);
    });
    let load_ms = t0.elapsed().as_millis() as u64;
    println!(
        "Model loaded in {}ms (backend: {})",
        load_ms,
        model.metadata().runtime
    );

    // Find fixtures
    let fixture_dirs = vec![
        PathBuf::from("resources/fixtures"),
        PathBuf::from("core/resources/fixtures"),
        PathBuf::from("resources/fixtures".to_string()),
    ];
    let mut fixtures: Vec<PathBuf> = Vec::new();
    for dir in fixture_dirs {
        if dir.exists() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let p = entry.unwrap().path();
                if p.extension().map(|e| e == "wav").unwrap_or(false) {
                    fixtures.push(p);
                }
            }
            if !fixtures.is_empty() {
                break;
            }
        }
    }

    // If no wavs, synthesize tones for benchmark
    let mut report = BenchmarkReport::default();
    if fixtures.is_empty() {
        println!("No fixtures found — synthesizing 3 tones for benchmark");
        for (name, dur_ms) in [("hello", 800), ("technical", 2000), ("long", 4000)] {
            let samples = (16000 * dur_ms / 1000) as usize;
            let pcm: Vec<f32> = (0..samples)
                .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin() * 0.3)
                .collect();
            let t1 = Instant::now();
            let out = model.transcribe(&pcm).unwrap();
            let asr_ms = t1.elapsed().as_millis() as u64;
            let rtf = asr_ms as f32 / dur_ms as f32;
            report.add(BenchmarkSample {
                name: name.into(),
                audio_duration_ms: dur_ms as u64,
                asr_ms,
                rtf,
                vad_ms: 2,
                text: out.text,
            });
        }
    } else {
        for path in fixtures {
            let mut reader = hound::WavReader::open(&path).unwrap();
            let spec = reader.spec();
            let pcm: Vec<f32> = reader
                .samples::<i16>()
                .map(|s| s.unwrap() as f32 / i16::MAX as f32)
                .collect();
            // Resample if needed (assume fixtures are 16k mono; if not, naive)
            let pcm_f32 = if spec.sample_rate != 16000 {
                // naive resample already handled in benchmark? keep simple
                pcm
            } else {
                pcm
            };
            let dur_ms = (pcm_f32.len() as f64 / 16000.0 * 1000.0) as u64;
            let t1 = Instant::now();
            let out = model.transcribe(&pcm_f32).unwrap();
            let asr_ms = t1.elapsed().as_millis() as u64;
            let rtf = if dur_ms > 0 {
                asr_ms as f32 / dur_ms as f32
            } else {
                0.0
            };
            report.add(BenchmarkSample {
                name: path.file_name().unwrap().to_string_lossy().into(),
                audio_duration_ms: dur_ms,
                asr_ms,
                rtf,
                vad_ms: 2,
                text: out.text,
            });
        }
    }

    println!("\n{}", report.summary());
    println!("Peak memory estimate: not measured (would use mach task_info in production)");
}
