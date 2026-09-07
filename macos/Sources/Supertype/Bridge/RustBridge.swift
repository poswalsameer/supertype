import Foundation
import Combine

#if canImport(CSupertypeCore)
import CSupertypeCore
#endif

// MARK: - App State (mirrors Rust AppState)

public enum AppState: UInt8, Codable, Equatable, CaseIterable {
    case idle = 0
    case preparing = 1
    case recording = 2
    case processing = 3
    case completed = 4
    case error = 5

    var displayName: String {
        switch self {
        case .idle: return "Idle"
        case .preparing: return "Preparing"
        case .recording: return "Recording"
        case .processing: return "Processing"
        case .completed: return "Completed"
        case .error: return "Error"
        }
    }
}

// MARK: - Settings (mirrors Rust Settings)

public struct AppSettings: Codable, Equatable {
    public var selectedMicrophoneId: String?
    public var globalShortcut: String
    public var selectedModelId: String
    public var historyEnabled: Bool
    public var launchAtLogin: Bool
    public var overlayEnabled: Bool
    public var shortcutBehavior: String
    public var language: String
    public var punctuationEnabled: Bool
    public var capitalizationEnabled: Bool

    public static var `default`: AppSettings {
        AppSettings(
            selectedMicrophoneId: nil,
            globalShortcut: "fn",
            selectedModelId: "whisper-tiny",
            historyEnabled: true,
            launchAtLogin: false,
            overlayEnabled: true,
            shortcutBehavior: "hold",
            language: "en",
            punctuationEnabled: true,
            capitalizationEnabled: true
        )
    }

    enum CodingKeys: String, CodingKey {
        case selectedMicrophoneId = "selected_microphone_id"
        case globalShortcut = "global_shortcut"
        case selectedModelId = "selected_model_id"
        case historyEnabled = "history_enabled"
        case launchAtLogin = "launch_at_login"
        case overlayEnabled = "overlay_enabled"
        case shortcutBehavior = "shortcut_behavior"
        case language = "language"
        case punctuationEnabled = "punctuation_enabled"
        case capitalizationEnabled = "capitalization_enabled"
    }

    public init(selectedMicrophoneId: String? = nil, globalShortcut: String = "fn", selectedModelId: String = "whisper-tiny", historyEnabled: Bool = true, launchAtLogin: Bool = false, overlayEnabled: Bool = true, shortcutBehavior: String = "hold", language: String = "en", punctuationEnabled: Bool = true, capitalizationEnabled: Bool = true) {
        self.selectedMicrophoneId = selectedMicrophoneId
        self.globalShortcut = globalShortcut
        self.selectedModelId = selectedModelId
        self.historyEnabled = historyEnabled
        self.launchAtLogin = launchAtLogin
        self.overlayEnabled = overlayEnabled
        self.shortcutBehavior = shortcutBehavior
        self.language = language
        self.punctuationEnabled = punctuationEnabled
        self.capitalizationEnabled = capitalizationEnabled
    }
}

// MARK: - Engine Events

public enum EngineEvent: Equatable {
    case recordingStarted
    case recordingStopped
    case speechDetected
    case speechEnded
    case partialTranscript(String)
    case finalTranscript(String)
    case processingStarted
    case processingCompleted
    case error(String)
    case stateChanged(from: String, to: String)
    case unknown(String)

    static func from(json: String) -> EngineEvent? {
        guard let data = json.data(using: .utf8),
              let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let type = obj["type"] as? String else { return nil }
        switch type {
        case "RecordingStarted": return .recordingStarted
        case "RecordingStopped": return .recordingStopped
        case "SpeechDetected": return .speechDetected
        case "SpeechEnded": return .speechEnded
        case "PartialTranscript": return .partialTranscript(obj["payload"] as? String ?? "")
        case "FinalTranscript": return .finalTranscript(obj["payload"] as? String ?? "")
        case "ProcessingStarted": return .processingStarted
        case "ProcessingCompleted": return .processingCompleted
        case "Error": return .error(obj["payload"] as? String ?? "unknown")
        case "StateChanged":
            if let payload = obj["payload"] as? [String: String] {
                return .stateChanged(from: payload["from"] ?? "", to: payload["to"] ?? "")
            }
            return .stateChanged(from: "", to: "")
        default: return .unknown(type)
        }
    }
}

// MARK: - Rust Engine Wrapper (FFI)

