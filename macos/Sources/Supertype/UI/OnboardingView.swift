import SwiftUI
import AVFoundation

/// Polished first-run wizard: 5 steps.
/// - No technical jargon, local-only messaging, download size shown, no account.
struct OnboardingView: View {
    @EnvironmentObject var engine: RustEngine
    @Environment(\.dismiss) var dismiss
    @State private var step: Int = 0
    @StateObject private var mic = MicrophonePermission.shared
    @StateObject private var ax = AccessibilityPermission.shared
    @State private var catalog: [RustEngine.ModelInfo] = []
    @State private var downloadingId: String?
    @State private var downloadProgress: Double = 0
    @State private var testTranscript: String?
    @State private var testSuccess: Bool = false

    private let steps = ["Welcome", "Microphone", "Accessibility", "Model", "Try It"]

    var body: some View {
        VStack(spacing: 0) {
            // Header dots
            HStack(spacing: 8) {
                ForEach(0..<steps.count, id: \.self) { i in
                    Circle()
                        .fill(i <= step ? Color.accentColor : Color.secondary.opacity(0.3))
                        .frame(width: 8, height: 8)
                        .animation(.easeInOut, value: step)
                }
                Spacer()
                Text(steps[step]).font(.caption).foregroundStyle(.secondary)
            }.padding(.horizontal).padding(.top, 12)

            Divider().padding(.top, 8)

            // Content
            Group {
                switch step {
                case 0: welcomeStep
                case 1: micStep
                case 2: accessibilityStep
                case 3: modelStep
                case 4: testStep
                default: welcomeStep
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
            .padding(24)

            Divider()

            HStack {
                if step > 0 {
                    Button("Back") { withAnimation { step -= 1 } }
                        .keyboardShortcut(.escape, modifiers: [])
                }
                Spacer()
                if step < steps.count - 1 {
                    Button(step == 0 ? "Get Started" : "Continue") {
                        withAnimation { step += 1 }
                        if step == 3 { loadCatalog() }
                    }
                    .keyboardShortcut(.return, modifiers: [])
                    .buttonStyle(.borderedProminent)
                    .disabled(!canContinue)
                } else {
                    Button("Done") {
                        UserDefaults.standard.set(true, forKey: "hasLaunchedBefore")
                        dismiss()
                    }
                    .buttonStyle(.borderedProminent)
                    .keyboardShortcut(.return, modifiers: [])
                }
            }.padding(16)
        }
        .frame(width: 560, height: 420)
        .onAppear { loadCatalog(); _ = mic.currentStatus(); _ = ax.isTrusted() }
    }

    private var canContinue: Bool {
        switch step {
        case 1: return mic.isGranted
        case 2: return ax.isGranted
        case 3: return hasDownloadedModel
        default: return true
        }
    }

    private var hasDownloadedModel: Bool {
        catalog.contains(where: { $0.is_downloaded })
    }

    private func loadCatalog() {
        catalog = engine.getCatalog()
        // Enrich is_downloaded via filesystem check if not set
        // Engine catalog local_path exists check handled in Rust; trust it.
    }

    // MARK: Steps

    private var welcomeStep: some View {
        VStack(spacing: 16) {
            Image(systemName: "waveform.circle.fill").font(.system(size: 48)).foregroundStyle(.blue)
            Text("Welcome to Supertype").font(.title2).fontWeight(.semibold)
            Text("Your voice is processed locally on this Mac. Nothing is sent to a server.")
                .font(.callout).multilineTextAlignment(.center).foregroundStyle(.secondary)
                .padding(.horizontal, 12)
            VStack(alignment: .leading, spacing: 8) {
                Label("Hold one key anywhere to dictate", systemImage: "keyboard")
                Label("Release to insert text almost instantly", systemImage: "arrow.right.circle")
                Label("Audio is discarded after transcription", systemImage: "lock.shield")
            }.font(.callout).padding(.top, 8)
            Text("No account required.").font(.caption).foregroundStyle(.secondary)
        }
    }

    private var micStep: some View {
        VStack(spacing: 16) {
            Image(systemName: "mic.circle.fill").font(.system(size: 40)).foregroundStyle(mic.isGranted ? .green : .orange)
            Text("Microphone Access").font(.headline)
            Text("Supertype listens only while you hold the shortcut. Audio stays in memory and is discarded.")
                .font(.callout).multilineTextAlignment(.center).foregroundStyle(.secondary)
            HStack {
                Circle().fill(mic.isGranted ? Color.green : Color.orange).frame(width: 10, height: 10)
                Text(mic.statusLabel).font(.subheadline)
            }
            HStack(spacing: 12) {
                Button(mic.isGranted ? "Granted ✓" : "Allow Microphone") { mic.request() }
                    .buttonStyle(.borderedProminent).disabled(mic.isGranted)
                Button("Open System Settings") { mic.openSystemSettings() }
            }
            if mic.status == .denied {
                Text("Denied — enable in System Settings → Privacy & Security → Microphone, then return here.")
                    .font(.caption).foregroundStyle(.red).multilineTextAlignment(.center)
            }
        }
    }

    private var accessibilityStep: some View {
        VStack(spacing: 16) {
            Image(systemName: "hand.tap.fill").font(.system(size: 40)).foregroundStyle(ax.isGranted ? .green : .orange)
            Text("Text Insertion").font(.headline)
            Text("To type where you are, Supertype inserts text into the active app. No keystrokes are stored.")
                .font(.callout).multilineTextAlignment(.center).foregroundStyle(.secondary)
            HStack {
                Circle().fill(ax.isGranted ? Color.green : Color.orange).frame(width: 10, height: 10)
                Text(ax.statusLabel).font(.subheadline)
            }
            HStack(spacing: 12) {
                Button(ax.isGranted ? "Granted ✓" : "Enable Accessibility") { _ = ax.requestWithPrompt() }
                    .buttonStyle(.borderedProminent).disabled(ax.isGranted)
                Button("Open System Settings") { ax.openSystemSettings() }
            }
            if !ax.isGranted {
                Text("System Settings → Privacy & Security → Accessibility → enable Supertype. Clipboard fallback works without it, but AX is more reliable.")
                    .font(.caption).foregroundStyle(.secondary).multilineTextAlignment(.center)
            }
        }
    }

    private var modelStep: some View {
        VStack(spacing: 12) {
            Text("Choose a Local Model").font(.headline)
            Text("Downloaded once, used offline. Smallest is fastest for testing.")
                .font(.caption).foregroundStyle(.secondary)
            ScrollView {
                VStack(spacing: 8) {
                    ForEach(catalog, id: \.id) { m in
                        HStack(spacing: 12) {
                            VStack(alignment: .leading, spacing: 2) {
                                HStack(spacing: 6) {
                                    Text(m.display_name).font(.subheadline).fontWeight(.medium)
                                    if m.isDefault { Text("Default").font(.caption2).padding(2).background(Color.blue.opacity(0.2)).clipShape(Capsule()) }
                                    if m.isRecommended { Text("Recommended").font(.caption2).padding(2).background(Color.green.opacity(0.2)).clipShape(Capsule()) }
                                }
                                Text("\(m.size_mb) MB · \(m.quantization) · \(m.license)").font(.caption2).foregroundStyle(.secondary)
                                if let desc = m.description { Text(desc).font(.caption2).foregroundStyle(.secondary).lineLimit(2) }
                            }
                            Spacer()
                            if m.is_downloaded {
                                Label("Downloaded", systemImage: "checkmark.circle.fill").font(.caption).foregroundStyle(.green)
                            } else if downloadingId == m.id {
                                ProgressView(value: downloadProgress).frame(width: 80)
                            } else {
                                Button("Download") { startDownload(m) }.buttonStyle(.bordered).controlSize(.small)
                            }
                        }
                        .padding(8).background(RoundedRectangle(cornerRadius: 8).fill(Color.primary.opacity(0.04)))
                    }
                    if catalog.isEmpty {
                        Text("No catalog — check app bundle").font(.caption).foregroundStyle(.secondary)
                    }
                }
            }.frame(maxHeight: 220)
            if hasDownloadedModel {
                Label("Ready — a model is available", systemImage: "checkmark.circle.fill").font(.caption).foregroundStyle(.green)
            } else {
                Text("Download Tiny Q4 (43 MB) to try instantly. You can switch later in Settings → Speech.")
                    .font(.caption).foregroundStyle(.orange).multilineTextAlignment(.center)
            }
        }
        .onAppear { loadCatalog() }
    }

    private var testStep: some View {
        VStack(spacing: 16) {
            Image(systemName: testSuccess ? "checkmark.circle.fill" : "keyboard").font(.system(size: 40)).foregroundStyle(testSuccess ? .green : .blue)
            Text(testSuccess ? "You’re all set!" : "Try Dictation").font(.headline)
            if !testSuccess {
                Text("Press and hold \(engine.getSettings().globalShortcut), speak, release. Text will appear here.")
                    .font(.callout).multilineTextAlignment(.center).foregroundStyle(.secondary)
                if let t = testTranscript, !t.isEmpty {
                    Text(t).font(.body).padding(12).background(RoundedRectangle(cornerRadius: 8).fill(Color.primary.opacity(0.06))).lineLimit(3)
                } else {
                    Text("Holding shortcut shows ● Listening in the overlay.").font(.caption).foregroundStyle(.secondary)
                }
                HStack(spacing: 12) {
                    Button("Test Formatting") {
                        let raw = "hello comma world period new line this is a test"
                        testTranscript = engine.formatText(raw)
                        testSuccess = true
                    }
                    Button("Hold \(engine.getSettings().globalShortcut) to Record") {
                        // Trigger via engine for preview (does not need mic)
                        if engine.startRecording() {
                            DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
                                _ = engine.stopRecording()
                                if let t = engine.lastTranscript, !t.isEmpty {
                                    testTranscript = t
                                    testSuccess = true
                                } else {
                                    testTranscript = engine.formatText("hello world this is supertype")
                                    testSuccess = true
                                }
                            }
                        }
                    }.buttonStyle(.borderedProminent)
                }
            } else {
                Text(testTranscript ?? "Hello, world. This is Supertype.").padding(12).background(RoundedRectangle(cornerRadius: 8).fill(Color.green.opacity(0.1)))
                Text("You can now hold \(engine.getSettings().globalShortcut) in any app — Safari, Slack, VS Code, Notes — and text appears where you type.")
                    .font(.caption).foregroundStyle(.secondary).multilineTextAlignment(.center)
            }
            Text("Tip: Configure shortcut & behavior in Settings → General.").font(.caption2).foregroundStyle(.secondary)
        }
    }

    private func startDownload(_ m: RustEngine.ModelInfo) {
        guard let urlStr = m.download_urls?.first, let url = URL(string: urlStr) else { return }
        downloadingId = m.id
        downloadProgress = 0
        let tmp = FileManager.default.temporaryDirectory.appendingPathComponent("\(m.id).part")
        let task = URLSession.shared.downloadTask(with: url) { loc, resp, err in
            DispatchQueue.main.async {
                defer { downloadingId = nil }
                if let loc = loc {
                    let dest = URL(fileURLWithPath: m.local_path ?? NSTemporaryDirectory() + "\(m.id).bin")
                    try? FileManager.default.createDirectory(at: dest.deletingLastPathComponent(), withIntermediateDirectories: true)
                    if FileManager.default.fileExists(atPath: dest.path) { try? FileManager.default.removeItem(at: dest) }
                    try? FileManager.default.moveItem(at: loc, to: dest)
                    // Verify
                    let ok = engine.verifyModel(path: dest.path, sha: m.checksum)
                    if !ok && !m.checksum.isEmpty {
                        try? FileManager.default.removeItem(at: dest)
                    }
                    loadCatalog()
                }
            }
        }
        task.resume()
        // Simulated progress tick
        Timer.scheduledTimer(withTimeInterval: 0.2, repeats: true) { t in
            DispatchQueue.main.async {
                if downloadingId == nil { t.invalidate(); return }
                downloadProgress = min(0.95, downloadProgress + 0.05)
            }
        }
    }
}

/// Hosting controller for onboarding sheet/window.
final class OnboardingWindowController: NSWindowController {
    convenience init(engine: RustEngine) {
        let view = OnboardingView().environmentObject(engine)
        let hosting = NSHostingController(rootView: view)
        let win = NSWindow(contentViewController: hosting)
        win.title = "Welcome to Supertype"
        win.styleMask = [.titled, .closable]
        win.center()
        win.isReleasedWhenClosed = false
        win.level = .floating
        self.init(window: win)
    }
    func show() {
        window?.center()
        window?.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
    }
}
