import AppKit
import ApplicationServices

enum InjectionResult: Equatable {
    case success(method: String)
    case failed(reason: String)
}

/// Dedicated TextInjector abstraction.
/// Tries AX insertion first; falls back to clipboard + paste safely.
final class TextInjector {
    /// Whether AX insertion is likely to work (permission + focused element).
    func canInsertViaAX() -> Bool {
        guard AXIsProcessTrusted() else { return false }
        // Check we have a focused element
        guard let app = NSWorkspace.shared.frontmostApplication else { return false }
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var focused: AnyObject?
        let err = AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused)
        return err == .success && focused != nil
    }

    /// Insert `text` into the currently focused app.
    /// Returns success with method used.
    @discardableResult
    func insert(_ text: String) -> InjectionResult {
        if text.isEmpty { return .failed(reason: "empty text") }
        // Try AX first
        if let res = tryAXInsert(text), case .success = res {
            return res
        }
        // Fallback to clipboard
        return clipboardInsert(text)
    }

    private func tryAXInsert(_ text: String) -> InjectionResult? {
        guard AXIsProcessTrusted() else { return nil }
        guard let app = NSWorkspace.shared.frontmostApplication else { return nil }
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var focused: AnyObject?
        guard AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let element = focused as! AXUIElement? else { return nil }

        // Prefer kAXValueAttribute if available (most text fields)
        let axText = text as CFString
        var err = AXUIElementSetAttributeValue(element, kAXValueAttribute as CFString, axText)
        if err == .success { return .success(method: "AXValue") }

        // Try selected text replacement
        err = AXUIElementSetAttributeValue(element, kAXSelectedTextAttribute as CFString, axText)
        if err == .success { return .success(method: "AXSelectedText") }

        // AX failed — will fallback
        print("[TextInjector] AX insert failed: \(err.rawValue)")
        return nil
    }

    private func clipboardInsert(_ text: String) -> InjectionResult {
        let pasteboard = NSPasteboard.general
        let prevChange = pasteboard.changeCount
        let prevString = pasteboard.string(forType: .string)
        let prevItems = pasteboard.pasteboardItems

        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)

        // Synthesize Cmd+V
        guard let src = CGEventSource(stateID: .hidSystemState) else {
            restoreClipboard(prevString: prevString, prevItems: prevItems)
            return .failed(reason: "no event source")
        }
        let vDown = CGEvent(keyboardEventSource: src, virtualKey: 0x09, keyDown: true) // V
        vDown?.flags = .maskCommand
        let vUp = CGEvent(keyboardEventSource: src, virtualKey: 0x09, keyDown: false)
        vUp?.flags = .maskCommand
        vDown?.post(tap: .cghidEventTap)
        vUp?.post(tap: .cghidEventTap)

        // Give system time to process paste before restoring
        Thread.sleep(forTimeInterval: 0.12)

        // Restore previous clipboard safely (if changed since we set)
        if pasteboard.changeCount == prevChange + 1 {
            // Our paste still there — restore old
            restoreClipboard(prevString: prevString, prevItems: prevItems)
        } else {
            // User/app changed clipboard after paste — leave current
        }

        return .success(method: "ClipboardFallback")
    }

    private func restoreClipboard(prevString: String?, prevItems: [NSPasteboardItem]?) {
        let pb = NSPasteboard.general
        pb.clearContents()
        if let s = prevString {
            pb.setString(s, forType: .string)
        } else if let items = prevItems {
            pb.clearContents()
            for item in items {
                // Simplified restore: only string types
                if let str = item.string(forType: .string) {
                    pb.setString(str, forType: .string)
                }
            }
        }
    }
}
