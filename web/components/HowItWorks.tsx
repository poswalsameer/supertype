import { copy } from "@/lib/copy";

export function HowItWorks() {
  return (
    <section id="features" className="container-1120 py-14 md:py-20">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <h2 className="text-[24px] md:text-[30px] font-semibold tracking-[-0.03em]">{copy.how.title}</h2>
        <p className="text-[13px] text-[var(--muted)]">Press → Speak → Release → Keep typing</p>
      </div>

      <div className="mt-8 grid md:grid-cols-4 gap-4">
        {copy.how.steps.map((s, i) => (
          <div key={s.n} className="rounded-[16px] border border-[var(--border)] bg-white p-5 md:p-6 flex flex-col min-h-[180px]">
            <div className="text-[11px] font-mono tracking-wide text-[var(--muted-2)]">{s.n}</div>
            <div className="mt-2 text-[15px] font-semibold tracking-[-0.02em]">{s.h}</div>
            <p className="mt-2 text-[13px] leading-6 text-[var(--muted)]">{s.p}</p>
            <div className="mt-auto pt-4">
              <div className="h-px bg-[var(--border)]" />
              <div className="mt-3 inline-flex items-center gap-2 text-[11px] text-[var(--muted-2)]">
                <span className={`size-1.5 rounded-full ${i === 0 ? "bg-[var(--fg)]" : i === 1 ? "bg-[#FF3B30]" : i === 2 ? "bg-[#FF9500]" : "bg-[var(--success)]"}`} />
                {i === 0 ? "Overlay appears" : i === 1 ? "VAD keeps up" : i === 2 ? "Formatter + dictionary" : "At your cursor"}
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="mt-4 rounded-[12px] border border-[var(--border)] bg-[var(--bg-2)] px-4 py-3 flex flex-wrap gap-3 text-[12px] text-[var(--muted)]">
        <span className="inline-flex items-center gap-1.5"><span className="size-1.5 rounded-full bg-[var(--fg)]" /> Hold behavior default</span>
        <span className="text-[var(--border-strong)] hidden sm:inline">·</span>
        <span>Toggle available in Settings → General</span>
      </div>
    </section>
  );
}
