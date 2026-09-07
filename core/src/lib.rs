//! Supertype Rust core — C-ABI surface for Swift.
//!
//! The Swift layer owns `*mut Engine` and calls these functions.
//! All functions are thread-safe via internal `parking_lot::Mutex`.
//! No async runtime is required for Phase 1; Phase 2 adds a Tokio runtime.

#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::manual_div_ceil)]
#![allow(clippy::needless_return)]
#![allow(clippy::chunks_exact_to_as_chunks)]

pub mod audio;
pub mod engine;
pub mod formatting;
pub mod hardware;
pub mod input;
pub mod models;
pub mod performance;
pub mod settings;
pub mod storage;
pub mod text_processor;
pub mod transcript;
pub mod transcription;

use engine::{Engine, EngineError};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};

// ── Engine handle ──────────────────────────────────────────────────

/// Create a new engine. Caller must call `engine_free` when done.
#[no_mangle]
pub extern "C" fn engine_new() -> *mut Engine {
    let e = Box::new(Engine::new());
    Box::into_raw(e)
}

/// Free an engine created by `engine_new`. Safe to call with null.
#[no_mangle]
pub extern "C" fn engine_free(ptr: *mut Engine) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let _ = Box::from_raw(ptr);
    }
}

/// Initialize persistence. `db_path` must be a valid UTF-8 C string.
/// Returns 0 on success, non-zero on error.
#[no_mangle]
pub extern "C" fn engine_initialize(ptr: *mut Engine, db_path: *const c_char) -> c_int {
    if ptr.is_null() || db_path.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    let cstr = unsafe { CStr::from_ptr(db_path) };
    let path_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let path = PathBuf::from(path_str);
    match engine.initialize(path) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("[supertype-core] initialize failed: {e}");
            map_error_code(&e)
        }
    }
}

/// Initialize with in-memory DB (for tests / previews). Returns 0 on success.
#[no_mangle]
pub extern "C" fn engine_initialize_in_memory(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    match engine.initialize_in_memory() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("[supertype-core] initialize_in_memory failed: {e}");
            map_error_code(&e)
        }
    }
}

#[no_mangle]
pub extern "C" fn engine_get_state(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    engine.get_state().as_u8() as c_int
}

#[no_mangle]
pub extern "C" fn engine_start_recording(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    match engine.start_recording() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("[supertype-core] start_recording: {e}");
            map_error_code(&e)
        }
    }
}

#[no_mangle]
pub extern "C" fn engine_stop_recording(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    match engine.stop_recording() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("[supertype-core] stop_recording: {e}");
            map_error_code(&e)
        }
    }
}

#[no_mangle]
pub extern "C" fn engine_cancel_recording(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    match engine.cancel_recording() {
        Ok(()) => 0,
        Err(e) => map_error_code(&e),
    }
}

#[no_mangle]
pub extern "C" fn engine_acknowledge(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    match engine.acknowledge() {
        Ok(()) => 0,
        Err(e) => map_error_code(&e),
    }
}

/// Get settings as a JSON string. Caller must free with `engine_string_free`.
/// Returns null on error.
#[no_mangle]
pub extern "C" fn engine_get_settings(ptr: *mut Engine) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let engine = unsafe { &*ptr };
    let s = engine.get_settings();
    match serde_json::to_string(&s) {
        Ok(json) => match CString::new(json) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Update settings from a JSON C string. Returns 0 on success.
#[no_mangle]
pub extern "C" fn engine_update_settings(ptr: *mut Engine, json: *const c_char) -> c_int {
    if ptr.is_null() || json.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    let cstr = unsafe { CStr::from_ptr(json) };
    let s = match cstr.to_str() {
        Ok(v) => v,
        Err(_) => return -2,
    };
    let settings: Settings = match serde_json::from_str(s) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[supertype-core] bad settings json: {e}");
            return -3;
        }
    };
    match engine.update_settings(settings) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("[supertype-core] update_settings: {e}");
            map_error_code(&e)
        }
    }
}

