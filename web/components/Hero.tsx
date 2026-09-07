import { Button } from "./ui/Button";
import { HeroVisual } from "./HeroVisual";
import { copy } from "@/lib/copy";

export function Hero() {
  return (
    <section className="container-1120 pt-10 md:pt-16 pb-8">
      <div className="grid lg:grid-cols-[560px_1fr] gap-10 lg:gap-8 items-start">
        <div className="pt-2">
          <div className="inline-flex items-center gap-2 rounded-full border border-[var(--border)] bg-white px-3 py-1.5">
            <span className="size-1.5 rounded-full bg-[var(--success)] animate-pulse" />
            <span className="text-[11px] font-medium tracking-wide uppercase text-[var(--muted)]">{copy.hero.eyebrow}</span>
          </div>

          <h1 className="mt-6 text-[42px] md:text-[52px] lg:text-[56px] font-semibold leading-[0.95] tracking-[-0.04em]">
            {copy.hero.title}
            <br />
            <span className="text-[var(--muted)]">{copy.hero.titleAccent}</span>
          </h1>

          <p className="mt-5 max-w-[52ch] text-[16px] md:text-[17px] leading-7 text-[var(--muted)]">
            {copy.hero.sub}
          </p>

          <div className="mt-7 flex flex-wrap gap-3">
            <Button href="https://github.com/poswalsameer/supertype/releases/download/v0.2.0/Supertype-0.2.0.dmg" size="lg">
              {copy.hero.ctaPrimary}
            </Button>
            <Button variant="secondary" size="lg" href="https://github.com/poswalsameer/supertype">
              {copy.hero.ctaSecondary}
            </Button>
          </div>

          <p className="mt-3 text-[12.5px] text-[var(--muted-2)]">{copy.hero.ctaNote} · No account</p>

          <div className="mt-8 hidden md:flex items-center gap-3 text-[12.5px] text-[var(--muted)]">
            <span className="inline-flex items-center gap-1.5">
              <span className="size-1.5 rounded-full bg-[var(--fg)]" /> Hold to talk
            </span>
            <span className="text-[var(--border-strong)]">—</span>
            <span>Works in every text field</span>
          </div>
        </div>

        <div className="lg:pl-4">
          <HeroVisual />
          <p className="mt-3 text-center text-[12px] text-[var(--muted-2)]">{copy.hero.demoCaption}</p>
        </div>
      </div>

      <div className="mt-10 flex flex-wrap gap-2 text-[11px] font-medium tracking-wide uppercase text-[var(--muted-2)]">
        <span className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-white px-2.5 py-1">
          <span className="size-1.5 rounded-full bg-black/70" /> macOS 14+
        </span>
        <span className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-white px-2.5 py-1">Apple Silicon</span>
        <span className="inline-flex items-center gap-1.5 rounded-full border border-[var(--border)] bg-white px-2.5 py-1">Offline after download</span>
      </div>
    </section>
  );
}
