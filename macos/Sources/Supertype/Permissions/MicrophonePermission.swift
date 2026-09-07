import Foundation
import AVFoundation
import AppKit

final class MicrophonePermission: ObservableObject {
    static let shared = MicrophonePermission()
    @Published var status: AVAuthorizationStatus = .notDetermined

    var isGranted: Bool { status == .authorized }
    var statusLabel: String {
        switch status {
        case .authorized: return "Granted"
        case .denied: return "Denied"
        case .notDetermined: return "Not Determined"
        case .restricted: return "Restricted"
        @unknown default: return "Unknown"
        }
    }

    private init() { _ = currentStatus() }

    @discardableResult
    func currentStatus() -> AVAuthorizationStatus {
        let s = AVCaptureDevice.authorizationStatus(for: .audio)
        DispatchQueue.main.async { self.status = s }
        return s
    }

    func request(completion: ((Bool) -> Void)? = nil) {
        AVCaptureDevice.requestAccess(for: .audio) { granted in
            DispatchQueue.main.async {
                self.status = granted ? .authorized : .denied
                completion?(granted)
            }
        }
    }

    func openSystemSettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone") {
            NSWorkspace.shared.open(url)
        }
    }
}