/// Free a string returned by `engine_get_settings`.
#[no_mangle]
pub extern "C" fn engine_string_free(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

/// Poll one pending event as JSON. Returns:
///  1  -> an event was written to `out_json` (caller must free with engine_string_free)
///  0  -> no pending events (out_json set to null)
/// <0  -> error
#[no_mangle]
pub extern "C" fn engine_poll_event(ptr: *mut Engine, out_json: *mut *mut c_char) -> c_int {
    if ptr.is_null() || out_json.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    unsafe {
        *out_json = std::ptr::null_mut();
    }
    match engine.poll_event() {
        Some(ev) => match serde_json::to_string(&ev) {
            Ok(json) => match CString::new(json) {
                Ok(cs) => {
                    unsafe {
                        *out_json = cs.into_raw();
                    }
                    1
                }
                Err(_) => -2,
            },
            Err(_) => -3,
        },
        None => 0,
    }
}

/// Semantic version of the core.
#[no_mangle]
pub extern "C" fn engine_version() -> *const c_char {
    use std::sync::OnceLock;
    static VERSION_CSTR: OnceLock<CString> = OnceLock::new();
    let cstr = VERSION_CSTR.get_or_init(|| CString::new(env!("CARGO_PKG_VERSION")).unwrap());
    cstr.as_ptr()
}

/// Push 16k mono f32 PCM into the engine (must be Recording state).
/// `data` points to `len` f32 samples. Returns 0 on success.
#[no_mangle]
pub extern "C" fn engine_push_audio(ptr: *mut Engine, data: *const f32, len: usize) -> c_int {
    if ptr.is_null() || (data.is_null() && len > 0) {
        return -1;
    }
    let engine = unsafe { &*ptr };
    let slice = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    match engine.push_audio(slice) {
        Ok(_) => 0,
        Err(e) => map_error_code(&e),
    }
}

/// Push native-rate PCM with resampling (e.g. 48k stereo).
#[no_mangle]
pub extern "C" fn engine_push_audio_with_format(
    ptr: *mut Engine,
    data: *const f32,
    len: usize,
    sample_rate: u32,
    channels: u32,
) -> c_int {
    if ptr.is_null() || (data.is_null() && len > 0) {
        return -1;
    }
    let engine = unsafe { &*ptr };
    let slice = if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    };
    match engine.push_audio_with_format(slice, sample_rate, channels as usize) {
        Ok(_) => 0,
        Err(e) => map_error_code(&e),
    }
}

