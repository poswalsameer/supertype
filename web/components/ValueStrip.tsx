import { copy } from "@/lib/copy";

export function ValueStrip() {
  return (
    <section aria-label="Key properties" className="border-y border-[var(--border)] bg-[var(--surface-2)]">
      <div className="container-1120 py-3 flex flex-wrap gap-x-6 gap-y-2 text-[12px] font-medium tracking-[-0.01em] text-[var(--muted)] justify-center md:justify-between">
        {copy.trust.map((v) => (
          <span key={v} className="inline-flex items-center gap-2">
            <span className="size-1 rounded-full bg-[var(--fg)]/20" />
            {v}
          </span>
        ))}
      </div>
    </section>
  );
}
