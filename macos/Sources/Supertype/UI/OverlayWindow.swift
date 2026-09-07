import AppKit
import SwiftUI
import Combine

/// Lightweight floating indicator for hold-to-talk.
/// Behaves like a native overlay: non-activating panel, floats above all spaces,
/// no shadow clutter, minimal CPU (hidden when idle — window not even ordered in).
final class OverlayWindowController {
    private var panel: NSPanel?
    private var hosting: NSHostingView<OverlayView>?
    private var cancellables = Set<AnyCancellable>()
    private var engine: RustEngine?

    func bind(to engine: RustEngine) {
        self.engine = engine
        createPanelIfNeeded()
    }

    private func createPanelIfNeeded() {
        if panel != nil { return }
        let p = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 280, height: 56),
            styleMask: [.nonactivatingPanel, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        p.isOpaque = false
        p.backgroundColor = .clear
        p.hasShadow = true
        p.level = .floating
        p.collectionBehavior = [.canJoinAllSpaces, .stationary, .fullScreenAuxiliary]
        p.isReleasedWhenClosed = false
        p.hidesOnDeactivate = false
        p.ignoresMouseEvents = true
        p.isMovableByWindowBackground = false
        p.titleVisibility = .hidden
        p.titlebarAppearsTransparent = true
        p.standardWindowButton(.closeButton)?.isHidden = true
        p.standardWindowButton(.miniaturizeButton)?.isHidden = true
        p.standardWindowButton(.zoomButton)?.isHidden = true

        let view = OverlayView(state: .idle)
        let hv = NSHostingView(rootView: view)
        hv.translatesAutoresizingMaskIntoConstraints = false
        p.contentView = hv
        p.setContentSize(NSSize(width: 280, height: 56))
        p.center()
        // Start hidden
        p.orderOut(nil)
        self.panel = p
        self.hosting = hv
        // Hide on screen lock/sleep to avoid stale overlay
        NotificationCenter.default.addObserver(forName: NSApplication.didResignActiveNotification, object: nil, queue: .main) { [weak self] _ in
            if self?.engine?.state == .idle { self?.panel?.orderOut(nil) }
        }
        NSWorkspace.shared.notificationCenter.addObserver(forName: NSWorkspace.screensDidSleepNotification, object: nil, queue: .main) { [weak self] _ in
            self?.panel?.orderOut(nil)
        }
    }

