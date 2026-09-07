import Foundation
import AppKit

/// Swift-side helper that documents where SQLite lives and verifies it exists.
/// Actual read/write is done by Rust core; this class is for diagnostics/UI.
final class StorageManager: ObservableObject {
    static let shared = StorageManager()

    var dbPath: String { RustEngine.defaultDBPath() }

    var dbExists: Bool { FileManager.default.fileExists(atPath: dbPath) }

    var dbSizeBytes: UInt64 {
        (try? FileManager.default.attributesOfItem(atPath: dbPath)[.size] as? UInt64) ?? 0
    }

    func revealInFinder() {
        NSWorkspace.shared.selectFile(dbPath, inFileViewerRootedAtPath: "")
    }
}
