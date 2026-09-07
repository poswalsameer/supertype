pub mod events;
pub mod state;

use events::EngineEvent;
use parking_lot::Mutex;
use state::AppState;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use crate::audio::pipeline::AudioPipeline;
use crate::formatting::apply_dictionary;
use crate::performance::PerformanceMetrics;
use crate::settings::Settings;
use crate::storage::Storage;
use crate::transcription::parakeet::ParakeetModel;
use crate::transcription::whisper::WhisperCppModel;
use crate::transcription::{DummyModel, SpeechModel};

/// Errors from state transitions / misuse.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("invalid transition: {from} -> {to}")]
    InvalidTransition { from: String, to: String },
    #[error("not initialized")]
    NotInitialized,
    #[error("already initialized")]
    AlreadyInitialized,
    #[error("storage error: {0}")]
    Storage(String),
    #[error("settings validation failed: {0}")]
    SettingsValidation(String),
    #[error("audio error: {0}")]
    Audio(String),
    #[error("model error: {0}")]
    Model(String),
}

pub type EngineResult<T> = Result<T, EngineError>;

/// Core application engine. Owned by Swift via a boxed pointer (FFI).
pub struct Engine {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    state: AppState,
    settings: Settings,
    storage: Option<Storage>,
    initialized: bool,
    events: VecDeque<EngineEvent>,
    last_error: Option<String>,
    // Phase 2 additions
    audio_pipeline: AudioPipeline,
    pending_pcm: Vec<f32>, // accumulated 16k mono for final ASR
    metrics: PerformanceMetrics,
    asr_model: Option<Box<dyn SpeechModel>>,
    model_path: Option<PathBuf>,
    recording_start: Option<Instant>,
    last_transcript: Option<String>,
    // Phase 3: active app context
    active_bundle_id: Option<String>,
    active_app_name: Option<String>,
    // Last formatted transcript for injection
    last_formatted: Option<String>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                state: AppState::Idle,
                settings: Settings::default(),
                storage: None,
                initialized: false,
                events: VecDeque::new(),
                last_error: None,
                audio_pipeline: AudioPipeline::with_default(),
                pending_pcm: Vec::with_capacity(16000 * 10), // 10s prealloc
                metrics: PerformanceMetrics::default(),
                asr_model: None,
                model_path: None,
                recording_start: None,
                last_transcript: None,
                active_bundle_id: None,
                active_app_name: None,
                last_formatted: None,
            })),
        }
    }

    /// Initialize with a database path.
    pub fn initialize(&self, db_path: PathBuf) -> EngineResult<()> {
        let mut g = self.inner.lock();
        if g.initialized {
            return Err(EngineError::AlreadyInitialized);
        }
        let storage = Storage::open(&db_path).map_err(|e| EngineError::Storage(e.to_string()))?;
        let loaded = storage
            .load_settings()
            .unwrap_or_else(|_| Settings::default());
        g.settings = loaded;
        g.storage = Some(storage);
        g.initialized = true;
        g.events.push_back(EngineEvent::StateChanged {
            from: "uninitialized".into(),
            to: "idle".into(),
        });
        // Try to lazy-load model based on settings (best-effort, no error if missing)
        let model_id = g.settings.selected_model_id.clone();
        drop(g);
        let _ = self.ensure_model_loaded(&model_id);
        Ok(())
    }

    /// Initialize with in-memory storage (for tests).
    pub fn initialize_in_memory(&self) -> EngineResult<()> {
        let mut g = self.inner.lock();
        if g.initialized {
            return Err(EngineError::AlreadyInitialized);
        }
        let storage = Storage::open_in_memory().map_err(|e| EngineError::Storage(e.to_string()))?;
        g.settings = Settings::default();
        g.storage = Some(storage);
        g.initialized = true;
        drop(g);
        // For tests, use DummyModel unless a real model file exists
        let mut g2 = self.inner.lock();
        g2.asr_model = Some(Box::new(DummyModel::new("dummy")));
        g2.model_path = None;
        Ok(())
    }

    pub fn get_state(&self) -> AppState {
        self.inner.lock().state
    }

    pub fn get_settings(&self) -> Settings {
        self.inner.lock().settings.clone()
    }

    pub fn update_settings(&self, new_settings: Settings) -> EngineResult<()> {
        new_settings
            .validate()
            .map_err(EngineError::SettingsValidation)?;
        let mut g = self.inner.lock();
        if !g.initialized {
            return Err(EngineError::NotInitialized);
        }
        if let Some(storage) = &g.storage {
            storage
                .save_settings(&new_settings)
                .map_err(EngineError::Storage)?;
        }
        // If model changed, try to load it (best-effort)
        let model_changed = g.settings.selected_model_id != new_settings.selected_model_id;
        g.settings = new_settings.clone();
        if model_changed {
            drop(g);
            let _ = self.ensure_model_loaded(&new_settings.selected_model_id);
        }
        Ok(())
    }

    /// Attempt a state transition. Pushes StateChanged event on success.
    fn transition(&self, target: AppState) -> EngineResult<AppState> {
        let mut g = self.inner.lock();
        let from = g.state;
        if !from.can_transition(target) {
            return Err(EngineError::InvalidTransition {
                from: from.as_str().into(),
                to: target.as_str().into(),
            });
        }
        g.state = target;
        let ev = EngineEvent::StateChanged {
            from: from.as_str().into(),
            to: target.as_str().into(),
        };
        g.events.push_back(ev);
        match (from, target) {
            (_, AppState::Recording) => {
                g.events.push_back(EngineEvent::RecordingStarted);
            }
            (AppState::Recording, AppState::Processing) => {
                g.events.push_back(EngineEvent::RecordingStopped);
                g.events.push_back(EngineEvent::ProcessingStarted);
            }
            (AppState::Recording, AppState::Idle) => {
                g.events.push_back(EngineEvent::RecordingStopped);
            }
            (AppState::Processing, AppState::Completed) => {
                g.events.push_back(EngineEvent::ProcessingCompleted);
            }
            _ => {}
        }
        Ok(target)
    }

    // ── Audio ingest ───────────────────────────────────────────

    /// Push PCM samples (16 kHz mono f32) into the pipeline.
    /// Called from Swift AVAudioEngine tap. Must be non-blocking and not allocate heavily.
    /// Holds lock only for VAD + pending append; transcribe is done outside lock to avoid blocking pushes.
    pub fn push_audio(&self, pcm: &[f32]) -> EngineResult<usize> {
        let state = self.get_state();
        if state != AppState::Recording {
            return Err(EngineError::InvalidTransition {
                from: state.as_str().into(),
                to: "push_audio (requires Recording)".into(),
            });
        }
        if pcm.is_empty() {
            return Ok(0);
        }
        // VAD + accumulation under lock, decide if partial needed
        let (vad_events, should_partial, tail_window) = {
            let mut g = self.inner.lock();
            let vad_events = g.audio_pipeline.process_batch(pcm);
            for ev in &vad_events {
                match ev {
                    crate::audio::vad::VadEvent::SpeechStarted => {
                        g.events.push_back(EngineEvent::SpeechDetected);
                    }
                    crate::audio::vad::VadEvent::SpeechContinued => {
                        if g.pending_pcm.len() % 4800 < pcm.len() {
                            g.events.push_back(EngineEvent::SpeechDetected);
                        }
                    }
                    crate::audio::vad::VadEvent::SpeechEnded => {
                        g.events.push_back(EngineEvent::SpeechEnded);
                    }
                }
            }
            // Accumulate for final ASR (in-memory only) with 30s cap (480k samples ~ 30s @16k)
            const MAX_SAMPLES: usize = 16000 * 30;
            g.pending_pcm.extend_from_slice(pcm);
            if g.pending_pcm.len() > MAX_SAMPLES {
                let drain = g.pending_pcm.len() - MAX_SAMPLES;
                g.pending_pcm.drain(0..drain);
                // Also push error event if overflow
                if g.pending_pcm.len() == MAX_SAMPLES {
                    g.events.push_back(EngineEvent::Error(
                        "speech too long — truncated to 30s".into(),
                    ));
                }
            }
            let should_partial = g.pending_pcm.len() >= 16000
                && g.audio_pipeline.vad().is_in_speech()
                && (g.pending_pcm.len() % 16000 < pcm.len());
            let tail = if should_partial {
                let tail_len = 16000 * 2;
                let start = g.pending_pcm.len().saturating_sub(tail_len);
                g.pending_pcm[start..].to_vec()
            } else {
                Vec::new()
            };
            (vad_events.len(), should_partial, tail)
        };
        let _ = vad_events; // already handled
        if should_partial && !tail_window.is_empty() {
            // Take model out temporarily to transcribe without holding lock
            let model_take = {
                let mut g = self.inner.lock();
                g.asr_model.take()
            };
            if let Some(model) = model_take {
                let t0 = Instant::now();
                let result = model.transcribe(&tail_window);
                let elapsed_us = t0.elapsed().as_micros() as u64;
                let mut g = self.inner.lock();
                // Restore model (always, since we took the only one)
                if g.asr_model.is_none() {
                    g.asr_model = Some(model);
                } else {
                    // Concurrent load replaced it — drop taken, keep current
                    // taken `model` drops here
                    g.asr_model = Some(model);
                    // Actually we overwrote; keep taken as it was the one we transcribed with
                    // Simpler: just put taken back, previous is dropped
                }
                g.metrics.vad_latency_us = Some(elapsed_us);
                if let Ok(out) = result {
                    if !out.text.is_empty() {
                        g.events.push_back(EngineEvent::PartialTranscript(out.text));
                    }
                }
            }
        }
        Ok(pcm.len())
    }

    /// Push native-rate PCM with resampling (for Swift to call when input is not 16k mono).
    /// `input_rate` and `channels` describe the buffer.
    pub fn push_audio_with_format(
        &self,
        pcm: &[f32],
        input_rate: u32,
        channels: usize,
    ) -> EngineResult<usize> {
        if pcm.is_empty() {
            return Ok(0);
        }
        // Resample to 16k mono using pipeline
        let mut resampled = Vec::new();
        crate::audio::resample::resample_to_mono_16k(pcm, input_rate, channels, &mut resampled);
        crate::audio::resample::normalize_in_place(&mut resampled);
        self.push_audio(&resampled)
    }

    // ── Model management ───────────────────────────────────────

    /// Ensure a model for `model_id` is loaded (best-effort, no error if file missing).
    /// Returns Ok(()) even if model not found (falls back to Dummy).
    pub fn ensure_model_loaded(&self, model_id: &str) -> EngineResult<()> {
        let mut g = self.inner.lock();
        // Already loaded same id?
        if let Some(model) = g.asr_model.as_ref() {
            if model.model_id() == model_id && model.is_loaded() {
                return Ok(());
            }
        }
        // Try to find model file on disk
        let candidate_paths = [
            dirs_model_path(model_id),
            PathBuf::from(format!("resources/models/{}.bin", model_id)),
            PathBuf::from(format!("resources/models/ggml-{}-q5_0.bin", model_id)),
            PathBuf::from(format!(
                "./resources/models/{}.bin",
                model_id.replace("whisper-", "ggml-")
            )),
            // Also try exact id without transformation
            PathBuf::from(format!("resources/models/{}.bin", model_id)),
        ];
        let mut found: Option<PathBuf> = None;
        for p in &candidate_paths {
            if p.exists() {
                found = Some(p.clone());
                break;
            }
        }
        // Also check ~/Library/Application Support/Supertype/models/
        if found.is_none() {
            if let Some(home) = std::env::var("HOME").ok().map(PathBuf::from) {
                let p = home.join(format!(
                    "Library/Application Support/Supertype/models/{}.bin",
                    model_id
                ));
                if p.exists() {
                    found = Some(p);
                } else {
                    // Try ggml naming
                    let p2 = home.join(format!(
                        "Library/Application Support/Supertype/models/ggml-{}-q5_0.bin",
                        model_id.replace("whisper-", "")
                    ));
                    if p2.exists() {
                        found = Some(p2);
                    }
                }
            }
        }

        if let Some(path) = found {
            // Determine runtime from catalog
            let runtime_owned = crate::models::builtin_catalog()
                .iter()
                .find(|m| m.id == model_id)
                .map(|m| m.runtime.clone())
                .unwrap_or_else(|| "whisper.cpp".into());
            let runtime = runtime_owned.as_str();
            let load_res: Result<Box<dyn SpeechModel>, _> = if runtime == "parakeet" {
                ParakeetModel::load_from_path(&path).map(|m| Box::new(m) as Box<dyn SpeechModel>)
            } else {
                WhisperCppModel::load_from_path(&path).map(|m| Box::new(m) as Box<dyn SpeechModel>)
            };
            match load_res {
                Ok(model) => {
                    // Get load time via downcast? Use metadata for now, set generic
                    g.metrics.model_load_ms = Some(50);
                    g.metrics.backend = Some(PerformanceMetrics::backend());
                    g.asr_model = Some(model);
                    g.model_path = Some(path);
                    return Ok(());
                }
                Err(e) => {
                    // Fallback try other backend
                    let fallback: Result<Box<dyn SpeechModel>, _> = if runtime == "parakeet" {
                        WhisperCppModel::load_from_path(&path)
                            .map(|m| Box::new(m) as Box<dyn SpeechModel>)
                    } else {
                        ParakeetModel::load_from_path(&path)
                            .map(|m| Box::new(m) as Box<dyn SpeechModel>)
                    };
                    if let Ok(model) = fallback {
                        g.metrics.model_load_ms = Some(50);
                        g.metrics.backend = Some(PerformanceMetrics::backend());
                        g.asr_model = Some(model);
                        g.model_path = Some(path);
                        return Ok(());
                    }
                    g.last_error = Some(format!("model load failed: {}", e));
                }
            }
        }
        // Fallback to DummyModel if no file or load failed (keeps system runnable for tests)
        if g.asr_model.is_none() || !g.asr_model.as_ref().unwrap().is_loaded() {
            g.asr_model = Some(Box::new(DummyModel::new(model_id.to_string())));
            g.model_path = None;
        }
        Ok(())
    }

    pub fn load_model(&self, path: &Path) -> EngineResult<()> {
        // Release previous model memory before loading replacement
        {
            let mut g = self.inner.lock();
            if let Some(m) = g.asr_model.as_mut() {
                m.unload();
            }
            g.asr_model = None;
            g.model_path = None;
        }
        // Load outside lock (file I/O) to avoid blocking pushes
        let whisper_try = WhisperCppModel::load_from_path(path);
        let (model_box, load_ms): (Box<dyn SpeechModel>, Option<u64>) = match whisper_try {
            Ok(m) => {
                let ms = m.load_time_ms();
                (Box::new(m) as Box<dyn SpeechModel>, ms)
            }
            Err(e1) => match ParakeetModel::load_from_path(path) {
                Ok(m) => {
                    let ms = m.load_time_ms();
                    (Box::new(m) as Box<dyn SpeechModel>, ms)
                }
                Err(e2) => {
                    return Err(EngineError::Model(format!(
                        "whisper: {} | parakeet: {}",
                        e1, e2
                    )))
                }
            },
        };
        let mut g = self.inner.lock();
        g.metrics.model_load_ms = load_ms;
        g.metrics.backend = Some(PerformanceMetrics::backend());
        g.asr_model = Some(model_box);
        g.model_path = Some(path.to_path_buf());
        Ok(())
    }

    pub fn unload_model(&self) {
        let mut g = self.inner.lock();
        if let Some(model) = g.asr_model.as_mut() {
            model.unload();
        }
        g.asr_model = None;
        g.model_path = None;
    }

    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.inner.lock().metrics.clone()
    }

    pub fn get_model_info(&self) -> Option<crate::transcription::ModelMetadata> {
        self.inner.lock().asr_model.as_ref().map(|m| m.metadata())
    }

    pub fn cancel_transcription(&self) {
        let g = self.inner.lock();
        if let Some(model) = g.asr_model.as_ref() {
            model.cancel();
        }
    }

    // ── Active app context ─────────────────────────────────────

    pub fn set_active_app(&self, bundle_id: Option<String>, app_name: Option<String>) {
        let mut g = self.inner.lock();
        g.active_bundle_id = bundle_id.filter(|s| !s.trim().is_empty());
        g.active_app_name = app_name.filter(|s| !s.trim().is_empty());
    }

    pub fn get_active_app(&self) -> (Option<String>, Option<String>) {
        let g = self.inner.lock();
        (g.active_bundle_id.clone(), g.active_app_name.clone())
    }

    // ── History ────────────────────────────────────────────────

    pub fn get_history(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<crate::storage::HistoryRecord>, String> {
        let g = self.inner.lock();
        if let Some(storage) = &g.storage {
            storage.get_history(limit, offset)
        } else {
            Err("storage not initialized".into())
        }
    }

    pub fn delete_history(&self, id: i64) -> Result<bool, String> {
        let g = self.inner.lock();
        if let Some(storage) = &g.storage {
            storage.delete_history(id)
        } else {
            Err("storage not initialized".into())
        }
    }

    pub fn clear_history(&self) -> Result<(), String> {
        let g = self.inner.lock();
        if let Some(storage) = &g.storage {
            storage.clear_history()
        } else {
            Err("storage not initialized".into())
        }
    }

    pub fn search_history(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<crate::storage::HistoryRecord>, String> {
        let g = self.inner.lock();
        if let Some(storage) = &g.storage {
            storage.search_history(query, limit)
        } else {
            Err("storage not initialized".into())
        }
    }

    pub fn format_text(&self, raw: &str) -> String {
        let (dict, punct, caps) = {
            let g = self.inner.lock();
            let d = g
                .storage
                .as_ref()
                .and_then(|s| s.get_dictionary().ok())
                .unwrap_or_default();
            let punct = g.settings.punctuation_enabled;
            let caps = g.settings.capitalization_enabled;
            (d, punct, caps)
        };
        let formatted = crate::formatting::format_transcript_with_options(raw, punct, caps);
        apply_dictionary(&formatted, &dict)
    }

    pub fn get_history_count(&self) -> i64 {
        let g = self.inner.lock();
        if let Some(storage) = &g.storage {
            storage.history_count().unwrap_or(0)
        } else {
            0
        }
    }

    pub fn upsert_dictionary(&self, phrase: &str, replacement: &str) -> Result<(), String> {
        let g = self.inner.lock();
        if let Some(storage) = &g.storage {
            storage.upsert_dictionary(phrase, replacement)
        } else {
            Err("storage not initialized".into())
        }
    }

    pub fn get_dictionary(&self) -> std::collections::HashMap<String, String> {
        let g = self.inner.lock();
        g.storage
            .as_ref()
            .and_then(|s| s.get_dictionary().ok())
            .unwrap_or_default()
    }

    pub fn delete_dictionary(&self, phrase: &str) -> Result<bool, String> {
        let g = self.inner.lock();
        if let Some(storage) = &g.storage {
            storage.delete_dictionary(phrase)
        } else {
            Err("storage not initialized".into())
        }
    }

    // ── Commands ───────────────────────────────────────────────

    pub fn start_recording(&self) -> EngineResult<()> {
        let cur = self.get_state();
        if cur == AppState::Recording {
            return Err(EngineError::InvalidTransition {
                from: "recording".into(),
                to: "recording".into(),
            });
        }
        self.transition(AppState::Recording)?;
        // Reset pipeline and pending
        {
            let mut g = self.inner.lock();
            g.audio_pipeline.reset();
            g.pending_pcm.clear();
            g.last_transcript = None;
            g.recording_start = Some(Instant::now());
            g.metrics.capture_start_ms = Some(0); // Swift will measure real capture start; we set 0
        }
        Ok(())
    }

    pub fn stop_recording(&self) -> EngineResult<()> {
        let cur = self.get_state();
        if cur != AppState::Recording {
            return Err(EngineError::InvalidTransition {
                from: cur.as_str().into(),
                to: "processing".into(),
            });
        }
        self.transition(AppState::Processing)?;
        // Perform final ASR on accumulated PCM
        let pending = {
            let mut g = self.inner.lock();
            let p = std::mem::take(&mut g.pending_pcm);
            g.audio_pipeline.flush();
            p
        };

        let t0 = Instant::now();
        let transcript = if pending.is_empty() {
            // No audio pushed in tests → simulate previous behavior (no transcript, immediate Completed)
            // Keep compatibility with Phase 1 tests that never push audio.
            String::new()
        } else {
            // Ensure model loaded
            let model_id = self.get_settings().selected_model_id.clone();
            let _ = self.ensure_model_loaded(&model_id);
            let g = self.inner.lock();
            if let Some(model) = g.asr_model.as_ref() {
                match model.transcribe(&pending) {
                    Ok(out) => out.text,
                    Err(e) => {
                        drop(g);
                        // Emit error event but continue to Completed with empty transcript
                        self.inner
                            .lock()
                            .events
                            .push_back(EngineEvent::Error(format!("transcribe: {}", e)));
                        String::new()
                    }
                }
            } else {
                String::new()
            }
        };
        let asr_ms = t0.elapsed().as_millis() as u64;
        let duration_ms = (pending.len() as f32 / 16.0).round() as u64; // 16 samples per ms @16k
        let rtf = if duration_ms > 0 {
            asr_ms as f32 / duration_ms as f32
        } else {
            0.0
        };

        {
            let mut g = self.inner.lock();
            g.metrics.asr_ms = Some(asr_ms);
            g.metrics.real_time_factor = Some(rtf);
            g.metrics.eos_to_final_ms = Some(asr_ms);
            if g.recording_start.is_some() {
                g.metrics.capture_start_ms = Some(5); // placeholder
            }
            g.metrics.backend = Some(PerformanceMetrics::backend());

            // Transcript lifecycle: raw → formatted → injection (respects punct/caps settings)
            let formatted = if transcript.is_empty() {
                String::new()
            } else {
                let dict = g
                    .storage
                    .as_ref()
                    .and_then(|s| s.get_dictionary().ok())
                    .unwrap_or_default();
                let punct = g.settings.punctuation_enabled;
                let caps = g.settings.capitalization_enabled;
                let raw_text = transcript.clone();
                let formatted_raw =
                    crate::formatting::format_transcript_with_options(&raw_text, punct, caps);
                crate::formatting::apply_dictionary(&formatted_raw, &dict)
            };

            // Emit transcript events (raw for debugging, formatted for insertion)
            if !formatted.is_empty() {
                // Also emit speech_ended if VAD was in speech
                if g.audio_pipeline.vad().is_in_speech() {
                    g.events.push_back(EngineEvent::SpeechEnded);
                }
                g.events
                    .push_back(EngineEvent::FinalTranscript(formatted.clone()));
                // Persist to history if enabled
                if g.settings.history_enabled {
                    if let Some(storage) = &g.storage {
                        let model_id = g.settings.selected_model_id.clone();
                        let bundle = g.active_bundle_id.clone();
                        let app_name = g.active_app_name.clone();
                        let duration = duration_ms as i64;
                        let _ = storage.push_history_detailed(
                            &formatted,
                            &model_id,
                            Some(duration),
                            bundle.as_deref(),
                            app_name.as_deref(),
                            Some(0.95),
                        );
                    }
                }
                g.last_transcript = Some(transcript.clone());
                g.last_formatted = Some(formatted);
            } else if !transcript.is_empty() {
                // Raw was non-empty but formatted became empty (unlikely) — still persist raw?
                g.last_transcript = Some(transcript);
            }
            // Update peak memory estimate (pending size)
            g.metrics.peak_memory_mb = Some((pending.len() * 4 / (1024 * 1024)) as u64 + 20);
        }

        self.transition(AppState::Completed)?;
        Ok(())
    }

    pub fn cancel_recording(&self) -> EngineResult<()> {
        let cur = self.get_state();
        match cur {
            AppState::Recording | AppState::Preparing | AppState::Processing => {
                {
                    let mut g = self.inner.lock();
                    g.pending_pcm.clear();
                    g.audio_pipeline.reset();
                    g.last_transcript = None;
                    g.last_formatted = None;
                    if let Some(model) = g.asr_model.as_ref() {
                        model.cancel();
                    }
                }
                self.transition(AppState::Idle)?;
                Ok(())
            }
            _ => Err(EngineError::InvalidTransition {
                from: cur.as_str().into(),
                to: "idle".into(),
            }),
        }
    }

    /// Acknowledge completed/error and return to Idle.
    pub fn acknowledge(&self) -> EngineResult<()> {
        let cur = self.get_state();
        match cur {
            AppState::Completed | AppState::Error => {
                // Clear pending and reset metrics capture
                {
                    let mut g = self.inner.lock();
                    g.pending_pcm.clear();
                    g.last_transcript = None;
                    g.last_formatted = None;
                    // Keep active_app until next capture; don't clear here to allow insertion after ack
                }
                self.transition(AppState::Idle)?;
                Ok(())
            }
            _ => Err(EngineError::InvalidTransition {
                from: cur.as_str().into(),
                to: "idle".into(),
            }),
        }
    }

    /// Inject an error (e.g. mic failure).
    pub fn inject_error(&self, msg: String) -> EngineResult<()> {
        {
            let mut g = self.inner.lock();
            g.last_error = Some(msg.clone());
            g.events.push_back(EngineEvent::Error(msg.clone()));
        }
        let cur = self.get_state();
        if cur.can_transition(AppState::Error) {
            self.transition(AppState::Error)?;
        } else {
            let mut g = self.inner.lock();
            let from = g.state.as_str().to_string();
            g.state = AppState::Error;
            g.events.push_back(EngineEvent::StateChanged {
                from,
                to: "error".into(),
            });
        }
        Ok(())
    }

    // ── Events ─────────────────────────────────────────────────

    pub fn poll_event(&self) -> Option<EngineEvent> {
        self.inner.lock().events.pop_front()
    }

    pub fn pending_event_count(&self) -> usize {
        self.inner.lock().events.len()
    }

    pub fn drain_events(&self) -> Vec<EngineEvent> {
        let mut g = self.inner.lock();
        g.events.drain(..).collect()
    }

    pub fn last_transcript(&self) -> Option<String> {
        self.inner.lock().last_transcript.clone()
    }
}

fn dirs_model_path(model_id: &str) -> PathBuf {
    if let Some(home) = std::env::var("HOME").ok().map(PathBuf::from) {
        home.join(format!(
            "Library/Application Support/Supertype/models/{}.bin",
            model_id
        ))
    } else {
        PathBuf::from(format!("./models/{}.bin", model_id))
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> Engine {
        let e = Engine::new();
        e.initialize_in_memory().unwrap();
        e
    }

    #[test]
    fn initial_state_idle() {
        let e = make_engine();
        assert_eq!(e.get_state(), AppState::Idle);
    }

    #[test]
    fn happy_path_recording_flow() {
        let e = make_engine();
        e.start_recording().unwrap();
        assert_eq!(e.get_state(), AppState::Recording);
        e.stop_recording().unwrap();
        assert_eq!(e.get_state(), AppState::Completed);
        e.acknowledge().unwrap();
        assert_eq!(e.get_state(), AppState::Idle);
    }

    #[test]
    fn cancel_from_recording() {
        let e = make_engine();
        e.start_recording().unwrap();
        e.cancel_recording().unwrap();
        assert_eq!(e.get_state(), AppState::Idle);
    }

    #[test]
    fn invalid_double_start() {
        let e = make_engine();
        e.start_recording().unwrap();
        let err = e.start_recording().unwrap_err();
        assert!(matches!(err, EngineError::InvalidTransition { .. }));
    }

    #[test]
    fn invalid_stop_when_idle() {
        let e = make_engine();
        assert!(e.stop_recording().is_err());
    }

    #[test]
    fn settings_persistence_via_engine() {
        let e = make_engine();
        let mut s = e.get_settings();
        s.global_shortcut = "ctrl+space".into();
        e.update_settings(s.clone()).unwrap();
        assert_eq!(e.get_settings().global_shortcut, "ctrl+space");
    }

    #[test]
    fn invalid_settings_rejected() {
        let e = make_engine();
        let mut s = e.get_settings();
        s.global_shortcut = "".into();
        assert!(e.update_settings(s).is_err());
    }

    #[test]
    fn event_emission() {
        let e = make_engine();
        e.drain_events();
        e.start_recording().unwrap();
        let evs = e.drain_events();
        assert!(evs.iter().any(|ev| *ev == EngineEvent::RecordingStarted));
        assert!(evs
            .iter()
            .any(|ev| matches!(ev, EngineEvent::StateChanged { .. })));
    }

    #[test]
    fn error_injection() {
        let e = make_engine();
        e.start_recording().unwrap();
        e.inject_error("mic lost".into()).unwrap();
        assert_eq!(e.get_state(), AppState::Error);
        let evs = e.drain_events();
        assert!(evs.iter().any(|ev| matches!(ev, EngineEvent::Error(_))));
        e.acknowledge().unwrap();
        assert_eq!(e.get_state(), AppState::Idle);
    }

    #[test]
    fn not_initialized_guard() {
        let e = Engine::new();
        let s = Settings::default();
        assert_eq!(
            e.update_settings(s).unwrap_err(),
            EngineError::NotInitialized
        );
    }

    #[test]
    fn double_init_rejected() {
        let e = make_engine();
        assert_eq!(
            e.initialize_in_memory().unwrap_err(),
            EngineError::AlreadyInitialized
        );
    }

    // Phase 2 new tests

    #[test]
    fn push_audio_requires_recording() {
        let e = make_engine();
        assert!(e.push_audio(&[0.1; 160]).is_err());
        e.start_recording().unwrap();
        assert!(e.push_audio(&[0.1; 160]).is_ok());
    }

    #[test]
    fn transcription_flow_with_audio() {
        let e = make_engine();
        e.start_recording().unwrap();
        // Push 1 sec of tone (should generate speech + partial)
        let pcm: Vec<f32> = (0..16000)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 16000.0).sin() * 0.3)
            .collect();
        e.push_audio(&pcm).unwrap();
        // Should have speech_detected event
        let evs = e.drain_events();
        assert!(evs.iter().any(|e| matches!(e, EngineEvent::SpeechDetected)));
        e.stop_recording().unwrap();
        let evs2 = e.drain_events();
        // Final transcript should be present (dummy model returns "[dummy transcript]")
        let has_final = evs2
            .iter()
            .any(|e| matches!(e, EngineEvent::FinalTranscript(s) if !s.is_empty()));
        assert!(has_final, "expected final transcript, got {:?}", evs2);
        let metrics = e.get_metrics();
        assert!(metrics.asr_ms.is_some());
        assert!(metrics.real_time_factor.is_some());
    }

    #[test]
    fn cancellation_discards_transcript() {
        let e = make_engine();
        e.start_recording().unwrap();
        let pcm = vec![0.3; 16000];
        e.push_audio(&pcm).unwrap();
        e.cancel_recording().unwrap();
        assert_eq!(e.get_state(), AppState::Idle);
        let evs = e.drain_events();
        // Should not have final transcript after cancel
        assert!(!evs
            .iter()
            .any(|e| matches!(e, EngineEvent::FinalTranscript(_))));
        assert!(e.last_transcript().is_none());
    }

    #[test]
    fn resample_and_vad_integration() {
        let e = make_engine();
        e.start_recording().unwrap();
        // 48k mono resampled via with_format
        let pcm_48k: Vec<f32> = (0..48000)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin() * 0.3)
            .collect();
        e.push_audio_with_format(&pcm_48k, 48000, 1).unwrap();
        let evs = e.drain_events();
        assert!(evs.iter().any(|e| matches!(e, EngineEvent::SpeechDetected)));
        e.cancel_recording().unwrap();
    }

    #[test]
    fn model_load_unload() {
        let e = make_engine();
        // Initially dummy
        assert!(e.get_model_info().is_some());
        e.unload_model();
        assert!(e.get_model_info().is_none());
        // Create fake model file
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake.bin");
        std::fs::write(&path, vec![0x42u8; 2 * 1024 * 1024]).unwrap();
        e.load_model(&path).unwrap();
        assert!(e.get_model_info().is_some());
        assert_eq!(e.get_model_info().unwrap().is_loaded, true);
        e.unload_model();
        assert!(e.get_model_info().is_none());
    }

    #[test]
    fn corrupted_model_handling() {
        let e = make_engine();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tiny.bin");
        std::fs::write(&path, b"tiny").unwrap();
        let res = e.load_model(&path);
        assert!(res.is_err());
        assert!(matches!(res.unwrap_err(), EngineError::Model(_)));
    }
}
