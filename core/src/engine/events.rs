use serde::{Deserialize, Serialize};

/// Events emitted by the engine. Phase 1 emits only lifecycle events;
/// Phase 2 will add real `partial_transcript` / `final_transcript` payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum EngineEvent {
    RecordingStarted,
    RecordingStopped,
    SpeechDetected,
    SpeechEnded,
    PartialTranscript(String),
    FinalTranscript(String),
    ProcessingStarted,
    ProcessingCompleted,
    Error(String),
    StateChanged { from: String, to: String },
}

impl EngineEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::RecordingStarted => "recording_started",
            Self::RecordingStopped => "recording_stopped",
            Self::SpeechDetected => "speech_detected",
            Self::SpeechEnded => "speech_ended",
            Self::PartialTranscript(_) => "partial_transcript",
            Self::FinalTranscript(_) => "final_transcript",
            Self::ProcessingStarted => "processing_started",
            Self::ProcessingCompleted => "processing_completed",
            Self::Error(_) => "error",
            Self::StateChanged { .. } => "state_changed",
        }
    }
}

/// C-ABI friendly event discriminant for polling API.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    RecordingStarted = 0,
    RecordingStopped = 1,
    SpeechDetected = 2,
    SpeechEnded = 3,
    PartialTranscript = 4,
    FinalTranscript = 5,
    ProcessingStarted = 6,
    ProcessingCompleted = 7,
    Error = 8,
    StateChanged = 9,
    None = 255,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_serialization() {
        let e = EngineEvent::PartialTranscript("hello".into());
        let s = serde_json::to_string(&e).unwrap();
        let d: EngineEvent = serde_json::from_str(&s).unwrap();
        assert_eq!(e, d);
    }

    #[test]
    fn event_names() {
        assert_eq!(EngineEvent::RecordingStarted.name(), "recording_started");
        assert_eq!(EngineEvent::Error("oops".into()).name(), "error");
    }
}
