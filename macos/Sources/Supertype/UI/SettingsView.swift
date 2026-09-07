import SwiftUI
import ServiceManagement
import AVFoundation
import AppKit

struct SettingsView: View {
    @EnvironmentObject var engine: RustEngine
    @State private var selectedTab: String = "General"

    var body: some View {
        TabView(selection: $selectedTab) {
            GeneralTab().tabItem { Label("General", systemImage: "gear") }.tag("General")
            MicrophoneTab().tabItem { Label("Microphone", systemImage: "mic") }.tag("Microphone")
            SpeechTab().tabItem { Label("Speech", systemImage: "waveform") }.tag("Speech")
            HistoryTab().tabItem { Label("History", systemImage: "clock") }.tag("History")
            PrivacyTab().tabItem { Label("Privacy", systemImage: "hand.raised") }.tag("Privacy")
            AboutTab().tabItem { Label("About", systemImage: "info.circle") }.tag("About")
        }
        .frame(width: 700, height: 520)
        .padding()
    }
}

// MARK: - General (launch, shortcut, hold/toggle, overlay)

private struct GeneralTab: View {
    @EnvironmentObject var engine: RustEngine
    @State private var launchAtLogin: Bool = false
    @State private var hotkeyInput: String = "fn"
    @State private var behavior: String = "hold"
    @State private var overlayEnabled: Bool = true
    @State private var hotkeyError: String?
    @StateObject private var hotkeyManager = HotkeyManager()

    var body: some View {
        Form {
            Section("Launch") {
                Toggle("Launch at login", isOn: $launchAtLogin)
                    .onChange(of: launchAtLogin) { _, v in
                        LaunchAtLoginManager.shared.setEnabled(v)
                        var s = engine.getSettings(); s.launchAtLogin = v; _ = engine.updateSettings(s)
                    }
                Toggle("Show recording overlay", isOn: Binding(
                    get: { engine.getSettings().overlayEnabled },
                    set: { v in var s = engine.getSettings(); s.overlayEnabled = v; _ = engine.updateSettings(s); overlayEnabled = v }
                ))
            }
            Section("Global Shortcut") {
                HStack {
                    TextField("Shortcut", text: $hotkeyInput).onSubmit { apply() }
                    Button("Apply") { apply() }
                    Image(systemName: hotkeyManager.isRegistered ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
                        .foregroundStyle(hotkeyManager.isRegistered ? .green : .orange)
                }
                Picker("Behavior", selection: $behavior) {
                    Text("Hold to talk").tag("hold")
                    Text("Toggle (press to start/stop)").tag("toggle")
                }.onChange(of: behavior) { _, v in
                    var s = engine.getSettings(); s.shortcutBehavior = v; _ = engine.updateSettings(s)
                }
                if let err = hotkeyError ?? hotkeyManager.lastError {
                    Text(err).font(.caption).foregroundStyle(.red)
                }
                Text("Hold behavior is recommended for immediacy. Shortcut works globally even when Supertype is in background. Default: “fn”. Alternatives: “ctrl+space”, “option+space”.")
                    .font(.caption).foregroundStyle(.secondary)
                HStack {
                    Button("Use fn") { hotkeyInput = "fn"; apply() }
                    Button("Use ctrl+space") { hotkeyInput = "ctrl+space"; apply() }
                }.font(.caption)
            }
            Section("Status") {
                HStack { Text("State"); Spacer(); Text(engine.state.displayName).foregroundStyle(.secondary).monospaced() }
                if let err = engine.lastError { Text(err).font(.caption).foregroundStyle(.red) }
            }
        }
        .formStyle(.grouped)
        .onAppear {
            let s = engine.getSettings()
            launchAtLogin = s.launchAtLogin
            hotkeyInput = s.globalShortcut
            behavior = s.shortcutBehavior
            overlayEnabled = s.overlayEnabled
        }
        .onReceive(engine.objectWillChange) { _ in
            let s = engine.getSettings()
            if hotkeyInput != s.globalShortcut { hotkeyInput = s.globalShortcut }
            if behavior != s.shortcutBehavior { behavior = s.shortcutBehavior }
        }
    }
    private func apply() {
        let trimmed = hotkeyInput.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { hotkeyError = "shortcut empty"; return }
        let ok = hotkeyManager.register(shortcut: trimmed)
        if ok {
            var s = engine.getSettings(); s.globalShortcut = trimmed; s.shortcutBehavior = behavior
            if engine.updateSettings(s) { hotkeyError = nil } else { hotkeyError = "failed to save" }
        } else {
            hotkeyError = hotkeyManager.lastError ?? "registration failed (conflict?)"
        }
    }
}

// MARK: - Microphone (device UID, level)

private struct MicrophoneTab: View {
    @EnvironmentObject var engine: RustEngine
    @State private var level: Float = 0
    @State private var testing = false
    @State private var testEngine: AVAudioEngine?
    @State private var selectedId: String = "default"
    @StateObject private var micPerm = MicrophonePermission.shared