/// Get performance metrics as JSON (caller frees with engine_string_free).
#[no_mangle]
pub extern "C" fn engine_get_metrics(ptr: *mut Engine) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let engine = unsafe { &*ptr };
    let m = engine.get_metrics();
    match serde_json::to_string(&m) {
        Ok(json) => match CString::new(json) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get last transcript (caller frees). Returns null if none.
#[no_mangle]
pub extern "C" fn engine_get_last_transcript(ptr: *mut Engine) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let engine = unsafe { &*ptr };
    match engine.last_transcript() {
        Some(t) => match CString::new(t) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Load a Whisper model from absolute path (sets as active model).
#[no_mangle]
pub extern "C" fn engine_load_model(ptr: *mut Engine, path: *const c_char) -> c_int {
    if ptr.is_null() || path.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    let cstr = unsafe { CStr::from_ptr(path) };
    let p = match cstr.to_str() {
        Ok(s) => PathBuf::from(s),
        Err(_) => return -2,
    };
    match engine.load_model(&p) {
        Ok(()) => 0,
        Err(e) => map_error_code(&e),
    }
}

#[no_mangle]
pub extern "C" fn engine_unload_model(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    engine.unload_model();
    0
}

#[no_mangle]
pub extern "C" fn engine_cancel_transcription(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    engine.cancel_transcription();
    0
}

#[no_mangle]
pub extern "C" fn engine_get_model_info(ptr: *mut Engine) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let engine = unsafe { &*ptr };
    match engine.get_model_info() {
        Some(m) => match serde_json::to_string(&m) {
            Ok(json) => match CString::new(json) {
                Ok(cs) => cs.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Set active app context (bundle id + app name) for next history entry.
#[no_mangle]
pub extern "C" fn engine_set_active_app(
    ptr: *mut Engine,
    bundle_id: *const c_char,
    app_name: *const c_char,
) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    let bid = if bundle_id.is_null() {
        None
    } else {
        let cstr = unsafe { CStr::from_ptr(bundle_id) };
        cstr.to_str().ok().map(|s| s.to_string())
    };
    let aname = if app_name.is_null() {
        None
    } else {
        let cstr = unsafe { CStr::from_ptr(app_name) };
        cstr.to_str().ok().map(|s| s.to_string())
    };
    engine.set_active_app(bid, aname);
    0
}

/// Format raw transcript deterministically (no side effects).
#[no_mangle]
pub extern "C" fn engine_format_text(ptr: *mut Engine, raw: *const c_char) -> *mut c_char {
    if ptr.is_null() || raw.is_null() {
        return std::ptr::null_mut();
    }
    let engine = unsafe { &*ptr };
    let cstr = unsafe { CStr::from_ptr(raw) };
    let s = match cstr.to_str() {
        Ok(v) => v,
        Err(_) => return std::ptr::null_mut(),
    };
    let formatted = engine.format_text(s);
    match CString::new(formatted) {
        Ok(cs) => cs.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get history as JSON array (limit/offset).
#[no_mangle]
pub extern "C" fn engine_get_history(ptr: *mut Engine, limit: i64, offset: i64) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let engine = unsafe { &*ptr };
    match engine.get_history(limit, offset) {
        Ok(v) => match serde_json::to_string(&v) {
            Ok(json) => match CString::new(json) {
                Ok(cs) => cs.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn engine_delete_history(ptr: *mut Engine, id: i64) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    match engine.delete_history(id) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(_) => -2,
    }
}

#[no_mangle]
pub extern "C" fn engine_clear_history(ptr: *mut Engine) -> c_int {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    match engine.clear_history() {
        Ok(()) => 0,
        Err(_) => -2,
    }
}

#[no_mangle]
pub extern "C" fn engine_search_history(
    ptr: *mut Engine,
    query: *const c_char,
    limit: i64,
) -> *mut c_char {
    if ptr.is_null() || query.is_null() {
        return std::ptr::null_mut();
    }
    let engine = unsafe { &*ptr };
    let cstr = unsafe { CStr::from_ptr(query) };
    let q = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    match engine.search_history(q, limit) {
        Ok(v) => match serde_json::to_string(&v) {
            Ok(json) => match CString::new(json) {
                Ok(cs) => cs.into_raw(),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn engine_get_history_count(ptr: *mut Engine) -> i64 {
    if ptr.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    engine.get_history_count()
}

#[no_mangle]
pub extern "C" fn engine_get_catalog(_ptr: *mut Engine) -> *mut c_char {
    // ptr may be null — catalog does not require engine, but we accept null for Swift convenience
    let catalog = crate::models::builtin_catalog();
    match serde_json::to_string(&catalog) {
        Ok(json) => match CString::new(json) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn engine_get_hardware_info(_ptr: *mut Engine) -> *mut c_char {
    let hw = crate::hardware::HardwareInfo::probe();
    match serde_json::to_string(&hw) {
        Ok(json) => match CString::new(json) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn engine_get_recommended_models(ptr: *mut Engine) -> *mut c_char {
    let hw = crate::hardware::HardwareInfo::probe();
    let mgr = crate::models::ModelManager::with_default_dir();
    let rec = mgr.recommended_for_hardware(&hw);
    // Avoid unused ptr warning
    let _ = ptr;
    match serde_json::to_string(&rec) {
        Ok(json) => match CString::new(json) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn engine_upsert_dictionary(
    ptr: *mut Engine,
    phrase: *const c_char,
    replacement: *const c_char,
) -> c_int {
    if ptr.is_null() || phrase.is_null() || replacement.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    let cstr_phrase = unsafe { CStr::from_ptr(phrase) };
    let cstr_repl = unsafe { CStr::from_ptr(replacement) };
    let phrase_str = match cstr_phrase.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let repl_str = match cstr_repl.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    match engine.upsert_dictionary(phrase_str, repl_str) {
        Ok(()) => 0,
        Err(_) => -3,
    }
}

#[no_mangle]
pub extern "C" fn engine_get_dictionary(ptr: *mut Engine) -> *mut c_char {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let engine = unsafe { &*ptr };
    let dict = engine.get_dictionary();
    match serde_json::to_string(&dict) {
        Ok(json) => match CString::new(json) {
            Ok(cs) => cs.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "C" fn engine_delete_dictionary(ptr: *mut Engine, phrase: *const c_char) -> c_int {
    if ptr.is_null() || phrase.is_null() {
        return -1;
    }
    let engine = unsafe { &*ptr };
    let cstr = unsafe { CStr::from_ptr(phrase) };
    let phrase_str = match cstr.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    match engine.delete_dictionary(phrase_str) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

#[no_mangle]
pub extern "C" fn engine_verify_model(path: *const c_char, expected_sha: *const c_char) -> c_int {
    if path.is_null() {
        return -1;
    }
    let cstr_path = unsafe { CStr::from_ptr(path) };
    let path_str = match cstr_path.to_str() {
        Ok(s) => s,
        Err(_) => return -2,
    };
    let expected = if expected_sha.is_null() {
        ""
    } else {
        let cstr = unsafe { CStr::from_ptr(expected_sha) };
        cstr.to_str().unwrap_or("")
    };
    match crate::models::ModelManager::verify_checksum(Path::new(path_str), expected) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(_) => -3,
    }
}

fn map_error_code(e: &EngineError) -> c_int {
    match e {
        EngineError::InvalidTransition { .. } => 10,
        EngineError::NotInitialized => 11,
        EngineError::AlreadyInitialized => 12,
        EngineError::Storage(_) => 13,
        EngineError::SettingsValidation(_) => 14,
        EngineError::Audio(_) => 15,
        EngineError::Model(_) => 16,
    }
}

// Re-export for Rust callers / tests.
pub use engine::events::EngineEvent;
pub use engine::state::AppState;
pub use engine::Engine as CoreEngine;
pub use settings::Settings;
pub use storage::Storage;
