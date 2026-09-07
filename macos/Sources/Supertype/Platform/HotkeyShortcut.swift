import AppKit

extension String: @retroactive Error {}

struct HotkeyShortcut: Equatable {
    let raw: String
    let keyCode: UInt16? // nil for pure modifier like "fn"
    let modifiers: NSEvent.ModifierFlags
    let isFn: Bool

    static func parse(_ raw: String) -> Result<HotkeyShortcut, String> {
        let trimmed = raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if trimmed.isEmpty { return .failure("empty shortcut") }
        if trimmed == "fn" {
            return .success(HotkeyShortcut(raw: raw, keyCode: nil, modifiers: [], isFn: true))
        }
        // Split by "+"
        let parts = trimmed.split(separator: "+").map { $0.trimmingCharacters(in: .whitespaces) }
        var mods: NSEvent.ModifierFlags = []
        var keyPart: String?
        for p in parts {
            switch p {
            case "ctrl", "control", "ctl": mods.insert(.control)
            case "cmd", "command", "meta": mods.insert(.command)
            case "option", "opt", "alt": mods.insert(.option)
            case "shift": mods.insert(.shift)
            case "fn", "function": return .failure("fn must be used alone (e.g. 'fn')")
            default: 
                if keyPart != nil { return .failure("multiple keys: \(raw)") }
                keyPart = p
            }
        }
        guard let key = keyPart else { return .failure("no key in \(raw)") }
        let code: UInt16
        switch key {
        case "space", " ": code = 49
        case "a": code = 0
        case "b": code = 11
        case "c": code = 8
        case "d": code = 2
        case "e": code = 14
        case "f": code = 3
        case "g": code = 5
        case "h": code = 4
        case "i": code = 34
        case "j": code = 38
        case "k": code = 40
        case "l": code = 37
        case "m": code = 46
        case "n": code = 45
        case "o": code = 31
        case "p": code = 35
        case "q": code = 12
        case "r": code = 15
        case "s": code = 1
        case "t": code = 17
        case "u": code = 32
        case "v": code = 9
        case "w": code = 13
        case "x": code = 7
        case "y": code = 16
        case "z": code = 6
        case "0": code = 29
        case "1": code = 18
        case "2": code = 19
        case "3": code = 20
        case "4": code = 21
        case "5": code = 23
        case "6": code = 22
        case "7": code = 26
        case "8": code = 28
        case "9": code = 25
        case "f1": code = 122
        case "f2": code = 120
        case "f3": code = 99
        case "f4": code = 118
        default: return .failure("unsupported key: \(key)")
        }
        return .success(HotkeyShortcut(raw: raw, keyCode: code, modifiers: mods, isFn: false))
    }

    var displayString: String { raw }

    func matches(event: NSEvent) -> Bool {
        if isFn {
            // fn is detected via flagsChanged; handled elsewhere
            return false
        }
        guard let code = keyCode else { return false }
        // Check keyCode and modifiers (ignore capsLock, numericPad)
        let eventMods = event.modifierFlags.intersection([.command, .control, .option, .shift])
        return event.keyCode == code && eventMods == modifiers
    }
}
