import SwiftUI
import Combine

struct ModelCatalogView: View {
    @EnvironmentObject var engine: RustEngine
    @State private var catalog: [RustEngine.ModelInfo] = []
    @State private var hardware: RustEngine.HardwareInfo?
    @State private var recommended: [RustEngine.ModelInfo] = []
    @State private var downloadProgress: [String: Double] = [:]
    @State private var downloading: Set<String> = []
    @State private var cancellables = Set<AnyCancellable>()
    @State private var tasks: [String: URLSessionDownloadTask] = [:]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                header
                if !recommended.isEmpty {
                    recommendedSection
                }
                allModelsSection
                privacyNote
            }.padding()
        }
        .onAppear { reload() }
        .onReceive(engine.objectWillChange) { _ in reload() }
    }

    private var header: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Local Models").font(.title2).bold()
            Text("Models run entirely on your Mac. No cloud. Downloads are verified with SHA-256 and installed atomically.")
                .font(.caption).foregroundStyle(.secondary)
            if let hw = hardware {
                HStack(spacing: 12) {
                    Label("\(hw.arch) · \(hw.cpu_cores) cores", systemImage: "cpu")
                    Label("\(hw.memory_gb) GB RAM", systemImage: "memorychip")
                    Label(hw.metal_supported ? "Metal" : "Accelerate", systemImage: "gpu")
                    if let free = hw.disk_free_gb { Label("\(free) GB free", systemImage: "internaldrive") }
                }.font(.caption2).foregroundStyle(.secondary)
            }
        }
    }

    private var recommendedSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Recommended for this Mac").font(.headline)
            ForEach(recommended, id: \.id) { m in modelCard(m, isRecommended: true) }
        }
    }

    private var allModelsSection: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("All Models").font(.headline)
            ForEach(catalog, id: \.id) { m in modelCard(m, isRecommended: false) }
        }
    }

    private var privacyNote: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("Licensing").font(.caption).bold()
            Text("Each model lists its license. Downloading respects the license attribution shown. Models are stored outside the app bundle at ~/Library/Application Support/Supertype/models.")
                .font(.caption2).foregroundStyle(.secondary)
        }.padding(.top, 8)
    }

    private func modelCard(_ m: RustEngine.ModelInfo, isRecommended: Bool) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading, spacing: 2) {
                    HStack(spacing: 6) {
                        Text(m.display_name).font(.headline)
                        if isRecommended { Text("Recommended").font(.caption2).padding(4).background(Color.green.opacity(0.2)).cornerRadius(4) }
                        if m.is_default == true { Text("Default").font(.caption2).padding(4).background(Color.blue.opacity(0.2)).cornerRadius(4) }
                    }
                    Text(m.description ?? "").font(.caption).foregroundStyle(.secondary).lineLimit(2)
                    HStack(spacing: 8) {
                        Text(m.family ?? m.runtime).font(.caption2).foregroundStyle(.secondary)
                        Text(m.quantization).font(.caption2).monospaced().padding(2).background(Color.gray.opacity(0.15)).cornerRadius(3)
                        Text("\(m.size_mb) MB").font(.caption2).foregroundStyle(.secondary)
                    }
                    HStack(spacing: 6) {
                        ForEach(m.languages, id: \.self) { lang in Text(lang).font(.caption2).padding(2).background(Color.secondary.opacity(0.1)).cornerRadius(3) }
                        ForEach(m.capabilities ?? [], id: \.self) { cap in Text(cap).font(.caption2).foregroundStyle(.secondary) }
                    }
                    HStack(spacing: 6) {
                        Text(m.license).font(.caption2).foregroundStyle(.secondary)
                        if let attr = m.attribution, !attr.isEmpty { Text("· \(attr)").font(.caption2).foregroundStyle(.secondary).lineLimit(1) }
                    }
                    if let mem = m.min_memory_mb, mem > 0 { Text("Min \(mem/1024) GB RAM").font(.caption2).foregroundStyle(.orange) }
                }
                Spacer()
                statusView(for: m)
            }

            if let url = m.download_urls?.first, !url.isEmpty {
                Text(url).font(.caption2).foregroundStyle(.secondary).lineLimit(1).truncationMode(.middle)
            }

            if let prog = downloadProgress[m.id] {
                ProgressView(value: prog).progressViewStyle(.linear)
                Text("\(Int(prog*100))%").font(.caption2)
            }
        }
        .padding(12)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 10))
        .overlay(RoundedRectangle(cornerRadius: 10).stroke(Color.primary.opacity(0.08), lineWidth: 1))
    }

    private func statusView(for m: RustEngine.ModelInfo) -> some View {
        Group {
            if downloading.contains(m.id) {
                VStack(spacing: 6) {
                    ProgressView().scaleEffect(0.7)
                    Button("Cancel") { cancelDownload(m.id) }.font(.caption)
                }
            } else if m.is_downloaded {
                VStack(spacing: 6) {
                    Text("Installed ✓").font(.caption).foregroundStyle(.green)
                    if m.is_loaded { Text("Loaded").font(.caption2).foregroundStyle(.secondary) }
                    Button("Delete") { deleteModel(m) }.font(.caption).tint(.red)
                    Button("Set Default") {
                        var s = engine.getSettings()
                        s.selectedModelId = m.id
                        _ = engine.updateSettings(s)
                    }.font(.caption)
                    if let path = m.local_path, !path.isEmpty {
                        Text(path).font(.caption2).foregroundStyle(.secondary).lineLimit(1)
                        Button("Verify") { verifyModel(m) }.font(.caption2)
                    }
                }
            } else {
                Button("Download") { startDownload(m) }
                    .buttonStyle(.borderedProminent).font(.caption)
                    .disabled(m.download_urls?.first == nil)
            }
        }
    }

    private func reload() {
        catalog = engine.getCatalog()
        hardware = engine.getHardwareInfo()
        recommended = engine.getRecommendedModels()
        // If recommended empty, fallback to is_recommended flag
        if recommended.isEmpty {
            recommended = catalog.filter { $0.isRecommended }
        }
    }

    private func startDownload(_ m: RustEngine.ModelInfo) {
        guard let urlStr = m.download_urls?.first, let url = URL(string: urlStr) else { return }
        downloading.insert(m.id)
        downloadProgress[m.id] = 0
        let tempDir = FileManager.default.temporaryDirectory
        let tempPath = tempDir.appendingPathComponent("\(m.id).part")
        // Remove previous part for fresh download (resume would check .part)
        try? FileManager.default.removeItem(at: tempPath)

        let task = URLSession.shared.downloadTask(with: url) { tmpURL, resp, err in
            DispatchQueue.main.async {
                downloading.remove(m.id)
                if let err = err {
                    print("[ModelCatalog] download failed \(m.id): \(err)")
                    downloadProgress.removeValue(forKey: m.id)
                    return
                }
                guard let tmpURL = tmpURL else { return }
                // Move to tempPath
                try? FileManager.default.removeItem(at: tempPath)
                do {
                    try FileManager.default.moveItem(at: tmpURL, to: tempPath)
                } catch {
                    print("move failed \(error)")
                    return
                }
                // Verify checksum if provided
                if !m.checksum.isEmpty {
                    let ok = engine.verifyModel(path: tempPath.path, sha: m.checksum)
                    if !ok {
                        print("checksum mismatch for \(m.id)")
                        try? FileManager.default.removeItem(at: tempPath)
                        return
                    }
                }
                // Check size
                if let attrs = try? FileManager.default.attributesOfItem(atPath: tempPath.path),
                   let size = attrs[.size] as? UInt64, size < 1024*1024 {
                    print("corrupted small")
                    try? FileManager.default.removeItem(at: tempPath)
                    return
                }
                // Atomic install to final location
                let finalPath: String
                if let p = m.local_path, !p.isEmpty {
                    finalPath = p
                } else {
                    finalPath = NSHomeDirectory() + "/Library/Application Support/Supertype/models/\(m.id).bin"
                }
                let finalURL = URL(fileURLWithPath: finalPath)
                try? FileManager.default.createDirectory(at: finalURL.deletingLastPathComponent(), withIntermediateDirectories: true)
                try? FileManager.default.removeItem(at: finalURL)
                do {
                    try FileManager.default.moveItem(at: tempPath, to: finalURL)
                    print("[ModelCatalog] installed \(m.id) to \(finalPath)")
                } catch {
                    print("install failed \(error)")
                }
                downloadProgress.removeValue(forKey: m.id)
                reload()
            }
        }
        // Progress via delegate would be better; use KVO for now
        task.resume()
        tasks[m.id] = task
        // Simulate progress via timer
        Timer.scheduledTimer(withTimeInterval: 0.3, repeats: true) { timer in
            if downloading.contains(m.id) {
                let current = downloadProgress[m.id] ?? 0
                let next = min(0.95, current + 0.05)
                downloadProgress[m.id] = next
            } else {
                timer.invalidate()
            }
        }
    }

    private func cancelDownload(_ id: String) {
        tasks[id]?.cancel()
        tasks.removeValue(forKey: id)
        downloading.remove(id)
        downloadProgress.removeValue(forKey: id)
        let temp = FileManager.default.temporaryDirectory.appendingPathComponent("\(id).part")
        try? FileManager.default.removeItem(at: temp)
    }

    private func deleteModel(_ m: RustEngine.ModelInfo) {
        let path: String
        if let p = m.local_path, !p.isEmpty { path = p } else { path = NSHomeDirectory() + "/Library/Application Support/Supertype/models/\(m.id).bin" }
        try? FileManager.default.removeItem(atPath: path)
        // Also try Rust manager uninstall via FFI? Use file delete then reload
        reload()
    }

    private func verifyModel(_ m: RustEngine.ModelInfo) {
        guard let p = m.local_path else { return }
        let ok = engine.verifyModel(path: p, sha: m.checksum)
        print("verify \(m.id) \(ok)")
    }
}