    func reflect(state: AppState) {
        guard let panel else { return }
        // Overlay disabled check
        if engine?.getSettings().overlayEnabled == false {
            panel.orderOut(nil)
            return
        }
        switch state {
        case .idle:
            panel.orderOut(nil)
        case .preparing:
            updateView(text: "Preparing…", color: .systemGray)
            show()
        case .recording:
            // If partial transcript exists, show it; otherwise Listening
            if let partial = engine?.partialTranscript, !partial.isEmpty {
                let truncated = String(partial.prefix(40))
                updateView(text: truncated, color: .systemRed, isRecording: true)
            } else {
                updateView(text: "Listening…", color: .systemRed, isRecording: true)
            }
            show()
        case .processing:
            updateView(text: "Processing…", color: .systemOrange)
            show()
        case .completed:
            updateView(text: "✓ Done", color: .systemGreen)
            show()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) { [weak self] in
                if self?.engine?.state == .completed { self?.panel?.orderOut(nil) }
            }
        case .error:
            updateView(text: "Error", color: .systemRed)
            show()
            DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) { [weak self] in
                self?.panel?.orderOut(nil)
            }
        }
    }

    // Phase 3 additions

    func showPartial(_ text: String) {
        guard let panel, engine?.getSettings().overlayEnabled != false else { return }
        let truncated = text.count > 40 ? String(text.prefix(40)) + "…" : text
        updateView(text: truncated, color: .systemRed, isRecording: true)
        show()
    }

    func showSuccess() {
        guard let panel else { return }
        updateView(text: "✓ Done", color: .systemGreen)
        show()
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) { [weak self] in
            self?.panel?.orderOut(nil)
        }
    }

    func showError(_ msg: String) {
        guard let panel else { return }
        // Respect overlayEnabled for errors too (consistency)
        if engine?.getSettings().overlayEnabled == false {
            // Still log but not show
            return
        }
        let truncated = msg.count > 32 ? String(msg.prefix(32)) : msg
        updateView(text: truncated, color: .systemRed)
        show()
        DispatchQueue.main.asyncAfter(deadline: .now() + 1.5) { [weak self] in
            self?.panel?.orderOut(nil)
        }
    }

    private func updateView(text: String, color: NSColor, isRecording: Bool = false) {
        let state = engine?.state ?? .idle
        let v = OverlayView(state: state, text: text, dotColor: Color(nsColor: color), isRecording: isRecording || state == .recording)
        hosting?.rootView = v
        // Widen panel if text long
        let width = max(220, min(420, 40 + CGFloat(text.count) * 8))
        panel?.setContentSize(NSSize(width: width, height: 56))
    }

    private func show() {
        guard let panel else { return }
        if panel.isVisible { return }
        // Position async to avoid blocking main with AX IPC (20-50ms)
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let pos = self?.focusedCursorPosition()
            DispatchQueue.main.async {
                guard let self, let panel = self.panel, !panel.isVisible else { return }
                if let cursorPos = pos {
                    var origin = NSPoint(x: cursorPos.x - 140, y: cursorPos.y + 20)
                    // Clamp to visibleFrame of screen containing point (multi-display)
                    if let screen = NSScreen.screens.first(where: { NSMouseInRect(cursorPos, $0.frame, false) }) ?? NSScreen.main {
                        let frame = screen.visibleFrame
                        origin.x = max(frame.minX + 8, min(origin.x, frame.maxX - (panel.frame.width + 8)))
                        origin.y = max(frame.minY + 8, min(origin.y, frame.maxY - (panel.frame.height + 8)))
                    }
                    panel.setFrameOrigin(origin)
                } else {
                    panel.center()
                }
                // Non-activating show without orderOut flash
                panel.orderFrontRegardless()
            }
        }
    }

    private func focusedCursorPosition() -> NSPoint? {
        guard let app = NSWorkspace.shared.frontmostApplication else { return nil }
        let axApp = AXUIElementCreateApplication(app.processIdentifier)
        var focused: AnyObject?
        guard AXUIElementCopyAttributeValue(axApp, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let element = focused as! AXUIElement? else { return nil }
        var pos: AnyObject?
        var size: AnyObject?
        if AXUIElementCopyAttributeValue(element, kAXPositionAttribute as CFString, &pos) == .success,
           AXUIElementCopyAttributeValue(element, kAXSizeAttribute as CFString, &size) == .success,
           let posVal = pos, let sizeVal = size {
            var position = CGPoint.zero
            var s = CGSize.zero
            AXValueGetValue(posVal as! AXValue, .cgPoint, &position)
            AXValueGetValue(sizeVal as! AXValue, .cgSize, &s)
            return CGPoint(x: position.x + s.width/2, y: position.y - 20)
        }
        return nil
    }
}

struct OverlayView: View {
    var state: AppState
    var text: String?
    var dotColor: Color = .red
    var isRecording: Bool = false

    var body: some View {
        let label: String = text ?? {
            switch state {
            case .idle: return ""
            case .preparing: return "Preparing…"
            case .recording: return "Listening…"
            case .processing: return "Processing…"
            case .completed: return "✓ Done"
            case .error: return "Error"
            }
        }()

        HStack(spacing: 10) {
            Circle().fill(dotColor).frame(width: 10, height: 10)
                .opacity(isRecording ? 1 : 0.9)
                .scaleEffect(isRecording ? 1.1 : 1.0)
                .animation(isRecording ? .easeInOut(duration: 0.6).repeatForever(autoreverses: true) : .default, value: state)
            Text(label).font(.system(.body, design: .rounded)).fontWeight(.medium).lineLimit(1).truncationMode(.tail)
            Spacer(minLength: 0)
            if isRecording {
                // Subtle waveform indication
                HStack(spacing: 2) {
                    ForEach(0..<3) { i in
                        RoundedRectangle(cornerRadius: 1).fill(dotColor.opacity(0.6)).frame(width: 2, height: 8 + CGFloat(i*2))
                            .animation(.easeInOut(duration: 0.4).repeatForever().delay(Double(i)*0.1), value: isRecording)
                    }
                }
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Color.primary.opacity(0.08), lineWidth: 1))
        .shadow(color: .black.opacity(0.18), radius: 12, y: 4)
        .frame(width: 280, height: 56)
    }
}
