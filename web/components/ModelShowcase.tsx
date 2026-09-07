import { copy } from "@/lib/copy";

export function ModelShowcase() {
  return (
    <section id="models" className="container-1120 py-14 md:py-20">
      <div className="max-w-[720px]">
        <div className="text-[11px] font-medium tracking-[0.12em] uppercase text-[var(--muted-2)]">{copy.models.eyebrow}</div>
        <h2 className="mt-2 text-[28px] md:text-[36px] font-semibold leading-[1.05] tracking-[-0.03em] whitespace-pre-line">{copy.models.title}</h2>
        <p className="mt-3 max-w-[60ch] text-[15px] leading-7 text-[var(--muted)]">{copy.models.sub}</p>
      </div>

      <div className="mt-8 grid md:grid-cols-2 gap-4">
        {copy.models.cards.map((c) => (
          <div
            key={c.id}
            className="group rounded-[16px] border border-[var(--border)] bg-white p-5 md:p-6 hover:border-[var(--border-strong)] hover:bg-[var(--bg-2)]/30 transition-colors"
          >
            <div className="flex items-start justify-between gap-3">
              <div className="text-[13px] font-semibold tracking-[-0.02em] flex items-center gap-2">
                <span className="size-1.5 rounded-full bg-[var(--fg)]/20 group-hover:bg-[var(--fg)] transition-colors" />
                {c.name}
              </div>
              {c.badge ? (
                <span
                  className={`text-[10px] font-medium tracking-wide uppercase rounded-full px-2 py-1 border ${
                    c.badge === "Recommended"
                      ? "bg-[var(--fg)] text-white border-[var(--fg)]"
                      : "bg-[var(--bg-2)] text-[var(--muted)] border-[var(--border)]"
                  }`}
                >
                  {c.badge}
                </span>
              ) : null}
            </div>
            <div className="mt-1 font-mono text-[11px] tracking-[-0.01em] text-[var(--muted-2)]">{c.meta}</div>
            <p className="mt-3 text-[13px] leading-6 text-[var(--muted)]">{c.desc}</p>
            <div className="mt-4 flex items-center gap-2 text-[11px] text-[var(--muted-2)]">
              <span className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-white px-2 py-1">
                <span className="size-1 rounded-full bg-[var(--success)]" /> Local
              </span>
              <span>· Metal</span>
            </div>
          </div>
        ))}
      </div>

      <p className="mt-4 text-[12px] text-[var(--muted-2)]">{copy.models.foot}</p>
    </section>
  );
}
