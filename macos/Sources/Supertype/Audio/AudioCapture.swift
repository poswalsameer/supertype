import AVFoundation
import Foundation
import CSupertypeCore

/// Native microphone capture using AVAudioEngine.
/// Transport is lock-free: tap callback only does a memcpy into Rust's ring via FFI.
/// No SQLite, no inference, no allocation beyond the AVAudioPCMBuffer supplied by the system.
final class AudioCapture: ObservableObject {
    private var engine: AVAudioEngine?
    private var rustHandle: OpaquePointer?
    private var isCapturing = false
    private var converter: AVAudioConverter?
    private var targetFormat: AVAudioFormat?

    var captureStartTime: Date?

    /// Start capture and bind to a Rust Engine handle.
    /// Must be called from main thread after `engine_start_recording` succeeded.
    func start(rustHandle: OpaquePointer) -> Bool {
        guard !isCapturing else { return true }
        self.rustHandle = rustHandle

        let audioEngine = AVAudioEngine()
        let input = audioEngine.inputNode
        let nativeFormat = input.outputFormat(forBus: 0)
        let nativeRate = nativeFormat.sampleRate
        let nativeChannels = nativeFormat.channelCount

        print("[Supertype] Mic native format: \(nativeRate) Hz, \(nativeChannels)ch, \(nativeFormat)")

        // Target is 16k mono float32 (what Rust expects for push_audio)
        guard let target = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: 16000,
            channels: 1,
            interleaved: false
        ) else {
            print("[Supertype] Failed to create target format")
            return false
        }
        self.targetFormat = target

        // Prepare converter if native != target
        if nativeFormat.sampleRate != 16000 || nativeFormat.channelCount != 1 {
            guard let conv = AVAudioConverter(from: nativeFormat, to: target) else {
                print("[Supertype] Failed to create converter")
                return false
            }
            self.converter = conv
        } else {
            self.converter = nil
        }

        // Install tap — callback must be extremely lightweight
        input.installTap(onBus: 0, bufferSize: 1024, format: nativeFormat) { [weak self] buffer, _ in
            guard let self, let handle = self.rustHandle else { return }
            self.handleBuffer(buffer, native: nativeFormat)
            _ = handle // keep alive
        }

        do {
            try audioEngine.start()
            self.engine = audioEngine
            self.isCapturing = true
            self.captureStartTime = Date()
            print("[Supertype] Audio capture started (native \(nativeRate) → 16k)")
            return true
        } catch {
            print("[Supertype] AudioEngine start failed: \(error)")
            input.removeTap(onBus: 0)
            return false
        }
    }

    func stop() {
        guard isCapturing, let audioEngine = engine else { return }
        audioEngine.inputNode.removeTap(onBus: 0)
        audioEngine.stop()
        self.engine = nil
        self.converter = nil
        self.isCapturing = false
        let dur = captureStartTime.map { Date().timeIntervalSince($0) } ?? 0
        print("[Supertype] Audio capture stopped after \(String(format: "%.2f", dur))s, total pushed")
        self.rustHandle = nil
    }

    var isActive: Bool { isCapturing }

    // MARK: - Buffer handling

    private func handleBuffer(_ buffer: AVAudioPCMBuffer, native: AVAudioFormat) {
        guard let handle = rustHandle else { return }
        let frameCount = Int(buffer.frameLength)
        if frameCount == 0 { return }

        if let converter = converter, let target = targetFormat {
            // Convert via AVAudioConverter (efficient, uses Accelerate internally)
            let ratio = target.sampleRate / native.sampleRate
            let capacity = AVAudioFrameCount(Double(frameCount) * ratio + 16)
            guard let converted = AVAudioPCMBuffer(pcmFormat: target, frameCapacity: capacity) else { return }
            var error: NSError?
            let status = converter.convert(to: converted, error: &error, withInputFrom: { _, outStatus in
                outStatus.pointee = .haveData
                return buffer
            })
            if status == .error, let e = error {
                print("[Supertype] Convert error: \(e)")
                return
            }
            let frames = Int(converted.frameLength)
            if frames == 0 { return }
            if let data = converted.floatChannelData?[0] {
                let ptr = UnsafePointer<Float>(data)
                // Use push_audio (already 16k mono)
                _ = engine_push_audio(handle, ptr, UInt(frames))
            }
        } else {
            // Native already 16k mono — direct push
            if let data = buffer.floatChannelData?[0] {
                let ptr = UnsafePointer<Float>(data)
                _ = engine_push_audio(handle, ptr, UInt(frameCount))
            }
        }
    }
}
