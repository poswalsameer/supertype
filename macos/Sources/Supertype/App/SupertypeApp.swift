import SwiftUI
import AppKit
import Combine

@main
struct SupertypeApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        Settings {
            SettingsView()
                .environmentObject(appDelegate.engine)
        }
    }
}

final class AppDelegate: NSObject, NSApplicationDelegate, ObservableObject {
    var statusItem: NSStatusItem?
    var settingsWindowController: NSWindowController?
    let engine = RustEngine()
    private var cancellables = Set<AnyCancellable>()
    private var overlay: OverlayWindowController?
    private var audioCapture: AudioCapture?

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)

        // Initialize Rust core with persistent DB
        let dbPath = RustEngine.defaultDBPath()
        let ok = engine.initialize(dbPath: dbPath)
        if ok {
            print("[Supertype] Engine initialized at \(dbPath) state=\(engine.state)")
        } else {
            print("[Supertype] Engine initialize failed: \(engine.lastError ?? "unknown")")
        }

        setupMenuBar()
        setupOverlay()
        observeEngine()

        // Warm permissions state (no prompt until needed)
        _ = MicrophonePermission.shared.currentStatus()
        _ = AccessibilityPermission.shared.isTrusted()

        print("[Supertype] Launched. Menu bar ready. Idle resource usage minimal.")
    }

    func applicationWillTerminate(_ notification: Notification) {
        engine.shutdown()
        print("[Supertype] Terminated cleanly.")
    }

    func applicationSupportsSecureRestorableState(_ app: NSApplication) -> Bool { true }

    // MARK: Menu Bar

    private func setupMenuBar() {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        guard let button = statusItem?.button else { return }
        button.image = NSImage(systemSymbolName: "waveform", accessibilityDescription: "Supertype")
        button.image?.isTemplate = true
        button.toolTip = "Supertype — Hold shortcut to dictate"

        let menu = NSMenu()
        menu.addItem(NSMenuItem(title: "Supertype", action: nil, keyEquivalent: ""))
        menu.items[0].isEnabled = false
        menu.addItem(.separator())

        let stateItem = NSMenuItem(title: "State: \(engine.state.displayName)", action: nil, keyEquivalent: "")
        stateItem.tag = 100
        stateItem.isEnabled = false
        menu.addItem(stateItem)

        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Settings…", action: #selector(openSettings), keyEquivalent: ","))
        menu.addItem(NSMenuItem(title: "Permissions…", action: #selector(openPermissions), keyEquivalent: ""))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Start Recording (Test)", action: #selector(testStart), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Stop Recording (Test)", action: #selector(testStop), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Cancel", action: #selector(testCancel), keyEquivalent: ""))
        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Quit Supertype", action: #selector(quit), keyEquivalent: "q"))

        statusItem?.menu = menu
    }

    private func observeEngine() {
        engine.objectWillChange.sink { [weak self] _ in
            DispatchQueue.main.async { self?.updateMenuState(); self?.updateOverlay() }
        }.store(in: &cancellables)

        NotificationCenter.default.publisher(for: .engineStateChanged).sink { [weak self] _ in
            DispatchQueue.main.async {
                self?.updateOverlay()
                self?.syncAudioCapture()
            }
        }.store(in: &cancellables)

        // Also poll transcript events for menu/log
        NotificationCenter.default.publisher(for: .engineEvent).sink { note in
            if let ev = note.object as? EngineEvent {
                switch ev {
                case .partialTranscript(let t): print("[Supertype] partial: \(t)")
                case .finalTranscript(let t): print("[Supertype] final: \(t)")
                default: break
                }
            }
        }.store(in: &cancellables)
    }

    private func syncAudioCapture() {
        switch engine.state {
        case .recording:
            if audioCapture == nil {
                // Check mic permission first
                let status = MicrophonePermission.shared.currentStatus()
                if status != .authorized {
                    print("[Supertype] Mic not authorized (\(status.rawValue)), requesting…")
                    MicrophonePermission.shared.request { granted in
                        DispatchQueue.main.async {
                            if granted, let h = self.engine.rustHandle {
                                let cap = AudioCapture()
                                if cap.start(rustHandle: h) {
                                    self.audioCapture = cap
                                }
                            } else if !granted {
                                print("[Supertype] Mic denied — cannot capture")
                                _ = self.engine.cancelRecording()
                            }
                        }
                    }
                    return
                }
                if let h = engine.rustHandle {
                    let cap = AudioCapture()
                    if cap.start(rustHandle: h) {
                        audioCapture = cap
                    }
                }
            }
        default:
            if audioCapture != nil {
                audioCapture?.stop()
                audioCapture = nil
                // Metrics after stop
                if let m = engine.getMetrics() {
                    print("[Supertype] metrics: \(m)")
                }
                if let t = engine.getLastTranscript() {
                    print("[Supertype] last transcript: \(t)")
                }
            }
        }
    }

    private func updateMenuState() {
        guard let menu = statusItem?.menu, let item = menu.items.first(where: { $0.tag == 100 }) else { return }
        item.title = "State: \(engine.state.displayName)"
        if let err = engine.lastError, engine.state == .error {
            item.title += " — \(err)"
        }
    }

    // MARK: Overlay

    private func setupOverlay() {
        overlay = OverlayWindowController()
        overlay?.bind(to: engine)
    }

    private func updateOverlay() {
        overlay?.reflect(state: engine.state)
    }

    // MARK: Actions

    @objc func openSettings() {
        if #available(macOS 14.0, *) {
            NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
        } else {
            NSApp.sendAction(Selector(("showPreferencesWindow:")), to: nil, from: nil)
        }
        NSApp.activate(ignoringOtherApps: true)
    }

    @objc func openPermissions() {
        openSettings()
        // SettingsView will show Permissions tab; for now just open settings.
    }

    @objc func testStart() { _ = engine.startRecording() }
    @objc func testStop() { _ = engine.stopRecording() }
    @objc func testCancel() { _ = engine.cancelRecording() }

    @objc func quit() { NSApp.terminate(nil) }
}
