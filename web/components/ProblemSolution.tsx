import { copy } from "@/lib/copy";

export function ProblemSolution() {
  return (
    <section className="container-1120 py-14 md:py-20">
      <div className="grid md:grid-cols-2 gap-10 md:gap-12 items-start">
        <div>
          <h2 className="text-[28px] md:text-[34px] font-semibold leading-[1.05] tracking-[-0.03em] whitespace-pre-line">{copy.problem.title}</h2>
          <p className="mt-4 max-w-[48ch] text-[15px] leading-7 text-[var(--muted)]">{copy.problem.p1}</p>
          <p className="mt-3 text-[13px] font-medium tracking-[-0.01em]">{copy.problem.p2}</p>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div className="rounded-[16px] border border-[var(--border)] bg-white p-5">
            <div className="text-[11px] font-medium tracking-wide uppercase text-[var(--muted-2)]">Typing</div>
            <div className="mt-3 font-mono text-[12.5px] leading-5 text-[var(--muted)]">“I’ll write this…”</div>
            <div className="mt-4 text-[11px] text-[var(--muted-2)]">Slow · interrupts flow</div>
          </div>
          <div className="rounded-[16px] border border-[var(--fg)] bg-[var(--fg)] p-5 text-white">
            <div className="text-[11px] font-medium tracking-wide uppercase text-white/60">Voice</div>
            <div className="mt-3 font-mono text-[12.5px] leading-5">“I’ll just say it.”</div>
            <div className="mt-4 text-[11px] text-white/60">Hold · speak · done</div>
          </div>
          <div className="col-span-2 rounded-[12px] border border-[var(--border)] bg-[var(--bg-2)] px-4 py-3 text-[12.5px] text-[var(--muted)] flex items-center gap-2">
            <span className="size-2 rounded-full bg-[var(--fg)]" />
            One shortcut. Every text field. No window switch.
          </div>
        </div>
      </div>
    </section>
  );
}
