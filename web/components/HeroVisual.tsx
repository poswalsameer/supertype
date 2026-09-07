"use client";
import * as React from "react";
import { MockWindow, OverlayMock } from "./ui/MockWindow";

const lines = [
  { role: "You", text: "Can you send me the analytics report tomorrow morning?" },
  { role: "Supertype", text: "Can you send me the analytics report tomorrow morning?" },
];

export function HeroVisual() {
  const [typed, setTyped] = React.useState("");
  const full = lines[1].text;

  React.useEffect(() => {
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduce) {
      const id = window.setTimeout(() => setTyped(full), 0);
      return () => clearTimeout(id);
    }
    let i = 0;
    let raf = 0;
    let timeout = 0;
    const tick = () => {
      if (i <= full.length) {
        setTyped(full.slice(0, i));
        i += 1;
        timeout = window.setTimeout(() => {
          raf = window.requestAnimationFrame(tick);
        }, 18) as unknown as number;
      }
    };
    tick();
    return () => {
      window.cancelAnimationFrame(raf);
      clearTimeout(timeout);
    };
  }, [full]);

  return (
    <div className="relative">
      <MockWindow title="Notes — Project memo" className="overflow-visible">
        <div className="p-5 md:p-6">
          <div className="flex items-center gap-2 text-[11px] font-medium tracking-wide uppercase text-[var(--muted-2)]">
            <span className="size-2 rounded-full bg-[var(--border-strong)]" /> Today · TextEdit
          </div>
          <div className="mt-4 space-y-3 font-mono text-[12.5px] leading-5">
            <div className="flex gap-3">
              <span className="text-[var(--muted-2)] select-none w-6 text-right">1</span>
              <p className="text-[var(--muted)]">Draft the update for design review:</p>
            </div>
            <div className="flex gap-3">
              <span className="text-[var(--muted-2)] select-none w-6 text-right">2</span>
              <p className="text-[var(--fg)]">
                {typed}
                <span className="inline-block w-px h-[14px] bg-[var(--fg)] ml-px animate-pulse align-[-2px]" />
              </p>
            </div>
            <div className="flex gap-3 opacity-60">
              <span className="text-[var(--muted-2)] select-none w-6 text-right">3</span>
              <p className="text-[var(--muted)]">— spoken once, keep typing</p>
            </div>
          </div>

          <div className="mt-6 flex items-center gap-2 text-[11px] text-[var(--muted)] border-t border-[var(--border)] pt-4">
            <span className="inline-flex items-center gap-1.5 rounded-full bg-[var(--bg-2)] border border-[var(--border)] px-2.5 py-1">
              <span className="size-1.5 rounded-full bg-[#FF3B30]" /> hold <span className="font-mono border border-[var(--border)] rounded px-1 py-0.5 bg-white">fn</span> to talk
            </span>
            <span className="hidden sm:inline">· release to insert</span>
          </div>
        </div>
      </MockWindow>

      <div className="absolute -right-2 -top-3 md:-right-4 md:-top-2 hidden sm:block">
        <OverlayMock state="listening" />
      </div>
      <div className="absolute -left-2 -bottom-3 md:-left-3 md:-bottom-2 hidden md:block">
        <div className="rounded-full border border-[var(--border)] bg-white px-2.5 py-1.5 shadow-sm flex items-center gap-2 text-[11px]">
          <span className="size-1.5 rounded-full bg-[var(--success)]" /> Local · 43 MB
          <span className="text-[var(--border-strong)]">·</span>
          <span className="text-[var(--muted)]">Metal</span>
        </div>
      </div>
    </div>
  );
}
