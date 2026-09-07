# Release Process

## Versioning

- `Info.plist` `CFBundleShortVersionString` 0.2.0, `CFBundleVersion` 42 (build number).
- `core/Cargo.toml` `version = 0.2.0` should match; keep in sync.
- `engine_version()` C-ABI returns `CARGO_PKG_VERSION` for About → Diagnostics.

## Build

```bash
source $HOME/.cargo/env
cargo build --release --manifest-path core/Cargo.toml
cargo test --manifest-path core/Cargo.toml
cargo clippy --manifest-path core/Cargo.toml -- -D warnings

swift build --package-path macos -c release
swift build --package-path macos -c debug
DYLD_LIBRARY_PATH=core/target/release macos/.build/release/SupertypeFFITest
DYLD_LIBRARY_PATH=core/target/debug macos/.build/debug/SupertypeFFITest
```

## Signing (requires Apple Developer credentials — not in CI)

Project is release-ready but not notarized in dev environment (no certificates). Remaining external steps:

```bash
# 1. Build xcframework for distribution (removes unsafeFlags for notarization)
# Generate with xcodebuild or lipo:
lipo -create core/target/release/libsupertype_core.a -output SupertypeCore.xcframework

# 2. Xcode archive (open macos/Supertype.xcodeproj if generated via xcodegen)
xcodebuild -project macos/Supertype.xcodeproj -scheme Supertype -configuration Release archive -archivePath build/Supertype.xcarchive

# 3. Sign with hardened runtime
codesign --force --options runtime --entitlements macos/Supertype.entitlements --sign "Developer ID Application: Your Team" build/Supertype.xcarchive/Products/Applications/Supertype.app

# 4. Notarize
xcrun notarytool submit build/Supertype.zip --apple-id ... --team-id ... --wait
xcrun stapler staple build/Supertype.app

# 5. DMG
./scripts/make-dmg.sh
```

`Supertype.entitlements` includes `app-sandbox false`, `device.audio-input`, `automation.apple-events`, `allow-jit`, `allow-unsigned-executable-memory`, `disable-library-validation`.

## Permissions Strings

- `NSMicrophoneUsageDescription`: "Supertype uses the microphone to transcribe your speech locally on-device. Audio is never sent to a server."
- `NSAppleEventsUsageDescription`: "Supertype uses Accessibility to insert transcribed text into the app you are typing in."

## Model Licenses

- `whisper.cpp` MIT — attribution in Speech → Models card.
- `parakeet-tdt-0.6b` CC-BY-4.0 — attribution + link in catalog.

## DMG

`./scripts/make-dmg.sh` creates `dist/Supertype-0.2.0.dmg` via `create-dmg` if installed, otherwise `hdiutil`. Icon at `macos/Resources/AppIcon.icns` (generate from 1024 png via `iconutil`).

## First-Run

Onboarding shows welcome → mic → accessibility → model (size shown) → test. No account. Local inference statement prominent.

## Checklist

- [x] `cargo test` 108, `bench` RTF <1.0, `swift build` release
- [x] `Info.plist` + `entitlements` hardened runtime
- [x] `docs/privacy.md` + `docs/security.md` + `docs/compatibility.md` + `docs/benchmarks/report.md`
- [ ] Signing/notarization (requires certs)
