import SwiftUI

struct DictionaryView: View {
    @EnvironmentObject var engine: RustEngine
    @State private var entries: [(String,String)] = []
    @State private var newPhrase: String = ""
    @State private var newReplacement: String = ""
    @State private var filter: String = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Custom Vocabulary").font(.headline)
            Text("Add company, people, technical terms. Applied locally: raw → vocab → punctuation → final text.")
                .font(.caption).foregroundStyle(.secondary)

            HStack {
                TextField("Phrase (e.g. wisp er)", text: $newPhrase)
                TextField("Replacement (e.g. Wispr)", text: $newReplacement)
                Button("Add") { add() }.disabled(newPhrase.trimmingCharacters(in: .whitespaces).isEmpty || newReplacement.trimmingCharacters(in: .whitespaces).isEmpty)
            }.textFieldStyle(.roundedBorder)

            TextField("Filter", text: $filter).textFieldStyle(.roundedBorder)

            if entries.isEmpty {
                Text("No custom terms yet").foregroundStyle(.secondary).frame(maxWidth: .infinity, alignment: .center).padding()
            } else {
                List {
                    ForEach(filtered, id: \.0) { phrase, repl in
                        HStack {
                            Text(phrase).monospaced()
                            Text("→").foregroundStyle(.secondary)
                            Text(repl)
                            Spacer()
                            Button(role: .destructive) { delete(phrase) } label: { Image(systemName: "trash") }
                                .buttonStyle(.plain)
                        }
                    }
                }.frame(minHeight: 150)
            }

            HStack {
                Button("Test formatting") {
                    let raw = "hello wisp er comma world"
                    let formatted = engine.formatText(raw)
                    print("test format: \(raw) → \(formatted)")
                }
                Spacer()
                Text("\(entries.count) entries").font(.caption).foregroundStyle(.secondary)
            }
        }
        .padding()
        .onAppear { reload() }
        .onReceive(engine.objectWillChange) { _ in reload() }
    }

    private var filtered: [(String,String)] {
        if filter.isEmpty { return entries }
        let q = filter.lowercased()
        return entries.filter { $0.0.lowercased().contains(q) || $0.1.lowercased().contains(q) }
    }

    private func reload() {
        let dict = engine.getDictionary()
        entries = dict.map { ($0.key, $0.value) }.sorted { $0.0 < $1.0 }
    }

    private func add() {
        let p = newPhrase.trimmingCharacters(in: .whitespaces)
        let r = newReplacement.trimmingCharacters(in: .whitespaces)
        guard !p.isEmpty, !r.isEmpty else { return }
        if engine.upsertDictionary(phrase: p, replacement: r) {
            newPhrase = ""; newReplacement = ""
            reload()
        }
    }

    private func delete(_ phrase: String) {
        if engine.deleteDictionary(phrase: phrase) { reload() }
    }
}
