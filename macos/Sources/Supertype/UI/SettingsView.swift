import SwiftUI
import ServiceManagement
import AVFoundation

struct SettingsView: View {
    @EnvironmentObject var engine: RustEngine
    @StateObject private var micPermission = MicrophonePermission.shared
    @StateObject private var axPermission = AccessibilityPermission.shared
    @StateObject private var hotkeyManager = HotkeyManager()
    @State private var launchAtLogin: Bool = false
    @State private var selectedTab: String = "General"
    @State private var hotkeyInput: String = "fn"
    @State private var hotkeyError: String?

    var body: some View {
        TabView(selection: $selectedTab) {
            generalTab.tabItem { Label("General", systemImage: "gear") }.tag("General")
            permissionsTab.tabItem { Label("Permissions", systemImage: "lock.shield") }.tag("Permissions")
            historyTab.tabItem { Label("History", systemImage: "clock") }.tag("History")
            modelsTab.tabItem { Label("Models", systemImage: "cpu") }.tag("Models")
            dictionaryTab.tabItem { Label("Vocabulary", systemImage: "book") }.tag("Dictionary")
            privacyTab.tabItem { Label("Privacy", systemImage: "hand.raised") }.tag("Privacy")
        }
        .frame(width: 680, height: 520)
        .padding()
        .onAppear { syncFromEngine(); hotkeyInput = engine.getSettings().globalShortcut }
        .onReceive(engine.objectWillChange) { _ in syncFromEngine() }
    }

    private func syncFromEngine() {
        let s = engine.getSettings()
        launchAtLogin = s.launchAtLogin
        if hotkeyInput != s.globalShortcut { hotkeyInput = s.globalShortcut }
    }

    // MARK: General