    var body: some View {
        Form {
            Section("Input Device") {
                Picker("Microphone", selection: $selectedId) {
                    Text("System Default").tag("default")
                    ForEach(availableMics(), id: \.id) { dev in Text(dev.name).tag(dev.id) }
                }.onChange(of: selectedId) { _, v in
                    var s = engine.getSettings()
                    s.selectedMicrophoneId = (v == "default" ? nil : v)
                    _ = engine.updateSettings(s)
                }
                Text("Changes apply on next recording. System default follows macOS Sound settings.")
                    .font(.caption).foregroundStyle(.secondary)
                HStack {
                    Circle().fill(micPerm.isGranted ? Color.green : Color.orange).frame(width: 8, height: 8)
                    Text(micPerm.statusLabel).font(.caption)
                    Spacer()
                    Button("Request Access") { micPerm.request() }
                    Button("Open Settings") { micPerm.openSystemSettings() }
                }
            }
            Section("Input Level") {
                HStack {
                    ProgressView(value: level).frame(width: 200)
                    Text("\(Int(level*100))%").font(.caption).monospaced()
                    Spacer()
                    Button(testing ? "Stop Test" : "Test Microphone") { toggleTest() }
                }
                Text("Speak normally — level should peak ~60-80%. No audio is saved.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .onAppear {
            selectedId = engine.getSettings().selectedMicrophoneId ?? "default"
            _ = micPerm.currentStatus()
        }
        .onDisappear { stopTest() }
    }

    private struct MicDevice: Identifiable { let id: String; let name: String }
    private func availableMics() -> [MicDevice] {
        let session = AVCaptureDevice.DiscoverySession(deviceTypes: [.builtInMicrophone, .externalUnknown], mediaType: .audio, position: .unspecified)
        return session.devices.map { MicDevice(id: $0.uniqueID, name: $0.localizedName) }
    }
    private func toggleTest() {
        if testing { stopTest() } else { startTest() }
    }
    private func startTest() {
        guard micPerm.isGranted else { micPerm.request { g in if g { startTest() } }; return }
        let eng = AVAudioEngine()
        let input = eng.inputNode
        let format = input.outputFormat(forBus: 0)
        input.installTap(onBus: 0, bufferSize: 1024, format: format) { buf, _ in
            guard let ch = buf.floatChannelData?[0] else { return }
            let n = Int(buf.frameLength)
            var sum: Float = 0
            for i in 0..<n { sum += abs(ch[i]) }
            let avg = n > 0 ? sum / Float(n) : 0
            DispatchQueue.main.async { level = min(1, avg * 6) }
        }
        do { try eng.start(); testEngine = eng; testing = true } catch { level = 0 }
    }
    private func stopTest() {
        testEngine?.inputNode.removeTap(onBus: 0)
        testEngine?.stop()
        testEngine = nil
        testing = false
        level = 0
    }
}

// MARK: - Speech (model, language, formatting, vocabulary)

private struct SpeechTab: View {
    @EnvironmentObject var engine: RustEngine
    @State private var language: String = "en"
    @State private var punct: Bool = true
    @State private var caps: Bool = true

    var body: some View {
        TabView {
            ModelCatalogView().environmentObject(engine).tabItem { Text("Models") }.tag(0)
            DictionaryView().environmentObject(engine).tabItem { Text("Vocabulary") }.tag(1)
            formattingView.tabItem { Text("Formatting") }.tag(2)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onAppear {
            let s = engine.getSettings()
            language = s.language; punct = s.punctuationEnabled; caps = s.capitalizationEnabled
        }
    }

    private var formattingView: some View {
        Form {
            Section("Language") {
                Picker("Language", selection: $language) {
                    Text("English").tag("en")
                    Text("Auto-detect").tag("auto")
                    Text("Multilingual").tag("multilingual")
                }.onChange(of: language) { _, v in var s = engine.getSettings(); s.language = v; _ = engine.updateSettings(s) }
                Text("Language affects model selection. Tiny models are English-biased; Parakeet is multilingual.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section("Formatting") {
                Toggle("Spoken punctuation (comma → ,)", isOn: $punct)
                    .onChange(of: punct) { _, v in var s = engine.getSettings(); s.punctuationEnabled = v; _ = engine.updateSettings(s) }
                Toggle("Capitalize sentences", isOn: $caps)
                    .onChange(of: caps) { _, v in var s = engine.getSettings(); s.capitalizationEnabled = v; _ = engine.updateSettings(s) }
                Text("Formatting is deterministic and local — no LLM rewrite. Example: “hello comma world period” → “Hello, world.”")
                    .font(.caption).foregroundStyle(.secondary)
                Button("Test Formatting") {
                    let raw = "hello comma world period new line this is supertype"
                    let out = engine.formatText(raw)
                    NSSound.beep()
                    print("[Speech] format test: \(out)")
                }.font(.caption)
            }
        }.formStyle(.grouped)
    }
}

// MARK: - History

private struct HistoryTab: View {
    @EnvironmentObject var engine: RustEngine
    var body: some View { HistoryView().environmentObject(engine) }
}

// MARK: - Privacy (explicit behavior)

private struct PrivacyTab: View {
    @EnvironmentObject var engine: RustEngine
    var body: some View {
        Form {
            Section("What happens to your voice") {
                Text("1. You hold the shortcut — microphone starts (in memory only).\n2. Voice → VAD → local ASR → formatted text.\n3. Text is inserted where you type. Audio is discarded.\n4. Optional history saves only the final text locally in SQLite.")
                    .font(.callout)
                Label("No audio is ever written to disk. No cloud transcription.", systemImage: "lock.shield.fill")
                    .font(.caption).foregroundStyle(.green)
            }
            Section("History") {
                HStack {
                    Text("Save transcription history")
                    Spacer()
                    Toggle("", isOn: Binding(
                        get: { engine.getSettings().historyEnabled },
                        set: { v in var s = engine.getSettings(); s.historyEnabled = v; _ = engine.updateSettings(s) }
                    )).labelsHidden()
                }
                HStack {
                    Text("Entries: \(engine.getHistoryCount())")
                    Spacer()
                    Button("Clear History") { _ = engine.clearHistory() }
                    Button("Reveal Database") { StorageManager.shared.revealInFinder() }
                }
                Text("Location: ~/Library/Application Support/Supertype/supertype.db (SQLite, WAL). Text-only, 30-day retention. Use Clear to purge.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section("Network") {
                Text("Supertype does not contact any server during dictation. Network is used only when you tap Download in Models — HTTPS to Hugging Face, verified by SHA-256, atomic install. No telemetry by default.")
                    .font(.caption)
                Button("Open Privacy Audit") {
                    if let url = Bundle.main.url(forResource: "privacy", withExtension: "md") {
                        NSWorkspace.shared.open(url)
                    } else if let url = URL(string: "file://\(FileManager.default.currentDirectoryPath)/docs/privacy.md") {
                        NSWorkspace.shared.open(url)
                    }
                }
            }
            Section("Permissions") {
                PermissionSummaryView()
            }
        }.formStyle(.grouped)
    }
}

private struct PermissionSummaryView: View {
    @StateObject private var mic = MicrophonePermission.shared
    @StateObject private var ax = AccessibilityPermission.shared
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack { Circle().fill(mic.isGranted ? Color.green : Color.orange).frame(width: 8, height: 8); Text("Microphone: \(mic.statusLabel)").font(.caption) }
            HStack { Circle().fill(ax.isGranted ? Color.green : Color.orange).frame(width: 8, height: 8); Text("Accessibility: \(ax.statusLabel)").font(.caption) }
        }
        .onAppear { _ = mic.currentStatus(); _ = ax.isTrusted() }
    }
}

// MARK: - About (version, licenses, diagnostics)

private struct AboutTab: View {
    @EnvironmentObject var engine: RustEngine
    @State private var diagnostics: String = ""
    @State private var showDiag = false

    var body: some View {
        Form {
            Section("Supertype") {
                HStack { Text("Version"); Spacer(); Text(appVersion).foregroundStyle(.secondary) }
                HStack { Text("macOS"); Spacer(); Text(macOSVersion).foregroundStyle(.secondary) }
                HStack { Text("Model runtime"); Spacer(); Text(engine.getMetrics().flatMap { parseBackend($0) } ?? "—").foregroundStyle(.secondary) }
                if let err = engine.lastError { Text(err).font(.caption).foregroundStyle(.red) }
            }
            Section("Licenses") {
                Text("App: MIT (see LICENSE). Models: whisper.cpp MIT, Parakeet TDT 0.6B CC-BY-4.0 — attribution shown per model in Speech → Models.")
                    .font(.caption)
                Link("View Privacy Documentation", destination: URL(string: "file://\(FileManager.default.currentDirectoryPath)/docs/privacy.md") ?? URL(fileURLWithPath: "/tmp"))
                    .font(.caption)
            }
            Section("Diagnostics (local only)") {
                Text("Copy diagnostics for troubleshooting — not sent automatically.")
                    .font(.caption).foregroundStyle(.secondary)
                Button(showDiag ? "Hide Diagnostics" : "Show Diagnostics") {
                    if !showDiag { diagnostics = buildDiagnostics() }
                    showDiag.toggle()
                }
                if showDiag {
                    ScrollView { Text(diagnostics).font(.system(.caption, design: .monospaced)).textSelection(.enabled).frame(maxWidth: .infinity, alignment: .leading) }
                        .frame(height: 160).background(Color.primary.opacity(0.05)).clipShape(RoundedRectangle(cornerRadius: 8))
                    HStack {
                        Button("Copy") { NSPasteboard.general.clearContents(); NSPasteboard.general.setString(diagnostics, forType: .string) }
                        Button("Export…") { exportDiagnostics() }
                    }.font(.caption)
                }
            }
        }.formStyle(.grouped)
    }

    private var appVersion: String {
        let v = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "0.1.0"
        let b = Bundle.main.infoDictionary?["CFBundleVersion"] as? String ?? "1"
        return "\(v) (\(b))"
    }
    private var macOSVersion: String {
        let v = ProcessInfo.processInfo.operatingSystemVersion
        return "\(v.majorVersion).\(v.minorVersion).\(v.patchVersion)"
    }
    private func parseBackend(_ json: String) -> String? {
        if let data = json.data(using: .utf8),
           let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            return obj["backend"] as? String
        }
        return nil
    }
    private func buildDiagnostics() -> String {
        var lines: [String] = []
        lines.append("Supertype Diagnostics — \(Date())")
        lines.append("App version: \(appVersion)")
        lines.append("macOS: \(macOSVersion) arch=\(engine.getHardwareInfo()?.arch ?? "?")")
        if let hw = engine.getHardwareInfo() {
            lines.append("Hardware: \(hw.cpu_cores) cores \(hw.memory_gb)GB metal=\(hw.metal_supported) free=\(hw.disk_free_gb ?? 0)GB")
        }
        let s = engine.getSettings()
        lines.append("Settings: shortcut=\(s.globalShortcut) behav=\(s.shortcutBehavior) model=\(s.selectedModelId) lang=\(s.language) punct=\(s.punctuationEnabled) caps=\(s.capitalizationEnabled) overlay=\(s.overlayEnabled) history=\(s.historyEnabled)")
        lines.append("State: \(engine.state.displayName) lastError=\(engine.lastError ?? "none")")
        if let m = engine.getMetrics() { lines.append("Metrics: \(m)") }
        if let c = try? JSONEncoder().encode(engine.getHardwareInfo()), let js = String(data: c, encoding: .utf8) { lines.append("HW JSON: \(js)") }
        lines.append("History count: \(engine.getHistoryCount()) db=\(StorageManager.shared.dbExists ? "exists" : "none") size=\(StorageManager.shared.dbSizeBytes) bytes")
        lines.append("Permissions: mic=\(MicrophonePermission.shared.currentStatus().rawValue) ax=\(AccessibilityPermission.shared.isTrusted())")
        return lines.joined(separator: "\n")
    }
    private func exportDiagnostics() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "supertype-diagnostics.txt"
        if panel.runModal() == .OK, let url = panel.url {
            try? buildDiagnostics().write(to: url, atomically: true, encoding: .utf8)
        }
    }
}
