import Foundation

/// Phase 1 placeholder for global hotkey registration.
/// Hold-to-talk will be implemented in Phase 3 via CGEventTap / Carbon hotkeys.
/// This stub validates the architecture without capturing keys yet.
final class HotkeyManager: ObservableObject {
    @Published var configuredShortcut: String = "fn"
    @Published var isRegistered: Bool = false

    func register(shortcut: String) -> Bool {
        // Phase 3: translate string → keycode+modifiers, register CGEventTap
        configuredShortcut = shortcut
        isRegistered = false // not yet active in Phase 1
        print("[Supertype] HotkeyManager placeholder: would register '\(shortcut)' in Phase 3")
        return true
    }

    func unregister() {
        isRegistered = false
    }
}
