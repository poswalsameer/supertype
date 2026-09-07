import { copy } from "@/lib/copy";

export function PrivacySection() {
  return (
    <section id="privacy" className="border-y border-[var(--border)] bg-[var(--surface-2)]">
      <div className="container-1120 py-14 md:py-20">
        <div className="grid lg:grid-cols-[560px_1fr] gap-10 items-start">
          <div>
            <div className="text-[11px] font-medium tracking-[0.12em] uppercase text-[var(--muted-2)]">{copy.privacy.eyebrow}</div>
            <h2 className="mt-2 text-[30px] md:text-[36px] font-semibold leading-[1.05] tracking-[-0.03em] whitespace-pre-line">{copy.privacy.title}</h2>
            <p className="mt-4 max-w-[48ch] text-[15px] leading-7 text-[var(--muted)]">{copy.privacy.sub}</p>

            <ul className="mt-6 space-y-2.5">
              {copy.privacy.bullets.map((b) => (
                <li key={b} className="flex gap-2 text-[13px] leading-6 text-[var(--fg)]">
                  <span className="mt-2 size-1 rounded-full bg-[var(--fg)] shrink-0" />
                  <span>{b}</span>
                </li>
              ))}
            </ul>

            <div className="mt-6 inline-flex items-center gap-2 rounded-full bg-white border border-[var(--border)] px-3 py-1.5 text-[11px] font-medium">
              <span className="size-1.5 rounded-full bg-[var(--success)]" /> No cloud inference after download
            </div>
          </div>

          <div className="rounded-[16px] border border-[var(--border)] bg-white p-6 md:p-7">
            <div className="text-[11px] font-medium tracking-wide uppercase text-[var(--muted-2)]">How it flows</div>
            <div className="mt-5 flex flex-col gap-1">
              {copy.privacy.diagram.map((step, i) => (
                <div key={step} className="flex items-center gap-3">
                  <div className="size-7 rounded-full bg-[var(--fg)] text-white grid place-items-center text-[11px] font-medium">
                    {i + 1}
                  </div>
                  <div className="flex-1 rounded-[10px] border border-[var(--border)] bg-[var(--bg-2)] px-3 py-2.5 text-[13px] font-medium tracking-[-0.01em]">
                    {step}
                  </div>
                </div>
              ))}
              <div className="flex flex-col items-center gap-1 py-1">
                {copy.privacy.diagram.slice(0, -1).map((_, i) => (
                  <div key={i} className="flex flex-col items-center">
                    <span className="w-px h-3 bg-[var(--border-strong)]" />
                    <span className="text-[10px] text-[var(--muted-2)]">↓</span>
                  </div>
                ))}
              </div>
            </div>
            <div className="mt-6 rounded-[12px] bg-[var(--bg-2)] border border-[var(--border)] p-4">
              <div className="text-[11px] font-medium tracking-wide uppercase text-[var(--muted-2)]">Then</div>
              <p className="mt-1 font-mono text-[12px] leading-5 text-[var(--muted)]">Audio discarded. Text inserted at your cursor. Only formatted text may be saved to history if you enable it.</p>
            </div>
            <p className="mt-3 text-[11px] text-[var(--muted-2)]">Verified: <span className="font-mono">AUDIO_NEVER_PERSISTED</span> · no audio SQLite table · redacted logs.</p>
          </div>
        </div>
      </div>
    </section>
  );
}
