import SwiftUI
import ServiceManagement

struct SettingsView: View {
    @EnvironmentObject var engine: RustEngine
    @StateObject private var micPermission = MicrophonePermission.shared
    @StateObject private var axPermission = AccessibilityPermission.shared
    @State private var launchAtLogin: Bool = false
    @State private var selectedTab: String = "General"

    var body: some View {
        TabView(selection: $selectedTab) {
            generalTab.tabItem { Label("General", systemImage: "gear") }.tag("General")
            permissionsTab.tabItem { Label("Permissions", systemImage: "lock.shield") }.tag("Permissions")
            modelsTab.tabItem { Label("Models", systemImage: "cpu") }.tag("Models")
            privacyTab.tabItem { Label("Privacy", systemImage: "hand.raised") }.tag("Privacy")
        }
        .frame(width: 560, height: 420)
        .padding()
        .onAppear { syncFromEngine() }
        .onReceive(engine.objectWillChange) { _ in syncFromEngine() }
    }

    private func syncFromEngine() {
        let s = engine.getSettings()
        launchAtLogin = s.launchAtLogin
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
                    // Phase 2 will enumerate devices via AVAudioSession
                }
                Text("Phase 1 uses the system default input. Device enumeration lands in Phase 2.")
                    .font(.caption).foregroundStyle(.secondary)
            }

            Section("Hotkey") {
                TextField("Global Shortcut", text: Binding(
                    get: { engine.getSettings().globalShortcut },
                    set: { new in
                        var s = engine.getSettings()
                        s.globalShortcut = new
                        _ = engine.updateSettings(s)
                    }
                ))
                Text("Hold this shortcut to dictate. Global registration arrives in Phase 3.")
                    .font(.caption).foregroundStyle(.secondary)
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
                if let err = engine.lastError {
                    Text(err).font(.caption).foregroundStyle(.red)
                }
            }
        }
        .formStyle(.grouped)
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
            }
            Section("Accessibility") {
                HStack {
                    Circle().fill(axPermission.isGranted ? Color.green : Color.orange).frame(width: 10, height: 10)
                    Text(axPermission.statusLabel).font(.headline)
                    Spacer()
                }
                Text("Required to insert dictated text into other apps (Slack, browsers, editors). Not requested at launch; only when you first dictate.")
                    .font(.caption).foregroundStyle(.secondary)
                HStack {
                    Button("Check Accessibility") { _ = axPermission.isTrusted() }
                    Button("Open System Settings") { axPermission.openSystemSettings() }
                }
            }
            Section {
                Text("If denied, go to System Settings → Privacy & Security → Microphone / Accessibility and enable Supertype, then relaunch.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }

    // MARK: Models

    private var modelsTab: some View {
        Form {
            Section("Selected Model") {
                Picker("Model", selection: Binding(
                    get: { engine.getSettings().selectedModelId },
                    set: { v in var s = engine.getSettings(); s.selectedModelId = v; _ = engine.updateSettings(s) }
                )) {
                    Text("Whisper Tiny (39M) — fastest").tag("whisper-tiny")
                    Text("Whisper Base (74M)").tag("whisper-base")
                    Text("Parakeet TDT 0.6B").tag("parakeet-tdt-0.6b")
                }
                Text("Model catalog and downloads land in Phase 4. Switching here validates persistence and state plumbing.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section("Storage") {
                Text("Models are stored under Application Support and are not bundled with the installer.")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }

    private var privacyTab: some View {
        Form {
            Section("Principles") {
                Text("• Audio stays in memory → VAD → ASR → text → discard. Never persisted.\n• No cloud transcription, no telemetry by default.\n• Production logs never include transcript content.\n• History can be disabled entirely.")
                    .font(.callout)
            }
            Section("Data") {
                Button("Clear Transcription History (Phase 3)") { }
                    .disabled(true)
                Text("SQLite lives at ~/Library/Application Support/Supertype/supertype.db")
                    .font(.caption).foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }
}
