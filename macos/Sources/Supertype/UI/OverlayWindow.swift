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
            contentRect: NSRect(x: 0, y: 0, width: 220, height: 56),
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
        p.setContentSize(NSSize(width: 220, height: 56))
        p.center()
        // Start hidden
        p.orderOut(nil)
        self.panel = p
        self.hosting = hv
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
            updateView(text: "Listening…", color: .systemRed)
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

    private func updateView(text: String, color: NSColor) {
        // SwiftUI view is immutable through hosting; rebuild for simplicity (cheap).
        let state = engine?.state ?? .idle
        let v = OverlayView(state: state, text: text, dotColor: Color(nsColor: color))
        hosting?.rootView = v
    }

    private func show() {
        guard let panel else { return }
        if !panel.isVisible {
            panel.center()
            panel.orderFrontRegardless()
        }
    }
}

struct OverlayView: View {
    var state: AppState
    var text: String?
    var dotColor: Color = .red

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
                .opacity(state == .recording ? 1 : 0.9)
                .scaleEffect(state == .recording ? 1.1 : 1.0)
                .animation(state == .recording ? .easeInOut(duration: 0.6).repeatForever(autoreverses: true) : .default, value: state)
            Text(label).font(.system(.body, design: .rounded)).fontWeight(.medium)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Color.primary.opacity(0.08), lineWidth: 1))
        .shadow(color: .black.opacity(0.18), radius: 12, y: 4)
        .frame(width: 220, height: 56)
    }
}