/// Thin wrapper around the Rust `Engine` opaque pointer.
/// All calls are synchronous and thread-safe (Rust side uses `parking_lot::Mutex`).
/// Heavy work is never done on the caller thread beyond the mutex lock; actual
/// audio/ML work will be offloaded in Phase 2 to a background Tokio runtime.
public final class RustEngine: ObservableObject {
    private var handle: OpaquePointer?
    private var pollTimer: AnyCancellable?

    @Published public private(set) var state: AppState = .idle
    @Published public private(set) var settings: AppSettings = .default
    @Published public private(set) var lastError: String?
    @Published public private(set) var lastTranscript: String?
    @Published public private(set) var partialTranscript: String?
    @Published public private(set) var metricsJSON: String?

    public let objectWillChange = ObservableObjectPublisher()

    /// Expose handle for AudioCapture (read-only).
    var rustHandle: OpaquePointer? { handle }

    public init() {}

    deinit {
        shutdown()
    }

    // MARK: Lifecycle

    /// Initialize with a persistent database path under Application Support.
    @discardableResult
    public func initialize(dbPath: String? = nil) -> Bool {
        // If Rust is linked, use FFI; otherwise fallback to Swift-only mock.
        #if canImport(CSupertypeCore)
        if Self.loadRustIfAvailable() {
            return initializeViaRust(dbPath: dbPath)
        }
        #endif
        return initializeMock(dbPath: dbPath)
    }

    public func shutdown() {
        pollTimer?.cancel()
        pollTimer = nil
        #if canImport(CSupertypeCore)
        if let h = handle {
            engine_free(h)
            handle = nil
        }
        #endif
    }

    // MARK: Commands (mirrors Rust API)

    @discardableResult
    public func startRecording() -> Bool {
        #if canImport(CSupertypeCore)
        if let h = handle {
            let rc = engine_start_recording(h)
            if rc == 0 { refreshState(); startPolling(); return true }
            lastError = "start_recording failed code \(rc)"
            return false
        }
        #endif
        // Mock: transition logic mirrors Rust state machine
        guard state == .idle else {
            lastError = "invalid transition \(state) -> recording"
            return false
        }
        updateState(.recording)
        return true
    }

