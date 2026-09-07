import { copy } from "@/lib/copy";

export function Footer() {
  return (
    <footer className="border-t border-[var(--border)] bg-[var(--bg)]">
      <div className="container-1120 py-8 flex flex-col md:flex-row gap-6 justify-between">
        <div>
          <div className="flex items-center gap-2">
            <span className="size-6 rounded-[7px] bg-[var(--fg)] text-white grid place-items-center text-[10px] font-semibold">S</span>
            <span className="text-[13px] font-semibold tracking-[-0.03em]">Supertype</span>
            <span className="text-[11px] font-mono text-[var(--muted-2)]">v0.2.0 · macOS 14+</span>
          </div>
          <p className="mt-2 max-w-[48ch] text-[12.5px] leading-6 text-[var(--muted)]">{copy.footer.tagline} Whisper MIT · Parakeet CC-BY-4.0.</p>
        </div>

        <div className="flex flex-wrap gap-2 text-[12.5px]">
          {copy.footer.links.map((l) => (
            <a
              key={l.label}
              href={l.href}
              target={l.href.startsWith("http") ? "_blank" : undefined}
              rel={l.href.startsWith("http") ? "noreferrer" : undefined}
              className="px-3 py-1.5 rounded-full border border-[var(--border)] bg-white hover:border-[var(--border-strong)] text-[var(--muted)] hover:text-[var(--fg)]"
            >
              {l.label}
            </a>
          ))}
        </div>
      </div>
      <div className="container-1120 pb-8 flex flex-wrap gap-3 text-[11px] text-[var(--muted-2)]">
        <span>© {new Date().getFullYear()} Supertype</span>
        <span className="hidden sm:inline">·</span>
        <span>Local-first · No cloud inference · Audio discarded after insertion</span>
      </div>
    </footer>
  );
}
