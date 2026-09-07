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
    private var hotkeyManager: HotkeyManager?
    private var textInjector = TextInjector()
    private var lastActiveApp: ActiveApp?

    private var onboardingController: OnboardingWindowController?
    private var sleepObserver: Any?
    private var didRegisterSleepObservers = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.accessory)

        // Initialize Rust core
        let dbPath = RustEngine.defaultDBPath()
        let ok = engine.initialize(dbPath: dbPath)
        if ok {
            print("[Supertype] Engine initialized at \(dbPath) state=\(engine.state)")
        } else {
            print("[Supertype] Engine initialize failed: \(engine.lastError ?? "unknown")")
        }

        setupMenuBar()
        setupOverlay()
        setupHotkey()
        observeEngine()
        observeSettings()
        setupSleepObservers()

        // Warm permissions without prompting
        _ = MicrophonePermission.shared.currentStatus()
        _ = AccessibilityPermission.shared.isTrusted()

        print("[Supertype] Launched. Menu bar ready. Hold \(engine.getSettings().globalShortcut) to dictate.")

        // First-run onboarding: show if never launched or missing model/permissions
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) { [weak self] in self?.maybeShowOnboarding() }
    }

    func applicationWillTerminate(_ notification: Notification) {
        hotkeyManager?.unregister()
        audioCapture?.stop()
        engine.shutdown()
        if let obs = sleepObserver { NSWorkspace.shared.notificationCenter.removeObserver(obs) }
        if let obs2 = sleepObserver { NotificationCenter.default.removeObserver(obs2) }
        print("[Supertype] Terminated cleanly.")
    }

    // MARK: Onboarding

    private func maybeShowOnboarding() {
        let hasLaunched = UserDefaults.standard.bool(forKey: "hasLaunchedBefore")
        let catalog = engine.getCatalog()
        let hasModel = catalog.contains(where: { $0.is_downloaded })
        let micOK = MicrophonePermission.shared.currentStatus() == .authorized
        let axOK = AccessibilityPermission.shared.isTrusted()
        if !hasLaunched || !hasModel || !micOK || !axOK {
            showOnboarding()
        }
    }

    private func showOnboarding() {
        if onboardingController == nil {
            onboardingController = OnboardingWindowController(engine: engine)
        }
        onboardingController?.show()
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

        let shortcutItem = NSMenuItem(title: "Shortcut: \(engine.getSettings().globalShortcut)", action: nil, keyEquivalent: "")
        shortcutItem.tag = 101
        shortcutItem.isEnabled = false
        menu.addItem(shortcutItem)

        menu.addItem(.separator())
        menu.addItem(NSMenuItem(title: "Settings…", action: #selector(openSettings), keyEquivalent: ","))
        menu.addItem(NSMenuItem(title: "Onboarding…", action: #selector(showOnboardingAction), keyEquivalent: ""))
        menu.addItem(NSMenuItem(title: "Permissions…", action: #selector(openPermissions), keyEquivalent: ""))
        menu.addItem(.separator())
        let lastItem = NSMenuItem(title: "Copy Last Transcript", action: #selector(copyLast), keyEquivalent: "")
        lastItem.tag = 102
        menu.addItem(lastItem)
        menu.addItem(NSMenuItem(title: "Show History…", action: #selector(showHistory), keyEquivalent: ""))
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

        NotificationCenter.default.publisher(for: .engineEvent).sink { [weak self] note in
            guard let self, let ev = note.object as? EngineEvent else { return }
            DispatchQueue.main.async { self.handleEngineEvent(ev) }
        }.store(in: &cancellables)
    }

    private func observeSettings() {
        // Re-register hotkey when shortcut changes
        engine.objectWillChange.sink { [weak self] _ in
            DispatchQueue.main.async { self?.syncHotkeyFromSettings() }
        }.store(in: &cancellables)
    }

    private func syncHotkeyFromSettings() {
        let settings = engine.getSettings()
        let shortcut = settings.globalShortcut
        let mode: HotkeyMode = settings.shortcutBehavior == "toggle" ? .toggle : .hold
        hotkeyManager?.setMode(mode)
        if hotkeyManager?.configuredShortcut != shortcut {
            let ok = hotkeyManager?.register(shortcut: shortcut) ?? false
            if !ok, let err = hotkeyManager?.lastError {
                print("[Supertype] Hotkey register failed: \(err)")
                showTransientError("Shortcut failed: \(err)")
            }
            updateMenuState()
        }
    }

    private func updateMenuState() {
        guard let menu = statusItem?.menu else { return }
        if let item = menu.items.first(where: { $0.tag == 100 }) {
            item.title = "State: \(engine.state.displayName)"
            if let err = engine.lastError, engine.state == .error {
                item.title += " — \(err)"
            }
            if let text = engine.lastTranscript, !text.isEmpty, engine.state == .completed {
                item.toolTip = text
            }
        }
        if let item = menu.items.first(where: { $0.tag == 101 }) {
            item.title = "Shortcut: \(engine.getSettings().globalShortcut) \(hotkeyManager?.isRegistered == true ? "✓" : "⚠")"
            if let err = hotkeyManager?.lastError {
                item.toolTip = err
            }
        }
        if let item = menu.items.first(where: { $0.tag == 102 }) {
            item.isEnabled = (engine.lastTranscript != nil && !(engine.lastTranscript?.isEmpty ?? true))
        }
    }

    // MARK: Hotkey

    private func setupHotkey() {
        hotkeyManager = HotkeyManager()
        hotkeyManager?.onKeyDown = { [weak self] in self?.handleHotkeyDown() }
        hotkeyManager?.onKeyUp = { [weak self] in self?.handleHotkeyUp() }
        let settings = engine.getSettings()
        hotkeyManager?.setMode(settings.shortcutBehavior == "toggle" ? .toggle : .hold)
        let ok = hotkeyManager?.register(shortcut: settings.globalShortcut) ?? false
        if !ok {
            print("[Supertype] Hotkey '\(settings.globalShortcut)' registration failed: \(hotkeyManager?.lastError ?? "unknown")")
        } else {
            print("[Supertype] Hotkey registered: \(settings.globalShortcut) mode=\(settings.shortcutBehavior)")
        }
    }

    private func handleHotkeyDown() {
        // Model-required gate: if no model downloaded, guide to onboarding/models
        let catalog = engine.getCatalog()
        if !catalog.contains(where: { $0.is_downloaded }) {
            showTransientError("Download a model in Settings → Speech")
            showOnboarding()
            return
        }
        // Debounce rapid presses (<120ms)
        if let last = lastHotkeyDown, Date().timeIntervalSince(last) < 0.12 { return }
        lastHotkeyDown = Date()
        // Capture active app before overlay (overlay is non-activating)
        let active = ActiveApp.capture()
        // Secure field guard: never inject into password fields
        if let app = active, ActiveApp.isSecureFieldFocused(in: app) {
            showTransientError("Secure field — not injecting")
            copyToClipboardFallback()
            return
        }
        lastActiveApp = active
        engine.setActiveApp(bundleId: active?.bundleId, appName: active?.appName)
        print("[Supertype] Hotkey down — active: \(active?.appName ?? "unknown") \(active?.bundleId ?? "")")

        // Check mic permission before starting
        let micStatus = MicrophonePermission.shared.currentStatus()
        if micStatus != .authorized {
            showTransientError("Mic denied — enable in Settings")
            MicrophonePermission.shared.request { granted in
                DispatchQueue.main.async {
                    if granted { _ = self.engine.startRecording() }
                }
            }
            return
        }
        _ = engine.startRecording()
    }
    private var lastHotkeyDown: Date?

    private func handleHotkeyUp() {
        if engine.state == .recording {
            _ = engine.stopRecording()
        }
    }

    private func handleEngineEvent(_ ev: EngineEvent) {
        switch ev {
        case .partialTranscript(let t):
            // Redact transcript in production logs — only length
            print("[Supertype] partial len=\(t.count)")
            // Filter dummy in production (tests expect it, but prod should not inject placeholder)
            if t.contains("[Dummy") { return }
            overlay?.showPartial(t)
        case .finalTranscript(let t):
            // Drop dummy placeholder in production — treat as missing model error
            if t.contains("[Dummy") || t.trimmingCharacters(in: .whitespaces).isEmpty {
                print("[Supertype] dummy/empty transcript — model missing")
                showTransientError("Model not ready — download in Settings → Speech")
                _ = engine.cancelRecording()
                return
            }
            print("[Supertype] final len=\(t.count) app=\(lastActiveApp?.bundleId ?? "")")
            DispatchQueue.global(qos: .userInitiated).async { [weak self] in
                DispatchQueue.main.async { self?.insertTranscript(t) }
            }
        case .error(let msg):
            print("[Supertype] error: \(msg)")
            showTransientError(msg)
            if msg.contains("corrupted") || msg.contains("Model") || msg.contains("disk full") {
                showTransientError("Model unavailable — download in Settings")
            }
            if msg.lowercased().contains("disk") || msg.lowercased().contains("no space") {
                showTransientError("Disk full — free space")
            }
        default: break
        }
    }

    private func insertTranscript(_ text: String) {
        guard !text.isEmpty else { return }
        // Secure field re-check before inject
        if let curApp = ActiveApp.capture(), ActiveApp.isSecureFieldFocused(in: curApp) {
            print("[Supertype] Secure field at inject — abort, copy only")
            overlay?.showError("Secure field — Copied")
            copyToClipboard(text)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) { [weak self] in _ = self?.engine.acknowledge() }
            return
        }
        // Check if focused app still matches captured (avoid injecting into wrong app if focus changed)
        let current = ActiveApp.capture()
        if let last = lastActiveApp, let cur = current {
            if last.bundleId != cur.bundleId {
                print("[Supertype] Focus changed from \(last.bundleId ?? "") to \(cur.bundleId ?? "") — abort injection, keep transcript for copy")
                overlay?.showError("Focus changed — Copied")
                copyToClipboard(text)
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) { [weak self] in _ = self?.engine.acknowledge() }
                return
            }
            // Also check if target app crashed (no frontmost)
            if cur.bundleId == nil {
                overlay?.showError("Target lost — Copied")
                copyToClipboard(text)
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) { [weak self] in _ = self?.engine.acknowledge() }
                return
            }
        }

        // Check AX permission
        if !AccessibilityPermission.shared.isTrusted() {
            print("[Supertype] AX not trusted — using clipboard fallback")
        }

        let result = textInjector.insert(text)
        switch result {
        case .success(let method):
            print("[Supertype] Inserted via \(method)")
            overlay?.showSuccess()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) { [weak self] in _ = self?.engine.acknowledge() }
        case .failed(let reason):
            print("[Supertype] Injection failed: \(reason) — preserve for copy")
            overlay?.showError("Insert failed")
            copyToClipboard(text)
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) { [weak self] in _ = self?.engine.acknowledge() }
        }
    }
    private func copyToClipboardFallback() {
        if let t = engine.lastTranscript { copyToClipboard(t) }
    }

    private func copyToClipboard(_ text: String) {
        let pb = NSPasteboard.general
        pb.clearContents()
        pb.setString(text, forType: .string)
    }

    private func showTransientError(_ msg: String) {
        overlay?.showError(msg)
    }

    // MARK: Overlay

    private func setupOverlay() {
        overlay = OverlayWindowController()
        overlay?.bind(to: engine)
    }

    private func updateOverlay() {
        overlay?.reflect(state: engine.state)
    }

    private func syncAudioCapture() {
        switch engine.state {
        case .recording:
            if audioCapture == nil {
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
                                self.showTransientError("Mic denied")
                            }
                        }
                    }
                    return
                }
                if let h = engine.rustHandle {
                    let cap = AudioCapture()
                    if cap.start(rustHandle: h) {
                        audioCapture = cap
                    } else {
                        print("[Supertype] Audio capture failed")
                        showTransientError("Mic start failed")
                        _ = engine.cancelRecording()
                    }
                }
            }
        default:
            if audioCapture != nil {
                audioCapture?.stop()
                audioCapture = nil
                if let m = engine.getMetrics() {
                    print("[Supertype] metrics: \(m)")
                }
            }
        }
    }

    // MARK: Sleep/Wake

    private func setupSleepObservers() {
        guard !didRegisterSleepObservers else { return }
        didRegisterSleepObservers = true
        NSWorkspace.shared.notificationCenter.addObserver(self, selector: #selector(handleSleep), name: NSWorkspace.screensDidSleepNotification, object: nil)
        NSWorkspace.shared.notificationCenter.addObserver(self, selector: #selector(handleWake), name: NSWorkspace.screensDidWakeNotification, object: nil)
        NSWorkspace.shared.notificationCenter.addObserver(self, selector: #selector(handleSleep), name: NSWorkspace.willSleepNotification, object: nil)
        NSWorkspace.shared.notificationCenter.addObserver(self, selector: #selector(handleWake), name: NSWorkspace.didWakeNotification, object: nil)
    }

    @objc private func handleSleep() {
        if engine.state == .recording || engine.state == .processing {
            print("[Supertype] Sleep — cancel recording")
            _ = engine.cancelRecording()
            audioCapture?.stop()
            audioCapture = nil
        }
    }

    @objc private func handleWake() {
        // Re-register hotkey after wake (CGEventTap may be invalidated)
        syncHotkeyFromSettings()
        print("[Supertype] Wake — hotkey re-registered")
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

    @objc func openPermissions() { openSettings() }

    @objc func showOnboardingAction() { showOnboarding() }

    @objc func copyLast() {
        if let t = engine.lastTranscript {
            copyToClipboard(t)
            overlay?.showSuccess()
        } else if let t = engine.partialTranscript {
            copyToClipboard(t)
        }
    }

    @objc func showHistory() { openSettings() }

    @objc func testStart() { handleHotkeyDown() }
    @objc func testStop() { handleHotkeyUp() }
    @objc func testCancel() { _ = engine.cancelRecording() }

    @objc func quit() { NSApp.terminate(nil) }
}
