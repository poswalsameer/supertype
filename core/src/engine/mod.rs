pub mod events;
pub mod state;

use events::EngineEvent;
use parking_lot::Mutex;
use state::AppState;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use crate::settings::Settings;
use crate::storage::Storage;

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
    // Simple in-memory event queue; Swift drains via poll_event.
    // Phase 2 will swap this for tokio::broadcast.
    events: VecDeque<EngineEvent>,
    last_error: Option<String>,
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
            })),
        }
    }

    /// Initialize with a database path. Idempotent guard prevents double-init.
    pub fn initialize(&self, db_path: PathBuf) -> EngineResult<()> {
        let mut g = self.inner.lock();
        if g.initialized {
            return Err(EngineError::AlreadyInitialized);
        }
        let storage = Storage::open(&db_path).map_err(|e| EngineError::Storage(e.to_string()))?;
        // Load persisted settings, fallback to defaults.
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
        g.settings = new_settings;
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
        // Also emit semantic events for key transitions
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

    // ── Commands ───────────────────────────────────────────────

    pub fn start_recording(&self) -> EngineResult<()> {
        // Fast-path: Idle -> Recording (or via Preparing if needed)
        let cur = self.get_state();
        if cur == AppState::Recording {
            return Err(EngineError::InvalidTransition {
                from: "recording".into(),
                to: "recording".into(),
            });
        }
        // Allow Idle -> Recording directly; also Idle->Preparing->Recording
        if cur == AppState::Idle {
            // For Phase 1 we skip Preparing latency and go straight to Recording
            self.transition(AppState::Recording)?;
        } else {
            self.transition(AppState::Recording)?;
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
        // Simulate immediate processing -> completed for Phase 1 (no real ASR yet).
        // Phase 2 will keep it in Processing until ASR callback.
        // For now, push completion right away to prove the pipeline.
        // We do it synchronously so tests can assert the flow; real impl will be async.
        self.transition(AppState::Completed)?;
        Ok(())
    }

    pub fn cancel_recording(&self) -> EngineResult<()> {
        let cur = self.get_state();
        match cur {
            AppState::Recording | AppState::Preparing | AppState::Processing => {
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
        // Transition to Error from whatever state we are in (if legal; otherwise force)
        let cur = self.get_state();
        if cur.can_transition(AppState::Error) {
            self.transition(AppState::Error)?;
        } else {
            // Force state to Error for robustness
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
        // stop transitions Recording->Processing->Completed
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
        // Drain init event
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
}