    private var generalTab: some View {
        Form {
            Section("Input") {
                Picker("Microphone", selection: Binding(
                    get: { engine.getSettings().selectedMicrophoneId ?? "default" },
                    set: { new in
                        var s = engine.getSettings()
                        s.selectedMicrophoneId = (new == "default" ? nil : new)
                        _ = engine.updateSettings(s)
                    }
                )) {
                    Text("System Default").tag("default")
                    ForEach(availableMicrophones(), id: \.self) { name in
                        Text(name).tag(name)
                    }
                }
                Text("System default is used unless you pick a specific device. Changes take effect on next recording.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            Section("Hotkey") {
                HStack {
                    TextField("Global Shortcut", text: $hotkeyInput)
                        .onSubmit { applyHotkey() }
                    Button("Apply") { applyHotkey() }
                    if hotkeyManager.isRegistered {
                        Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                    } else {
                        Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
                    }
                }
                if let err = hotkeyError ?? hotkeyManager.lastError {
                    Text(err).font(.caption).foregroundStyle(.red)
                }
                Text("Hold this shortcut to dictate. Works even when Supertype is not focused. Default: “fn”. Alternatives: “ctrl+space”, “option+space”, “cmd+shift+space”.")
                    .font(.caption).foregroundStyle(.secondary)
                HStack {
                    Button("Use fn") { hotkeyInput = "fn"; applyHotkey() }
                    Button("Use ctrl+space") { hotkeyInput = "ctrl+space"; applyHotkey() }
                }.font(.caption)
            }

            Section("Behavior") {
                Toggle("Enable recording overlay", isOn: Binding(
                    get: { engine.getSettings().overlayEnabled },
                    set: { v in var s = engine.getSettings(); s.overlayEnabled = v; _ = engine.updateSettings(s) }
                ))
                Toggle("Keep transcription history", isOn: Binding(
                    get: { engine.getSettings().historyEnabled },
                    set: { v in var s = engine.getSettings(); s.historyEnabled = v; _ = engine.updateSettings(s) }
                ))
                Toggle("Launch at login", isOn: $launchAtLogin)
                    .onChange(of: launchAtLogin) { _, newValue in
                        LaunchAtLoginManager.shared.setEnabled(newValue)
                        var s = engine.getSettings()
                        s.launchAtLogin = newValue
                        _ = engine.updateSettings(s)
                    }
            }

            Section("Engine") {
                HStack {
                    Text("State")
                    Spacer()
                    Text(engine.state.displayName).foregroundStyle(.secondary).monospaced()
                }
                HStack {
                    Text("Model")
                    Spacer()
                    Text(engine.getSettings().selectedModelId).foregroundStyle(.secondary)
                }
                if let t = engine.lastTranscript {
                    VStack(alignment: .leading) {
                        Text("Last transcript:").font(.caption).foregroundStyle(.secondary)
                        Text(t).font(.caption)
                    }
                    HStack {
                        Button("Copy Last") { copyLast() }
                        Button("Clear") { _ = engine.acknowledge() }
                    }.font(.caption)
                }
                if let err = engine.lastError {
                    Text(err).font(.caption).foregroundStyle(.red)
                }
                if let partial = engine.partialTranscript, !partial.isEmpty {
                    Text("Partial: \(partial)").font(.caption).foregroundStyle(.orange).lineLimit(2)
                }
            }
        }
        .formStyle(.grouped)
    }

    private func applyHotkey() {
        let trimmed = hotkeyInput.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { hotkeyError = "shortcut empty"; return }
        let ok = hotkeyManager.register(shortcut: trimmed)
        if ok {
            var s = engine.getSettings()
            s.globalShortcut = trimmed
            if engine.updateSettings(s) {
                hotkeyError = nil
            } else {
                hotkeyError = "failed to save"
            }
        } else {
            hotkeyError = hotkeyManager.lastError ?? "registration failed (conflict?)"
        }
    }

    private func availableMicrophones() -> [String] {
        // Simple enumeration via AVFoundation (macOS)
        let discovery = AVCaptureDevice.DiscoverySession(deviceTypes: [.builtInMicrophone, .externalUnknown], mediaType: .audio, position: .unspecified)
        return discovery.devices.map { $0.localizedName }
    }

    private func copyLast() {
        if let t = engine.lastTranscript {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(t, forType: .string)
        }
    }

    // MARK: Permissions

    private var permissionsTab: some View {
        Form {
            Section("Microphone") {
                HStack {
                    Circle().fill(micPermission.isGranted ? Color.green : Color.orange).frame(width: 10, height: 10)
                    Text(micPermission.statusLabel).font(.headline)
                    Spacer()
                }
                Text("Transcription runs locally. Microphone audio is never sent to a server.")
                    .font(.caption).foregroundStyle(.secondary)
                HStack {
                    Button("Request Microphone Access") { micPermission.request() }
                    Button("Open System Settings") { micPermission.openSystemSettings() }
                }
                if micPermission.status == .denied {
                    Text("Denied — enable in System Settings → Privacy & Security → Microphone, then relaunch.")
                        .font(.caption).foregroundStyle(.red)
                }
            }
            Section("Accessibility — Text Insertion") {
                HStack {
                    Circle().fill(axPermission.isGranted ? Color.green : Color.orange).frame(width: 10, height: 10)
                    Text(axPermission.statusLabel).font(.headline)
                    Spacer()
                }
                Text("Required to insert dictated text into Slack, browsers, editors etc. If denied, Supertype falls back to clipboard paste (you still need to press Cmd+V).")
                    .font(.caption).foregroundStyle(.secondary)
                HStack {
                    Button("Check Accessibility") { _ = axPermission.isTrusted() }
                    Button("Open System Settings") { axPermission.openSystemSettings() }
                }
                if !axPermission.isGranted {
                    Text("Not granted — System Settings → Privacy & Security → Accessibility → enable Supertype.")
                        .font(.caption).foregroundStyle(.orange)
                }
            }
            Section("Input Monitoring (Global Hotkey)") {
                Text("Global hotkey (e.g. holding fn) uses Event Tap. If the shortcut does not trigger, grant Input Monitoring to Supertype in System Settings → Privacy & Security → Input Monitoring.")
                    .font(.caption).foregroundStyle(.secondary)
                Button("Open Input Monitoring Settings") {
                    if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent") {
                        NSWorkspace.shared.open(url)
                    }
                }
            }
            Section {
                Text("Supertype does not prompt repeatedly. It checks permissions only when you try to record or insert.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }

    // MARK: History

    private var historyTab: some View {
        HistoryView().environmentObject(engine)
    }

    // MARK: Models

    private var modelsTab: some View {
        ModelCatalogView().environmentObject(engine)
    }

    private var dictionaryTab: some View {
        DictionaryView().environmentObject(engine)
    }

    private var privacyTab: some View {
        Form {
            Section("Principles") {
                Text("• Audio stays in memory → VAD → ASR → formatted text → discard. Never persisted.\n• No cloud transcription, no telemetry by default.\n• Production logs never include transcript content.\n• History can be disabled entirely and stores only formatted text.\n• Local models run via Metal/Accelerate on this Mac.")
                    .font(.callout)
                Label("Inference is 100% local", systemImage: "lock.shield.fill").font(.caption).foregroundStyle(.green)
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
                    Text("History entries: \(engine.getHistoryCount())")
                    Spacer()
                    Button("Clear History") { _ = engine.clearHistory() }
                    Button("Reveal DB") { StorageManager.shared.revealInFinder() }
                }
                Text("SQLite lives at ~/Library/Application Support/Supertype/supertype.db")
                    .font(.caption).foregroundStyle(.secondary)
                Text("When history is disabled, no transcript content is written.")
                    .font(.caption).foregroundStyle(.orange)
            }
            Section("Audio Recordings") {
                HStack {
                    Text("Save audio recordings")
                    Spacer()
                    Toggle("", isOn: .constant(false)).labelsHidden().disabled(true)
                }
                Text("Audio is never persisted by default (OFF). Raw PCM stays in memory and is discarded after transcription. This architecture guarantees privacy even without a setting.")
                    .font(.caption).foregroundStyle(.secondary)
                Text("v1 does not implement audio saving; the toggle is for future local-only opt-in.").font(.caption2).foregroundStyle(.secondary)
            }
            Section("Hardware") {
                if let hw = engine.getHardwareInfo() {
                    VStack(alignment: .leading, spacing: 4) {
                        Text("\(hw.arch) · \(hw.cpu_cores) cores · \(hw.memory_gb) GB RAM · \(hw.metal_supported ? "Metal" : "Accelerate")")
                            .font(.caption)
                        if let rec = engine.getRecommendedModels().first {
                            Text("Recommended: \(rec.display_name) (\(rec.size_mb) MB, \(rec.quantization))").font(.caption).foregroundStyle(.green)
                        }
                        if let free = hw.disk_free_gb { Text("Disk free: \(free) GB").font(.caption2).foregroundStyle(.secondary) }
                    }
                } else {
                    Text("Hardware probe unavailable").font(.caption).foregroundStyle(.secondary)
                }
            }
            Section("Local Language Model (Future)") {
                HStack {
                    Text("Enhance with local LLM")
                    Spacer()
                    Toggle("", isOn: .constant(false)).labelsHidden().disabled(true)
                }
                Text("Extension point: TextProcessor → DeterministicFormatter (active) + OptionalLocalLLMProcessor (disabled, no download). Enable only after downloading a small local LLM (not bundled).")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section("Audit") {
                Button("Show Privacy Audit") {
                    if let url = URL(string: "file://\(FileManager.default.currentDirectoryPath)/docs/privacy.md") {
                        NSWorkspace.shared.open(url)
                    }
                }
                Text("Network during normal transcription: none. Only model downloads use HTTPS when you tap Download.")
                    .font(.caption2).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }
}
