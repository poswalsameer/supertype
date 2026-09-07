import AppKit
import ApplicationServices

/// Lightweight active-application context (Phase 3).
/// Only minimal metadata required for insertion/history—never captures window content.
struct ActiveApp: Codable, Equatable {
    let bundleId: String?
    let appName: String?
    let isTextEditable: Bool

    static func capture() -> ActiveApp? {
        // Frontmost app metadata only
        guard let app = NSWorkspace.shared.frontmostApplication else { return nil }
        let bundleId = app.bundleIdentifier
        let name = app.localizedName

        // Heuristic: check focused element role via AX (best-effort, no content)
        var isEditable = false
        let pid = app.processIdentifier
        let axApp = AXUIElementCreateApplication(pid)
        var focused: AnyObject?
        if AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
           let element = focused {
            var role: AnyObject?
            if AXUIElementCopyAttributeValue(element as! AXUIElement, kAXRoleAttribute as CFString, &role) == .success,
               let roleStr = role as? String {
                isEditable = ["AXTextField", "AXTextArea", "AXComboBox"].contains(roleStr)
                // Also consider if element supports value
                if !isEditable {
                    var value: AnyObject?
                    if AXUIElementCopyAttributeValue(element as! AXUIElement, kAXValueAttribute as CFString, &value) == .success {
                        isEditable = true
                    }
                }
            }
        } else {
            // If we cannot query AX (no permission), assume editable for permissive UX
            isEditable = true
        }

        return ActiveApp(bundleId: bundleId, appName: name, isTextEditable: isEditable)
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