    @discardableResult
    public func stopRecording() -> Bool {
        #if canImport(CSupertypeCore)
        if let h = handle {
            let rc = engine_stop_recording(h)
            if rc == 0 { refreshState(); return true }
            lastError = "stop_recording failed \(rc)"
            return false
        }
        #endif
        guard state == .recording else {
            lastError = "can only stop from recording, now \(state)"
            return false
        }
        updateState(.processing)
        // Simulate immediate completion (Phase 1)
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.05) { [weak self] in
            self?.updateState(.completed)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) { [weak self] in
                self?.acknowledge()
            }
        }
        return true
    }

    @discardableResult
    public func cancelRecording() -> Bool {
        #if canImport(CSupertypeCore)
        if let h = handle {
            let rc = engine_cancel_recording(h)
            if rc == 0 { refreshState(); return true }
            return false
        }
        #endif
        if state == .recording || state == .preparing || state == .processing {
            updateState(.idle)
            return true
        }
        return false
    }

    @discardableResult
    public func acknowledge() -> Bool {
        #if canImport(CSupertypeCore)
        if let h = handle {
            let rc = engine_acknowledge(h)
            if rc == 0 { refreshState(); return true }
            return false
        }
        #endif
        if state == .completed || state == .error {
            updateState(.idle)
            return true
        }
        return false
    }

    // MARK: Audio push (Phase 2)

    @discardableResult
    public func pushAudio(_ samples: [Float]) -> Bool {
        #if canImport(CSupertypeCore)
        guard let h = handle, !samples.isEmpty else { return false }
        return samples.withUnsafeBufferPointer { buf in
            let rc = engine_push_audio(h, buf.baseAddress, UInt(buf.count))
            return rc == 0
        }
        #else
        return false
        #endif
    }

    @discardableResult
    public func pushAudioWithFormat(_ samples: [Float], sampleRate: UInt32, channels: UInt32) -> Bool {
        #if canImport(CSupertypeCore)
        guard let h = handle, !samples.isEmpty else { return false }
        return samples.withUnsafeBufferPointer { buf in
            let rc = engine_push_audio_with_format(h, buf.baseAddress, UInt(buf.count), sampleRate, channels)
            return rc == 0
        }
        #else
        return false
        #endif
    }

    public func getMetrics() -> String? {
        #if canImport(CSupertypeCore)
        guard let h = handle, let cstr = engine_get_metrics(h), let s = String(validatingUTF8: cstr) else { return nil }
        engine_string_free(cstr)
        metricsJSON = s
        return s
        #else
        return nil
        #endif
    }

    public func getLastTranscript() -> String? {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return nil }
        if let cstr = engine_get_last_transcript(h) {
            defer { engine_string_free(cstr) }
            if let s = String(validatingUTF8: cstr) {
                lastTranscript = s
                return s
            }
        }
        return nil
        #else
        return lastTranscript
        #endif
    }

    @discardableResult
    public func loadModel(at path: String) -> Bool {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return false }
        let rc = path.withCString { cstr in engine_load_model(h, cstr) }
        if rc == 0 { return true }
        lastError = "load_model \(rc)"
        return false
        #else
        return false
        #endif
    }

    public func unloadModel() {
        #if canImport(CSupertypeCore)
        if let h = handle { _ = engine_unload_model(h) }
        #endif
    }

    public func cancelTranscription() {
        #if canImport(CSupertypeCore)
        if let h = handle { _ = engine_cancel_transcription(h) }
        #endif
    }

    // MARK: Phase 3 — Active app + History + Formatter

    public struct HistoryRecord: Codable, Identifiable {
        public let id: Int64
        public let text: String
        public let model_id: String
        public let created_at: String
        public let duration_ms: Int64?
        public let bundle_id: String?
        public let app_name: String?
        public let confidence: Float?
    }

    public func setActiveApp(bundleId: String?, appName: String?) {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return }
        if let bid = bundleId, let aname = appName {
            bid.withCString { b in aname.withCString { a in _ = engine_set_active_app(h, b, a) } }
        } else if let bid = bundleId {
            bid.withCString { b in _ = engine_set_active_app(h, b, nil) }
        } else if let aname = appName {
            aname.withCString { a in _ = engine_set_active_app(h, nil, a) }
        } else {
            _ = engine_set_active_app(h, nil, nil)
        }
        #endif
    }

    public func formatText(_ raw: String) -> String {
        #if canImport(CSupertypeCore)
        guard let h = handle, let cstr = raw.withCString({ engine_format_text(h, $0) }), let s = String(validatingUTF8: cstr) else { return raw }
        engine_string_free(cstr)
        return s
        #else
        return raw
        #endif
    }

    public func getHistory(limit: Int64 = 50, offset: Int64 = 0) -> [HistoryRecord] {
        #if canImport(CSupertypeCore)
        guard let h = handle, let cstr = engine_get_history(h, limit, offset), let json = String(validatingUTF8: cstr) else { return [] }
        engine_string_free(cstr)
        if let data = json.data(using: .utf8), let recs = try? JSONDecoder().decode([HistoryRecord].self, from: data) {
            return recs
        }
        return []
        #else
        return []
        #endif
    }

    public func deleteHistory(id: Int64) -> Bool {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return false }
        let rc = engine_delete_history(h, id)
        return rc == 0
        #else
        return false
        #endif
    }

    public func clearHistory() -> Bool {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return false }
        return engine_clear_history(h) == 0
        #else
        return false
        #endif
    }

    public func searchHistory(query: String, limit: Int64 = 20) -> [HistoryRecord] {
        #if canImport(CSupertypeCore)
        guard let h = handle, let cstr = query.withCString({ engine_search_history(h, $0, limit) }), let json = String(validatingUTF8: cstr) else { return [] }
        engine_string_free(cstr)
        if let data = json.data(using: .utf8), let recs = try? JSONDecoder().decode([HistoryRecord].self, from: data) {
            return recs
        }
        return []
        #else
        return []
        #endif
    }

    public func getHistoryCount() -> Int64 {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return 0 }
        return engine_get_history_count(h)
        #else
        return 0
        #endif
    }

    // MARK: Catalog / Hardware / Dictionary (Phase 4)

    public struct ModelInfo: Codable, Identifiable {
        public var id: String
        public var display_name: String
        public var size_mb: UInt64
        public var is_downloaded: Bool
        public var quantization: String
        public var runtime: String
        public var local_path: String?
        public var languages: [String]
        public var license: String
        public var checksum: String
        public var is_loaded: Bool
        public var description: String?
        public var family: String?
        public var download_urls: [String]?
        public var capabilities: [String]?
        public var min_memory_mb: UInt32?
        public var is_default: Bool?
        public var is_recommended: Bool?
        public var attribution: String?

        public var isRecommended: Bool { is_recommended ?? false }
        public var isDefault: Bool { is_default ?? false }
    }

    public struct HardwareInfo: Codable {
        public let arch: String
        public let is_apple_silicon: Bool
        public let cpu_cores: UInt32
        public let memory_gb: UInt32
        public let metal_supported: Bool
        public let disk_free_gb: UInt32?
    }

    public func getCatalog() -> [ModelInfo] {
        #if canImport(CSupertypeCore)
        guard let cstr = engine_get_catalog(nil), let json = String(validatingUTF8: cstr) else { return [] }
        engine_string_free(cstr)
        if let data = json.data(using: .utf8), let catalog = try? JSONDecoder().decode([ModelInfo].self, from: data) {
            return catalog
        }
        return []
        #else
        return []
        #endif
    }

    public func getHardwareInfo() -> HardwareInfo? {
        #if canImport(CSupertypeCore)
        guard let cstr = engine_get_hardware_info(nil), let json = String(validatingUTF8: cstr) else { return nil }
        engine_string_free(cstr)
        if let data = json.data(using: .utf8) {
            return try? JSONDecoder().decode(HardwareInfo.self, from: data)
        }
        return nil
        #else
        return nil
        #endif
    }

    public func getRecommendedModels() -> [ModelInfo] {
        #if canImport(CSupertypeCore)
        guard let cstr = engine_get_recommended_models(nil), let json = String(validatingUTF8: cstr) else { return [] }
        engine_string_free(cstr)
        if let data = json.data(using: .utf8), let rec = try? JSONDecoder().decode([ModelInfo].self, from: data) {
            return rec
        }
        return []
        #else
        return []
        #endif
    }

    public func getDictionary() -> [String:String] {
        #if canImport(CSupertypeCore)
        guard let h = handle, let cstr = engine_get_dictionary(h), let json = String(validatingUTF8: cstr) else { return [:] }
        engine_string_free(cstr)
        if let data = json.data(using: .utf8), let dict = try? JSONDecoder().decode([String:String].self, from: data) {
            return dict
        }
        return [:]
        #else
        return [:]
        #endif
    }

    @discardableResult
    public func upsertDictionary(phrase: String, replacement: String) -> Bool {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return false }
        let rc = phrase.withCString { p in replacement.withCString { r in engine_upsert_dictionary(h, p, r) } }
        return rc == 0
        #else
        return false
        #endif
    }

    @discardableResult
    public func deleteDictionary(phrase: String) -> Bool {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return false }
        let rc = phrase.withCString { p in engine_delete_dictionary(h, p) }
        return rc == 0
        #else
        return false
        #endif
    }

    public func verifyModel(path: String, sha: String) -> Bool {
        #if canImport(CSupertypeCore)
        let rc = path.withCString { p in sha.withCString { s in engine_verify_model(p, s) } }
        return rc == 0
        #else
        return false
        #endif
    }

    public func getSettings() -> AppSettings { settings }

    public func updateSettings(_ new: AppSettings) -> Bool {
        // Validate (mirror Rust validation)
        guard !new.globalShortcut.trimmingCharacters(in: .whitespaces).isEmpty,
              !new.selectedModelId.trimmingCharacters(in: .whitespaces).isEmpty else {
            lastError = "validation failed"
            return false
        }
        #if canImport(CSupertypeCore)
        if let h = handle {
            guard let json = try? JSONEncoder().encode(new),
                  let str = String(data: json, encoding: .utf8) else { return false }
            let rc = str.withCString { cstr in engine_update_settings(h, cstr) }
            if rc == 0 {
                settings = new
                objectWillChange.send()
                return true
            }
            lastError = "engine_update_settings \(rc)"
            return false
        }
        #endif
        // Mock persistence: UserDefaults + SQLite via StorageManager later
        settings = new
        persistMockSettings(new)
        objectWillChange.send()
        return true
    }

    // MARK: Private

    private func refreshState() {
        #if canImport(CSupertypeCore)
        if let h = handle {
            let raw = engine_get_state(h)
            if let s = AppState(rawValue: UInt8(max(0, raw))) {
                updateState(s)
            }
        }
        #endif
    }

    private func updateState(_ new: AppState) {
        if state != new {
            let old = state
            state = new
            objectWillChange.send()
            // Publish state change; overlay observes this.
            NotificationCenter.default.post(name: .engineStateChanged, object: nil, userInfo: ["from": old.displayName, "to": new.displayName])
        }
    }

    private func startPolling() {
        pollTimer?.cancel()
        // Poll on background queue while recording/processing; 50ms keeps main wakeups low (was 20ms)
        let queue = DispatchQueue.global(qos: .userInitiated)
        var timer: AnyCancellable?
        timer = Timer.publish(every: 0.05, on: .main, in: .common).autoconnect().sink { [weak self] _ in
            guard let self else { return }
            queue.async { self.pollRustEventsBackground() }
            if self.state == .idle || self.state == .completed {
                timer?.cancel()
                self.pollTimer?.cancel()
            }
        }
        pollTimer = timer
    }

    private func pollRustEventsBackground() {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return }
        var out: UnsafeMutablePointer<CChar>? = nil
        let rc = engine_poll_event(h, &out)
        if rc == 1, let ptr = out, let json = String(validatingUTF8: ptr) {
            engine_string_free(ptr)
            if let ev = EngineEvent.from(json: json) {
                DispatchQueue.main.async { self.handleEvent(ev) }
            }
        } else if rc == 1, let ptr = out {
            engine_string_free(ptr)
        }
        #endif
    }

    private func pollRustEvents() {
        #if canImport(CSupertypeCore)
        guard let h = handle else { return }
        var out: UnsafeMutablePointer<CChar>? = nil
        let rc = engine_poll_event(h, &out)
        if rc == 1, let ptr = out, let json = String(validatingUTF8: ptr) {
            engine_string_free(ptr)
            if let ev = EngineEvent.from(json: json) {
                handleEvent(ev)
            }
        } else if rc == 1, let ptr = out {
            engine_string_free(ptr)
        }
        #endif
    }

    private func handleEvent(_ ev: EngineEvent) {
        switch ev {
        case .stateChanged(_, let to):
            if let s = AppState.allCases.first(where: { $0.displayName.lowercased() == to.lowercased() }) {
                updateState(s)
            }
        case .partialTranscript(let text):
            partialTranscript = text
            objectWillChange.send()
        case .finalTranscript(let text):
            lastTranscript = text
            partialTranscript = nil
            objectWillChange.send()
            // Fetch metrics after final
            _ = getMetrics()
        case .speechDetected, .speechEnded:
            // propagate for UI debugging
            break
        default: break
        }
        NotificationCenter.default.post(name: .engineEvent, object: ev)
    }

    // MARK: Rust dynamic loading fallback

    private struct RustSymbols {
        let new: () -> OpaquePointer?
        let free: (OpaquePointer?) -> Void
    }

    private static func loadRustIfAvailable() -> Bool {
        // We link statically, so if we are here and canImport, Rust is available.
        // This helper just checks the dylib exists for diagnostic purposes.
        return true
    }

    private func initializeViaRust(dbPath: String?) -> Bool {
        #if canImport(CSupertypeCore)
        let h = engine_new()
        guard h != nil else { lastError = "engine_new failed"; return false }
        handle = h
        // Resolve DB path
        let path: String
        if let p = dbPath { path = p }
        else { path = Self.defaultDBPath() }
        let rc: Int32 = path.withCString { cstr in engine_initialize(h, cstr) }
        if rc != 0 {
            lastError = "engine_initialize \(rc) path=\(path)"
            engine_free(h)
            handle = nil
            return false
        }
        // Load settings
        if let cstr = engine_get_settings(h), let json = String(validatingUTF8: cstr) {
            if let data = json.data(using: .utf8), let decoded = try? JSONDecoder().decode(AppSettings.self, from: data) {
                settings = decoded
            }
            engine_string_free(cstr)
        }
        refreshState()
        startPolling()
        return true
        #else
        return false
        #endif
    }

    private func initializeMock(dbPath: String?) -> Bool {
        // Load from UserDefaults / file as mock persistence
        if let data = UserDefaults.standard.data(forKey: "supertype.settings"),
           let decoded = try? JSONDecoder().decode(AppSettings.self, from: data) {
            settings = decoded
        }
        state = .idle
        return true
    }

    private func persistMockSettings(_ s: AppSettings) {
        if let data = try? JSONEncoder().encode(s) {
            UserDefaults.standard.set(data, forKey: "supertype.settings")
        }
    }

    static func defaultDBPath() -> String {
        let fm = FileManager.default
        let base = fm.urls(for: .applicationSupportDirectory, in: .userDomainMask).first!
            .appendingPathComponent("Supertype", isDirectory: true)
        try? fm.createDirectory(at: base, withIntermediateDirectories: true)
        return base.appendingPathComponent("supertype.db").path
    }
}

public extension Notification.Name {
    static let engineStateChanged = Notification.Name("engineStateChanged")
    static let engineEvent = Notification.Name("engineEvent")
}
