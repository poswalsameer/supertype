#pragma once
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle to Engine
typedef struct Engine Engine;

// Lifecycle
Engine* engine_new(void);
void engine_free(Engine* ptr);
int32_t engine_initialize(Engine* ptr, const char* db_path);
int32_t engine_initialize_in_memory(Engine* ptr);
int32_t engine_get_state(Engine* ptr);
int32_t engine_start_recording(Engine* ptr);
int32_t engine_stop_recording(Engine* ptr);
int32_t engine_cancel_recording(Engine* ptr);
int32_t engine_acknowledge(Engine* ptr);

// Settings
char* engine_get_settings(Engine* ptr);
int32_t engine_update_settings(Engine* ptr, const char* json);
void engine_string_free(char* s);

// Events
int32_t engine_poll_event(Engine* ptr, char** out_json);

// Audio ingestion (Phase 2)
int32_t engine_push_audio(Engine* ptr, const float* data, uintptr_t len);
int32_t engine_push_audio_with_format(Engine* ptr, const float* data, uintptr_t len, uint32_t sample_rate, uint32_t channels);

// Metrics / model
char* engine_get_metrics(Engine* ptr);
char* engine_get_last_transcript(Engine* ptr);
char* engine_get_model_info(Engine* ptr);
int32_t engine_load_model(Engine* ptr, const char* path);
int32_t engine_unload_model(Engine* ptr);
int32_t engine_cancel_transcription(Engine* ptr);

// Active app + history + formatter (Phase 3)
int32_t engine_set_active_app(Engine* ptr, const char* bundle_id, const char* app_name);
char* engine_format_text(Engine* ptr, const char* raw);
char* engine_get_history(Engine* ptr, int64_t limit, int64_t offset);
int32_t engine_delete_history(Engine* ptr, int64_t id);
int32_t engine_clear_history(Engine* ptr);
char* engine_search_history(Engine* ptr, const char* query, int64_t limit);
int64_t engine_get_history_count(Engine* ptr);

// Catalog / hardware / dictionary (Phase 4)
char* engine_get_catalog(Engine* ptr);
char* engine_get_hardware_info(Engine* ptr);
char* engine_get_recommended_models(Engine* ptr);
int32_t engine_upsert_dictionary(Engine* ptr, const char* phrase, const char* replacement);
char* engine_get_dictionary(Engine* ptr);
int32_t engine_delete_dictionary(Engine* ptr, const char* phrase);
int32_t engine_verify_model(const char* path, const char* expected_sha);

// Version
const char* engine_version(void);

#ifdef __cplusplus
}
#endif
