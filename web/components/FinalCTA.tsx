import { Button } from "./ui/Button";
import { copy } from "@/lib/copy";

export function FinalCTA() {
  return (
    <section className="border-y border-[var(--border)] bg-[var(--fg)] text-white">
      <div className="container-1120 py-16 md:py-20">
        <div className="grid md:grid-cols-[1.1fr_0.9fr] gap-8 items-center">
          <div>
            <h2 className="text-[32px] md:text-[40px] font-semibold leading-[0.95] tracking-[-0.04em] whitespace-pre-line">{copy.final.title}</h2>
            <p className="mt-4 max-w-[48ch] text-[15px] leading-7 text-white/70">{copy.final.sub}</p>
            <div className="mt-6 flex flex-wrap gap-3">
              <Button
                href="https://github.com/poswalsameer/supertype/releases/download/v0.2.0/Supertype-0.2.0.dmg"
                className="bg-white !text-[var(--fg)] hover:!bg-white/90 border-white"
                size="lg"
              >
                {copy.final.cta}
              </Button>
              <Button href="https://github.com/poswalsameer/supertype" variant="ghost" size="lg" className="!text-white/80 hover:!text-white hover:!bg-white/10">
                View on GitHub ↗
              </Button>
            </div>
            <p className="mt-3 text-[12px] text-white/50">{copy.final.note}</p>
          </div>

          <div className="rounded-[16px] bg-white/5 border border-white/10 p-6 backdrop-blur">
            <div className="text-[11px] font-medium tracking-wide uppercase text-white/50">What you get</div>
            <ul className="mt-3 space-y-2.5 text-[13px] leading-6 text-white/80">
              <li className="flex gap-2"><span className="mt-2 size-1 rounded-full bg-white/60" /> One shortcut, every text field</li>
              <li className="flex gap-2"><span className="mt-2 size-1 rounded-full bg-white/60" /> Local models you control</li>
              <li className="flex gap-2"><span className="mt-2 size-1 rounded-full bg-white/60" /> Punctuation and dictionary, no rewrite</li>
              <li className="flex gap-2"><span className="mt-2 size-1 rounded-full bg-white/60" /> Menu bar utility, stays out of your way</li>
            </ul>
          </div>
        </div>
      </div>
    </section>
  );
}
