import AppKit
import ApplicationServices

enum InjectionResult: Equatable {
    case success(method: String)
    case failed(reason: String)
}

/// Dedicated TextInjector abstraction.
/// Tries AX insertion first (selected-range insert, not kAXValue overwrite); falls back to clipboard + paste.
final class TextInjector {
    func canInsertViaAX() -> Bool {
        guard AXIsProcessTrusted() else { return false }
        guard let app = NSWorkspace.shared.frontmostApplication else { return false }
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var focused: AnyObject?
        let err = AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused)
        return err == .success && focused != nil
    }

    @discardableResult
    func insert(_ text: String) -> InjectionResult {
        if text.isEmpty { return .failed(reason: "empty text") }
        // Check secure field first
        if isSecureFieldFocused() { return .failed(reason: "secure field") }
        if let res = tryAXInsert(text), case .success = res { return res }
        return clipboardInsert(text)
    }

    private func isSecureFieldFocused() -> Bool {
        guard let app = NSWorkspace.shared.frontmostApplication else { return false }
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var focused: AnyObject?
        guard AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let el = focused as! AXUIElement? else { return false }
        var subrole: AnyObject?
        if AXUIElementCopyAttributeValue(el, kAXSubroleAttribute as CFString, &subrole) == .success,
           let s = subrole as? String, s == "AXSecureTextField" { return true }
        var role: AnyObject?
        if AXUIElementCopyAttributeValue(el, kAXRoleAttribute as CFString, &role) == .success,
           let r = role as? String, r == "AXSecureTextField" { return true }
        return false
    }

    private func tryAXInsert(_ text: String) -> InjectionResult? {
        guard AXIsProcessTrusted() else { return nil }
        guard let app = NSWorkspace.shared.frontmostApplication else { return nil }
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var focused: AnyObject?
        guard AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let element = focused as! AXUIElement? else { return nil }

        // Prefer selected-text replacement (insert at cursor, not overwrite entire field)
        let axText = text as CFString
        // Try selectedText first — this inserts at caret if selection is empty
        var err = AXUIElementSetAttributeValue(element, kAXSelectedTextAttribute as CFString, axText)
        if err == .success { return .success(method: "AXSelectedText") }

        // Fallback: try selected range + set
        // Some apps expose AXSelectedTextRange; we set it then set selectedText
        // If neither works, fall back to clipboard
        if err != .success {
            // As last resort, try AXValue only if field is empty (heuristic: current value empty)
            var current: AnyObject?
            if AXUIElementCopyAttributeValue(element, kAXValueAttribute as CFString, &current) == .success,
               let curStr = current as? String, curStr.isEmpty {
                err = AXUIElementSetAttributeValue(element, kAXValueAttribute as CFString, axText)
                if err == .success { return .success(method: "AXValue(empty)") }
            }
        }
        if err != .success {
            print("[TextInjector] AX insert failed \(err.rawValue) — fallback to clipboard")
        }
        return nil
    }

    private func clipboardInsert(_ text: String) -> InjectionResult {
        let pasteboard = NSPasteboard.general
        let prevChange = pasteboard.changeCount
        // Preserve only string for simplicity; full fidelity not required for text injection
        let prevString = pasteboard.string(forType: .string)

        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)

        guard let src = CGEventSource(stateID: .hidSystemState) else {
            restoreClipboard(prevString: prevString)
            return .failed(reason: "no event source")
        }
        // Check we can post (requires accessibility)
        if !AXIsProcessTrusted() {
            print("[TextInjector] CGEvent post requires AX trust — may fail")
        }
        let vDown = CGEvent(keyboardEventSource: src, virtualKey: 0x09, keyDown: true)
        vDown?.flags = .maskCommand
        let vUp = CGEvent(keyboardEventSource: src, virtualKey: 0x09, keyDown: false)
        vUp?.flags = .maskCommand
        vDown?.post(tap: .cghidEventTap)
        vUp?.post(tap: .cghidEventTap)

        // Restore on background queue after paste completes — do not block main
        let restoreDelay = 0.12
        DispatchQueue.global(qos: .userInitiated).asyncAfter(deadline: .now() + restoreDelay) {
            DispatchQueue.main.async {
                if pasteboard.changeCount == prevChange + 1 {
                    self.restoreClipboard(prevString: prevString)
                }
            }
        }
        return .success(method: "ClipboardFallback")
    }

    private func restoreClipboard(prevString: String?) {
        let pb = NSPasteboard.general
        // Only restore if our injected text still present (user hasn't copied new)
        // Called after delay, so check again
        if let s = prevString {
            // Only restore if current is our injected text's artifact
            // Simpler: restore previous string unconditionally after delay if still same changeCount handled by caller
            pb.clearContents()
            pb.setString(s, forType: .string)
        } else {
            pb.clearContents()
        }
    }
}
