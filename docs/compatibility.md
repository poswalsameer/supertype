# Application Compatibility Matrix

Tested on macOS 14+, Apple Silicon, AX permission granted, Input Monitoring granted for global hotkey.

| App | Insertion Method | Result | Limitations | Fallback |
|-----|------------------|--------|-------------|----------|
| TextEdit | AXSelectedText | ✓ inserts at caret | — | — |
| Notes | AXSelectedText | ✓ | — | — |
| Safari (contentEditable, Google Docs) | AXSelectedText → Clipboard | ✓ | AXWebArea requires AX trust; otherwise clipboard | Cmd+V fallback |
| Chrome | AXSelectedText → Clipboard | ✓ | Same as Safari | ClipboardFallback |
| Slack (Electron) | AXWebArea → Clipboard | ✓ | Electron AX tree shallow; selectedText works when focused | ClipboardFallback |
| Discord (Electron) | Clipboard | ✓ | Needs focus on message input | ClipboardFallback |
| VS Code | AXSelectedText → Clipboard | ✓ | Terminal uses AXTextArea but value replace avoided | ClipboardFallback |
| Cursor | AXSelectedText → Clipboard | ✓ | Same as VS Code | ClipboardFallback |
| Notion (Electron/Web) | Clipboard | ✓ | — | ClipboardFallback |
| Terminal | AXSelectedText | ✓ | Avoids kAXValue overwrite; respects monospace | — |
| Finder (rename) | AXSelectedText | ✓ | — | — |
| Password fields (AXSecureTextField) | — | Blocked | Never injects; copies to clipboard + overlay "Secure field — Copied" | Clipboard (user pastes manually if desired) |

## Insertion Strategy

1. If `AXSecureTextField` → block injection, copy transcript to clipboard, overlay informs.
2. Try `kAXSelectedText` (insert at caret, not replace whole field) — works for most native text fields and many web/Electron inputs when AX trust present.
3. If AX fails or empty-field heuristic matches, fall back to clipboard + `CGEvent` Cmd+V (requires AX for `CGEvent` posting). Clipboard restore after 120 ms on background queue, not main thread.

## Known Limitations

- Generic solution, no per-app hacks. `AXValue` is only used when field is empty to avoid overwriting entire document.
- Clipboard fallback loses rich text but text dictation is plain text.
- If target app crashes or focus changes between hold and release (bundleId mismatch), injection aborts and transcript is copied with overlay "Focus changed — Copied".
- On some keyboards `fn` requires Input Monitoring; user can switch to `ctrl+space` in General if fn does not trigger.

## Test Procedure

1. Grant Microphone + Accessibility + Input Monitoring.
2. Download `whisper-tiny-q4_0` (43 MB).
3. In each app above, hold shortcut, speak "hello comma world period", release, verify "Hello, world." appears at caret within 300 ms after release.
4. Verify rapid press, cancel, and focus-change cases copy to clipboard instead of injecting into wrong app.
