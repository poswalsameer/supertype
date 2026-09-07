import { copy } from "@/lib/copy";
import { benchRows, benchMeta } from "@/lib/benchmarks";

export function PerformanceSection() {
  return (
    <section className="container-1120 py-14 md:py-20">
      <div className="grid lg:grid-cols-[520px_1fr] gap-10 items-start">
        <div>
          <div className="text-[11px] font-medium tracking-[0.12em] uppercase text-[var(--muted-2)]">{copy.performance.eyebrow}</div>
          <h2 className="mt-2 text-[28px] md:text-[34px] font-semibold leading-[1.05] tracking-[-0.03em] whitespace-pre-line">{copy.performance.title}</h2>
          <p className="mt-3 max-w-[48ch] text-[15px] leading-7 text-[var(--muted)]">{copy.performance.sub}</p>
          <ul className="mt-6 space-y-2">
            {copy.performance.principles.map((p) => (
              <li key={p} className="flex gap-2 text-[13px] text-[var(--muted)]">
                <span className="mt-2 size-1 rounded-full bg-[var(--fg)] shrink-0" /> {p}
              </li>
            ))}
          </ul>
          <div className="mt-6 rounded-[12px] border border-[var(--border)] bg-[var(--bg-2)] px-4 py-3">
            <div className="text-[11px] font-medium tracking-wide uppercase text-[var(--muted-2)]">Why it feels fast</div>
            <p className="mt-1 text-[13px] leading-6 text-[var(--muted)]">1024-frame taps, bounded ring, off-lock transcribe, 1s streaming partials, minimal Swift↔Rust copies, overlay that never steals focus.</p>
          </div>
        </div>

        <div className="rounded-[16px] border border-[var(--border)] bg-white overflow-hidden">
          <div className="px-5 py-4 border-b border-[var(--border)] bg-[var(--bg-2)] flex items-center justify-between">
            <div className="text-[12px] font-medium tracking-[-0.01em]">Local benchmark · Metal</div>
            <div className="text-[11px] font-mono text-[var(--muted-2)]">{benchMeta.device}</div>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-left">
              <thead>
                <tr className="text-[11px] font-medium tracking-wide uppercase text-[var(--muted-2)] border-b border-[var(--border)]">
                  <th className="px-4 py-2.5 font-medium">Model</th>
                  <th className="px-3 py-2.5 font-medium">Size</th>
                  <th className="px-3 py-2.5 font-medium">RTF</th>
                  <th className="px-3 py-2.5 font-medium">Load</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-[var(--border)]">
                {benchRows.map((r) => (
                  <tr key={r.model} className="hover:bg-[var(--bg-2)]/50">
                    <td className="px-4 py-3">
                      <div className="text-[13px] font-medium tracking-[-0.01em]">{r.model}</div>
                      <div className="text-[11px] text-[var(--muted-2)]">{r.note} · {r.quant}</div>
                    </td>
                    <td className="px-3 py-3 font-mono text-[12px]">{r.size}</td>
                    <td className="px-3 py-3 font-mono text-[12px] font-medium">{r.rtf}</td>
                    <td className="px-3 py-3 font-mono text-[12px] text-[var(--muted)]">{r.load}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="px-5 py-3 border-t border-[var(--border)] bg-[var(--bg-2)] flex flex-wrap gap-3 text-[11px] font-mono text-[var(--muted-2)]">
            <span>{benchMeta.cmd}</span>
            <span className="hidden sm:inline">·</span>
            <span>{benchMeta.src}</span>
          </div>
        </div>
      </div>
    </section>
  );
}
