import SwiftUI

struct HistoryView: View {
    @EnvironmentObject var engine: RustEngine
    @State private var records: [RustEngine.HistoryRecord] = []
    @State private var searchText: String = ""
    @State private var showCopied: String?

    var body: some View {
        VStack {
            HStack {
                TextField("Search", text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit { reload() }
                Button("Search") { reload() }
                Button("Clear All") { clear() }
                    .disabled(records.isEmpty)
            }.padding(.horizontal)

            if records.isEmpty {
                VStack(spacing: 8) {
                    Text("No history yet").foregroundStyle(.secondary)
                    Text("Completed dictations appear here when history is enabled.")
                        .font(.caption).foregroundStyle(.secondary)
                }.frame(maxHeight: .infinity)
            } else {
                List {
                    ForEach(records) { rec in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(rec.text).lineLimit(2)
                                .contextMenu {
                                    Button("Copy") { copy(rec.text) }
                                    Button("Delete", role: .destructive) { delete(rec) }
                                }
                            HStack {
                                Text(rec.app_name ?? rec.bundle_id ?? "Unknown app")
                                    .font(.caption).foregroundStyle(.secondary)
                                Spacer()
                                Text(rec.created_at).font(.caption2).foregroundStyle(.secondary)
                                Text("\(rec.duration_ms ?? 0)ms").font(.caption2).foregroundStyle(.secondary)
                            }
                        }.padding(.vertical, 2)
                    }
                }
            }

            if let msg = showCopied {
                Text(msg).font(.caption).foregroundStyle(.green).padding(.bottom, 4)
            }
        }
        .onAppear { reload() }
        .onReceive(engine.objectWillChange) { _ in reload() }
    }

    private func reload() {
        if searchText.trimmingCharacters(in: .whitespaces).isEmpty {
            records = engine.getHistory(limit: 100, offset: 0)
        } else {
            records = engine.searchHistory(query: searchText, limit: 100)
        }
    }

    private func delete(_ rec: RustEngine.HistoryRecord) {
        if engine.deleteHistory(id: rec.id) {
            records.removeAll { $0.id == rec.id }
        }
    }

    private func clear() {
        if engine.clearHistory() { records.removeAll() }
    }

    private func copy(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
        showCopied = "Copied"
        DispatchQueue.main.asyncAfter(deadline: .now() + 1) { showCopied = nil }
    }
}
