import Foundation
import AppKit
import Combine

enum HotkeyMode { case hold, toggle }

/// Dedicated InputController abstraction (Phase 3).
/// Owns global shortcut registration, handles press/hold/release, avoids polling.
final class HotkeyManager: ObservableObject {
    @Published var configuredShortcut: String = "fn"
    @Published var isRegistered: Bool = false
    @Published var lastError: String?

    private var globalMonitors: [Any] = []
    private var localMonitors: [Any] = []
    private var isKeyDown = false
    private var mode: HotkeyMode = .hold
    private var shortcut: HotkeyShortcut?

    var onKeyDown: (() -> Void)?
    var onKeyUp: (() -> Void)?

    deinit { unregister() }

    func setMode(_ mode: HotkeyMode) { self.mode = mode }

    @discardableResult
    func register(shortcut raw: String) -> Bool {
        unregister()
        configuredShortcut = raw
        switch HotkeyShortcut.parse(raw) {
        case .failure(let err):
            lastError = "invalid shortcut: \(err)"
            isRegistered = false
            print("[HotkeyManager] parse failed: \(err)")
            return false
        case .success(let sc):
            shortcut = sc
            if sc.isFn {
                return registerFn()
            } else {
                return registerModifiers(with: sc)
            }
        }
    }

    func unregister() {
        for m in globalMonitors { NSEvent.removeMonitor(m) }
        for m in localMonitors { NSEvent.removeMonitor(m) }
        globalMonitors.removeAll()
        localMonitors.removeAll()
        isRegistered = false
        isKeyDown = false
    }

    // MARK: - Fn (flagsChanged)

    private func registerFn() -> Bool {
        // Fn detection via flagsChanged globally + locally
        let handler: (NSEvent) -> Void = { [weak self] event in self?.handleFn(event: event) }
        if let g = NSEvent.addGlobalMonitorForEvents(matching: [.flagsChanged], handler: handler) {
            globalMonitors.append(g)
        }
        if let l = NSEvent.addLocalMonitorForEvents(matching: [.flagsChanged], handler: { [weak self] e in self?.handleFn(event: e); return e }) {
            localMonitors.append(l)
        }
        isRegistered = !globalMonitors.isEmpty || !localMonitors.isEmpty
        if isRegistered {
            print("[HotkeyManager] registered fn (hold)")
        } else {
            lastError = "failed to register fn"
        }
        return isRegistered
    }

    private func handleFn(event: NSEvent) {
        let fnPressed = event.modifierFlags.contains(.function)
        if fnPressed && !isKeyDown {
            isKeyDown = true
            DispatchQueue.main.async { [weak self] in self?.onKeyDown?() }
        } else if !fnPressed && isKeyDown {
            isKeyDown = false
            DispatchQueue.main.async { [weak self] in self?.onKeyUp?() }
        }
    }

    // MARK: - Modifier+key

    private func registerModifiers(with sc: HotkeyShortcut) -> Bool {
        guard let code = sc.keyCode else { return false }
        let mods = sc.modifiers
        var toggleActive = false

        let handler: (NSEvent) -> Void = { [weak self] event in
            guard let self, let shortcut = self.shortcut else { return }
            if event.type == .keyDown {
                if shortcut.matches(event: event) {
                    if self.mode == .hold {
                        if !self.isKeyDown {
                            self.isKeyDown = true
                            DispatchQueue.main.async { self.onKeyDown?() }
                        }
                    } else {
                        // Toggle: alternate down
                        DispatchQueue.main.async {
                            if !toggleActive {
                                toggleActive = true
                                self.onKeyDown?()
                            } else {
                                toggleActive = false
                                self.onKeyUp?()
                            }
                        }
                    }
                }
            } else if event.type == .keyUp, self.isKeyDown {
                if event.keyCode == code && self.mode == .hold {
                    self.isKeyDown = false
                    DispatchQueue.main.async { self.onKeyUp?() }
                }
            }
        }

        // We need to match both keyDown and keyUp. Use separate monitors for clarity.
        if let gDown = NSEvent.addGlobalMonitorForEvents(matching: [.keyDown], handler: handler) {
            globalMonitors.append(gDown)
        }
        if let gUp = NSEvent.addGlobalMonitorForEvents(matching: [.keyUp], handler: handler) {
            globalMonitors.append(gUp)
        }
        if let l = NSEvent.addLocalMonitorForEvents(matching: [.keyDown, .keyUp], handler: { e in handler(e); return e }) {
            localMonitors.append(l)
        }

        isRegistered = !globalMonitors.isEmpty
        lastError = isRegistered ? nil : "failed to register shortcut (conflict?)"
        if isRegistered { print("[HotkeyManager] registered \(sc.raw) mods \(mods) code \(code)") }
        return isRegistered
    }
}
