use serde::{Deserialize, Serialize};

/// Application state machine for the voice engine lifecycle.
///
/// Transitions are intentionally conservative — invalid transitions return
/// an error rather than silently coercing state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum AppState {
    /// No active operation. Microphone closed, idle resource usage.
    Idle = 0,
    /// Acquiring microphone / loading resources before recording.
    Preparing = 1,
    /// Actively capturing audio (hold-to-talk).
    Recording = 2,
    /// Post-recording: running VAD/ASR/formatting (Phase 2+ work).
    Processing = 3,
    /// Transient success state; auto-returns to Idle after UI acknowledgement.
    Completed = 4,
    /// Recoverable error; caller must acknowledge to return to Idle.
    Error = 5,
}

impl AppState {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Idle),
            1 => Some(Self::Preparing),
            2 => Some(Self::Recording),
            3 => Some(Self::Processing),
            4 => Some(Self::Completed),
            5 => Some(Self::Error),
            _ => None,
        }
    }

    /// Returns true if `self -> target` is a legal transition.
    pub fn can_transition(self, target: Self) -> bool {
        use AppState::*;
        matches!(
            (self, target),
            // Idle can start a new capture session
            (Idle, Preparing)
            | (Idle, Recording) // direct fast-path: skip Preparing if mic already warm
            // Preparing can become recording or fail/cancel
            | (Preparing, Recording)
            | (Preparing, Idle)
            | (Preparing, Error)
            // Recording can stop, cancel, or error
            | (Recording, Processing)
            | (Recording, Idle) // cancel
            | (Recording, Error)
            // Processing can complete or error
            | (Processing, Completed)
            | (Processing, Error)
            | (Processing, Idle) // cancel during processing
            // Completed auto-resets; explicit reset also allowed
            | (Completed, Idle)
            | (Completed, Preparing)
            // Error recovery
            | (Error, Idle)
            | (Error, Preparing)
            // Allow self-loops for idempotent UI refreshes (no-op)
            | (Idle, Idle)
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Preparing => "preparing",
            Self::Recording => "recording",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Error => "error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_u8() {
        for s in [
            AppState::Idle,
            AppState::Preparing,
            AppState::Recording,
            AppState::Processing,
            AppState::Completed,
            AppState::Error,
        ] {
            assert_eq!(AppState::from_u8(s.as_u8()), Some(s));
        }
        assert_eq!(AppState::from_u8(99), None);
    }

    #[test]
    fn valid_transitions() {
        assert!(AppState::Idle.can_transition(AppState::Recording));
        assert!(AppState::Idle.can_transition(AppState::Preparing));
        assert!(AppState::Recording.can_transition(AppState::Processing));
        assert!(AppState::Processing.can_transition(AppState::Completed));
        assert!(AppState::Completed.can_transition(AppState::Idle));
        assert!(AppState::Error.can_transition(AppState::Idle));
    }

    #[test]
    fn invalid_transitions() {
        // Cannot jump from Idle directly to Processing/Completed
        assert!(!AppState::Idle.can_transition(AppState::Processing));
        assert!(!AppState::Idle.can_transition(AppState::Completed));
        assert!(!AppState::Recording.can_transition(AppState::Completed));
        assert!(!AppState::Idle.can_transition(AppState::Error));
        // Cannot go backwards from Processing to Recording
        assert!(!AppState::Processing.can_transition(AppState::Recording));
    }
}
