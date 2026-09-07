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

check(engine_start_recording(e), "start")
if engine_get_state(e) != 2 { print("expected recording 2 got \(engine_get_state(e))"); exit(1) }
print("✓ recording")

check(engine_stop_recording(e), "stop")
// stop goes Recording->Processing->Completed (4)
if engine_get_state(e) != 4 { print("expected completed 4 got \(engine_get_state(e))"); exit(1) }
print("✓ completed")

check(engine_acknowledge(e), "ack")
if engine_get_state(e) != 0 { print("expected idle after ack got \(engine_get_state(e))"); exit(1) }
print("✓ ack to idle")

// Test settings roundtrip
if let cstr = engine_get_settings(e), let json = String(validatingUTF8: cstr) {
    print("✓ get_settings: \(json.prefix(80))")
    engine_string_free(cstr)
    // update
    let modified = json.replacingOccurrences(of: "whisper-tiny", with: "whisper-base")
    check(engine_update_settings(e, (modified as NSString).utf8String), "update_settings")
    print("✓ update_settings")
}

var out: UnsafeMutablePointer<CChar>? = nil
let rc = engine_poll_event(e, &out)
if rc == 1, let p = out, let j = String(validatingUTF8: p) {
    print("✓ poll_event: \(j.prefix(120))")
    engine_string_free(p)
} else {
    print("poll no event (rc=\(rc)) — still ok, events drained")
}

engine_free(e)
print("All FFI checks passed")
