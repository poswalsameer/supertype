import Foundation
import ServiceManagement

struct LaunchAtLoginManager {
    static let shared = LaunchAtLoginManager()

    @discardableResult
    func setEnabled(_ enabled: Bool) -> Bool {
        if #available(macOS 13.0, *) {
            do {
                if enabled { try SMAppService.mainApp.register() }
                else { try SMAppService.mainApp.unregister() }
                return true
            } catch {
                print("[Supertype] SMAppService \(enabled ? "register" : "unregister") failed: \(error)")
                return false
            }
        } else {
            return false
        }
    }

    func isEnabled() -> Bool {
        if #available(macOS 13.0, *) {
            return SMAppService.mainApp.status == .enabled
        }
        return false
    }
}
