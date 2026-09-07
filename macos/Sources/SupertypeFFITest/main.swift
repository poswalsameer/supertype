import Foundation
import CSupertypeCore

func check(_ rc: Int32, _ label: String) {
    if rc != 0 { print("FAIL \(label) rc=\(rc)"); exit(1) }
}

let e = engine_new()
guard e != nil else { print("engine_new null"); exit(1) }

// Use in-memory DB so test never touches user's real DB
check(engine_initialize_in_memory(e), "init_in_memory")

let s0 = engine_get_state(e)
if s0 != 0 { print("expected idle 0 got \(s0)"); exit(1) }
print("✓ initial state idle")

// ── Phase 1 happy path (no audio) ──────────────────────────
check(engine_start_recording(e), "start")
if engine_get_state(e) != 2 { print("expected recording 2 got \(engine_get_state(e))"); exit(1) }
print("✓ recording")

check(engine_stop_recording(e), "stop")
if engine_get_state(e) != 4 { print("expected completed 4 got \(engine_get_state(e))"); exit(1) }
print("✓ completed (no audio)")

check(engine_acknowledge(e), "ack")
if engine_get_state(e) != 0 { print("expected idle after ack got \(engine_get_state(e))"); exit(1) }
print("✓ ack to idle")

// ── Phase 2: streaming audio pipeline ─────────────────────
check(engine_start_recording(e), "start2")
print("✓ recording2")
// Push 16k mono PCM (1 sec 440Hz tone)
var pcm: [Float] = (0..<16000).map { i in sin(2 * Float.pi * 440 * Float(i) / 16000) * 0.3 }
pcm.withUnsafeBufferPointer { buf in
    let rc = engine_push_audio(e, buf.baseAddress, UInt(buf.count))
    if rc != 0 { print("FAIL push_audio rc=\(rc)"); exit(1) }
}
print("✓ push_audio 16k mono (1s)")

// Push with format conversion (48k stereo → should resample)
let pcm48kStereo: [Float] = (0..<96000).map { i in sin(Float(i) * 0.01) * 0.2 } // 1s @48k stereo interleaved (2*48000)
pcm48kStereo.withUnsafeBufferPointer { buf in
    let rc = engine_push_audio_with_format(e, buf.baseAddress, UInt(buf.count), 48000, 2)
    if rc != 0 { print("FAIL push_audio_with_format rc=\(rc)"); exit(1) }
}
print("✓ push_audio_with_format 48k stereo")

// Poll for speech/partal events
var sawSpeech = false
var sawPartial = false
for _ in 0..<20 {
    var out: UnsafeMutablePointer<CChar>? = nil
    let rc = engine_poll_event(e, &out)
    if rc == 1, let p = out, let j = String(validatingUTF8: p) {
        if j.contains("SpeechDetected") { sawSpeech = true }
        if j.contains("PartialTranscript") { sawPartial = true }
        engine_string_free(p)
    } else if rc == 0 { break }
}
print("✓ poll events speech=\(sawSpeech) partial=\(sawPartial)")

check(engine_stop_recording(e), "stop2")
print("✓ stop2 -> processing->completed")

// Check final transcript
var finalText: String? = nil
for _ in 0..<30 {
    var out: UnsafeMutablePointer<CChar>? = nil
    let rc = engine_poll_event(e, &out)
    if rc == 1, let p = out, let j = String(validatingUTF8: p) {
        if j.contains("FinalTranscript") {
            finalText = j
        }
        engine_string_free(p)
    } else if rc == 0 { break }
}
if let t = finalText {
    print("✓ final_transcript: \(t.prefix(200))")
} else {
    print("ℹ no final transcript (silence?) — checking last_transcript")
    if let cstr = engine_get_last_transcript(e), let s = String(validatingUTF8: cstr) {
        print("  last_transcript: \(s.prefix(200))")
        engine_string_free(cstr)
    }
}

// Metrics
if let cstr = engine_get_metrics(e), let j = String(validatingUTF8: cstr) {
    print("✓ metrics: \(j.prefix(300))")
    engine_string_free(cstr)
    if !j.contains("asr_ms") { print("FAIL metrics missing asr_ms"); exit(1) }
} else {
    print("FAIL metrics null"); exit(1)
}

// Model info
if let cstr = engine_get_model_info(e), let j = String(validatingUTF8: cstr) {
    print("✓ model_info: \(j.prefix(200))")
    engine_string_free(cstr)
}

// Cancel path
check(engine_acknowledge(e), "ack2")
check(engine_start_recording(e), "start3")
pcm.withUnsafeBufferPointer { buf in _ = engine_push_audio(e, buf.baseAddress, UInt(buf.count)) }
check(engine_cancel_recording(e), "cancel")
if engine_get_state(e) != 0 { print("expected idle after cancel got \(engine_get_state(e))"); exit(1) }
print("✓ cancel discards")

// Corrupted model handling (load should fail)
let badPath = "/tmp/does-not-exist-12345.bin"
let rcBad = badPath.withCString { c in engine_load_model(e, c) }
if rcBad == 0 { print("FAIL load bad model should fail"); exit(1) }
print("✓ corrupted/missing model handling rc=\(rcBad)")

// Settings roundtrip
if let cstr = engine_get_settings(e), let json = String(validatingUTF8: cstr) {
    print("✓ get_settings: \(json.prefix(80))")
    engine_string_free(cstr)
    let modified = json.replacingOccurrences(of: "whisper-tiny", with: "whisper-base")
    check(engine_update_settings(e, (modified as NSString).utf8String), "update_settings")
    print("✓ update_settings")
}

var out: UnsafeMutablePointer<CChar>? = nil
let rc2 = engine_poll_event(e, &out)
if rc2 == 1, let p = out, let j = String(validatingUTF8: p) {
    print("✓ poll_event: \(j.prefix(120))")
    engine_string_free(p)
}

engine_free(e)
print("All FFI checks passed (Phase 1 + 2)")
