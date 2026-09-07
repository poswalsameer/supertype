import Foundation
import AppKit
import ApplicationServices

final class AccessibilityPermission: ObservableObject {
    static let shared = AccessibilityPermission()
    @Published private(set) var trusted: Bool = false

    var isGranted: Bool { trusted }
    var statusLabel: String { trusted ? "Granted" : "Not Granted" }

    private init() { _ = isTrusted() }

    /// Check without prompting. Call `requestWithPrompt()` to prompt.
    @discardableResult
    func isTrusted() -> Bool {
        let t = AXIsProcessTrusted()
        DispatchQueue.main.async { self.trusted = t }
        return t
    }

    /// Prompt the user once (shows system dialog). Do not call at every launch.
    @discardableResult
    func requestWithPrompt() -> Bool {
        let opts = [kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String: true] as CFDictionary
        let t = AXIsProcessTrustedWithOptions(opts)
        DispatchQueue.main.async { self.trusted = t }
        return t
    }

    func openSystemSettings() {
        // Deep link to Accessibility
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }
        // Fallback: prompt via API if not yet trusted
        if !trusted { _ = requestWithPrompt() }
    }
}
