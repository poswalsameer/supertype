// Real bench values from docs/benchmarks/report.md and core/src/bin/bench.rs
// Do not fabricate — these are the stub/metal numbers.
export type BenchRow = { model: string; size: string; quant: string; load: string; rtf: string; note: string };
export const benchRows: BenchRow[] = [
  { model: "Whisper Tiny Q4", size: "43 MB", quant: "Q4_0", load: "~180 ms", rtf: "0.11", note: "Ultra light" },
  { model: "Whisper Tiny", size: "75 MB", quant: "Q5_0", load: "~180 ms", rtf: "0.04", note: "Default" },
  { model: "Whisper Base", size: "142 MB", quant: "Q5_0", load: "~200 ms", rtf: "0.04", note: "Balanced" },
  { model: "Parakeet TDT 0.6B", size: "600 MB", quant: "FP16", load: "~320 ms", rtf: "0.02", note: "Fastest" },
];

export const benchMeta = {
  device: "Apple Silicon · Metal backend · 0.8–3.5s fixtures",
  cmd: "cargo run --bin bench",
  src: "core/src/bin/bench.rs + resources/fixtures/*.wav",
};
