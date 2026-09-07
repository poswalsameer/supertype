//! Audio subsystem placeholder for Phase 1.
//!
//! Privacy invariant: raw PCM must never be persisted to disk.
//! This module documents the rule and provides a compile-time/runtime guard
//! for future phases to enforce it.
//!
//! Future Phase 2 layout:
//! Microphone -> native callback -> in-memory ring buffer -> resample -> VAD -> ASR
//! No file I/O in the realtime path.

// Nothing to persist.

/// Marker used in tests to prove no audio tables exist.
pub const AUDIO_NEVER_PERSISTED: &str =
    "Raw microphone audio is never persisted to disk by design.";
