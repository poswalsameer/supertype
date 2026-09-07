use serde::{Deserialize, Serialize};

/// Persistent application settings. All fields have sane defaults so a fresh
/// install is usable without user intervention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Optional CoreAudio device UID (e.g. "AppleHDAEngineInput:1").
    /// None means "system default input".
    pub selected_microphone_id: Option<String>,

    /// Human-readable global shortcut representation, e.g. "fn" or "ctrl+shift+space".
    /// Phase 1 stores it as a string; Phase 3 parses it into a hotkey binding.
    pub global_shortcut: String,

    /// Selected transcription model identifier, e.g. "whisper-tiny.en".
    pub selected_model_id: String,

    /// Whether transcription history is persisted to SQLite.
    pub history_enabled: bool,

    /// Whether the app should launch at login (via SMAppService).
    pub launch_at_login: bool,

    /// Whether the floating recording overlay is shown.
    pub overlay_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            selected_microphone_id: None,
            global_shortcut: "fn".to_string(),
            selected_model_id: "whisper-tiny".to_string(),
            history_enabled: true,
            launch_at_login: false,
            overlay_enabled: true,
        }
    }
}

impl Settings {
    /// Validate invariants before persisting.
    pub fn validate(&self) -> Result<(), String> {
        if self.global_shortcut.trim().is_empty() {
            return Err("global_shortcut must not be empty".into());
        }
        if self.selected_model_id.trim().is_empty() {
            return Err("selected_model_id must not be empty".into());
        }
        if let Some(id) = &self.selected_microphone_id {
            if id.trim().is_empty() {
                return Err("selected_microphone_id must not be empty string".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        Settings::default().validate().unwrap();
    }

    #[test]
    fn empty_shortcut_rejected() {
        let mut s = Settings::default();
        s.global_shortcut = "   ".into();
        assert!(s.validate().is_err());
    }

    #[test]
    fn serialization_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let d: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, d);
    }

    #[test]
    fn custom_settings_roundtrip() {
        let s = Settings {
            selected_microphone_id: Some("test-device-123".into()),
            global_shortcut: "ctrl+space".into(),
            selected_model_id: "parakeet-0.6b".into(),
            history_enabled: false,
            launch_at_login: true,
            overlay_enabled: false,
        };
        s.validate().unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let d: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, d);
    }
}
