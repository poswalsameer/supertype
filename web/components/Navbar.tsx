"use client";
import * as React from "react";
import { Button } from "./ui/Button";

export function Navbar() {
  const [scrolled, setScrolled] = React.useState(false);
  const [open, setOpen] = React.useState(false);

  React.useEffect(() => {
    const onScroll = () => setScrolled(window.scrollY > 8);
    onScroll();
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, []);

  return (
    <header
      className={`sticky top-0 z-40 backdrop-blur-[12px] border-b transition-colors ${
        scrolled ? "bg-[var(--bg)]/80 border-[var(--border)]" : "bg-transparent border-transparent"
      }`}
    >
      <nav className="container-1120 h-[56px] flex items-center justify-between gap-6">
        <a href="#" className="flex items-center gap-2.5 shrink-0" aria-label="Supertype home">
          <span className="size-7 rounded-[8px] bg-[var(--fg)] text-white grid place-items-center">
            <span className="text-[11px] font-semibold tracking-[-0.02em]">S</span>
          </span>
          <span className="text-[15px] font-semibold tracking-[-0.03em]">Supertype</span>
          <span className="hidden sm:inline text-[10px] font-medium tracking-widest uppercase text-[var(--muted)] border border-[var(--border)] rounded-full px-2 py-0.5 ml-1">
            macOS
          </span>
        </a>

        <div className="hidden md:flex items-center gap-1 text-[13.5px]">
          <a href="#features" className="px-3 py-2 rounded-md text-[var(--muted)] hover:text-[var(--fg)] hover:bg-[var(--bg-2)] transition-colors">
            Features
          </a>
          <a href="#privacy" className="px-3 py-2 rounded-md text-[var(--muted)] hover:text-[var(--fg)] hover:bg-[var(--bg-2)] transition-colors">
            Privacy
          </a>
          <a href="#models" className="px-3 py-2 rounded-md text-[var(--muted)] hover:text-[var(--fg)] hover:bg-[var(--bg-2)] transition-colors">
            Models
          </a>
          <a
            href="https://github.com/poswalsameer/supertype"
            target="_blank"
            rel="noreferrer"
            className="px-3 py-2 rounded-md text-[var(--muted)] hover:text-[var(--fg)] transition-colors"
          >
            GitHub ↗
          </a>
        </div>

        <div className="hidden md:flex items-center gap-2">
          <Button
            variant="ghost"
            size="sm"
            href="https://github.com/poswalsameer/supertype"
          >
            View source
          </Button>
          <Button
            size="sm"
            href="https://github.com/poswalsameer/supertype/releases/download/v0.2.0/Supertype-0.2.0.dmg"
          >
            Download for macOS
          </Button>
        </div>

        <button
          aria-label={open ? "Close menu" : "Open menu"}
          aria-expanded={open}
          onClick={() => setOpen((v) => !v)}
          className="md:hidden size-9 grid place-items-center rounded-md border border-transparent hover:border-[var(--border)] hover:bg-white"
        >
          <span className="relative block w-4 h-4">
            <span className={`absolute left-0 right-0 h-px bg-[var(--fg)] transition-all ${open ? "top-[7px] rotate-45" : "top-1"}`} />
            <span className={`absolute left-0 right-0 h-px bg-[var(--fg)] top-[7px] transition-opacity ${open ? "opacity-0" : "opacity-100"}`} />
            <span className={`absolute left-0 right-0 h-px bg-[var(--fg)] transition-all ${open ? "top-[7px] -rotate-45" : "top-[13px]"}`} />
          </span>
        </button>
      </nav>

      {open ? (
        <div className="md:hidden border-t border-[var(--border)] bg-[var(--bg)]">
          <div className="container-1120 py-3 flex flex-col gap-1">
            <a href="#features" onClick={() => setOpen(false)} className="px-3 py-2.5 rounded-md hover:bg-[var(--bg-2)] text-[14px]">
              Features
            </a>
            <a href="#privacy" onClick={() => setOpen(false)} className="px-3 py-2.5 rounded-md hover:bg-[var(--bg-2)] text-[14px]">
              Privacy
            </a>
            <a href="#models" onClick={() => setOpen(false)} className="px-3 py-2.5 rounded-md hover:bg-[var(--bg-2)] text-[14px]">
              Models
            </a>
            <a href="https://github.com/poswalsameer/supertype" target="_blank" rel="noreferrer" className="px-3 py-2.5 rounded-md hover:bg-[var(--bg-2)] text-[14px]">
              GitHub ↗
            </a>
            <div className="pt-2 flex gap-2">
              <Button href="https://github.com/poswalsameer/supertype/releases/download/v0.2.0/Supertype-0.2.0.dmg" className="flex-1">
                Download for macOS
              </Button>
            </div>
          </div>
        </div>
      ) : null}
    </header>
  );
}
