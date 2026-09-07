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

// Version
const char* engine_version(void);

#ifdef __cplusplus
}
#endif
