import AppKit
import ApplicationServices

/// Lightweight active-application context (Phase 3).
/// Only minimal metadata required for insertion/history—never captures window content.
struct ActiveApp: Codable, Equatable {
    let bundleId: String?
    let appName: String?
    let isTextEditable: Bool

    static func capture() -> ActiveApp? {
        guard let app = NSWorkspace.shared.frontmostApplication else { return nil }
        let bundleId = app.bundleIdentifier
        let name = app.localizedName
        var isEditable = false
        let pid = app.processIdentifier
        let axApp = AXUIElementCreateApplication(pid)
        var focused: AnyObject?
        if AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
           let element = focused {
            var role: AnyObject?
            if AXUIElementCopyAttributeValue(element as! AXUIElement, kAXRoleAttribute as CFString, &role) == .success,
               let roleStr = role as? String {
                // Include WebArea for Electron/Chrome, plus standard controls
                isEditable = ["AXTextField", "AXTextArea", "AXComboBox", "AXWebArea", "AXGroup"].contains(roleStr)
                if !isEditable {
                    var subrole: AnyObject?
                    if AXUIElementCopyAttributeValue(element as! AXUIElement, kAXSubroleAttribute as CFString, &subrole) == .success,
                       let sub = subrole as? String, sub == "AXSecureTextField" {
                        isEditable = false
                    } else {
                        var value: AnyObject?
                        if AXUIElementCopyAttributeValue(element as! AXUIElement, kAXValueAttribute as CFString, &value) == .success {
                            isEditable = true
                        } else if AXUIElementCopyAttributeValue(element as! AXUIElement, kAXSelectedTextAttribute as CFString, &value) == .success {
                            isEditable = true
                        }
                    }
                }
            }
        } else {
            isEditable = true
        }
        return ActiveApp(bundleId: bundleId, appName: name, isTextEditable: isEditable)
    }

    /// Heuristic: is focused element a password/secure field? If so, do not inject.
    static func isSecureFieldFocused(in app: ActiveApp) -> Bool {
        guard let front = NSWorkspace.shared.frontmostApplication else { return false }
        let axApp = AXUIElementCreateApplication(front.processIdentifier)
        var focused: AnyObject?
        guard AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let element = focused as! AXUIElement? else { return false }
        var subrole: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXSubroleAttribute as CFString, &subrole) == .success,
           let s = subrole as? String, s == "AXSecureTextField" { return true }
        var role: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &role) == .success,
           let r = role as? String, r == "AXSecureTextField" { return true }
        return false
    }
}

final class FocusManager {
    static let shared = FocusManager()
    private(set) var lastCaptured: ActiveApp?

    @discardableResult
    func capture() -> ActiveApp? {
        let app = ActiveApp.capture()
        lastCaptured = app
        // Also inform Rust core for history
        if let captured = app {
            // Bridge via RustEngine singleton in AppDelegate
            // We avoid importing RustBridge here to keep layer isolated; AppDelegate will call setActiveApp
        }
        return app
    }

    func clear() { lastCaptured = nil }
}
