import * as React from "react";

export function MockWindow({
  title,
  children,
  className = "",
  chrome = true,
}: {
  title?: string;
  children: React.ReactNode;
  className?: string;
  chrome?: boolean;
}) {
  return (
    <div
      className={`rounded-[16px] border border-[var(--border)] bg-white overflow-hidden shadow-[0_8px_32px_rgba(0,0,0,0.06),0_1px_2px_rgba(0,0,0,0.06)] ${className}`}
    >
      {chrome ? (
        <div className="h-9 flex items-center gap-1.5 px-4 border-b border-[var(--border)] bg-[var(--surface-2)]">
          <span className="size-3 rounded-full bg-[#FF5F56] border border-black/10" />
          <span className="size-3 rounded-full bg-[#FFBD2E] border border-black/10" />
          <span className="size-3 rounded-full bg-[#27CA3F] border border-black/10" />
          {title ? <span className="ml-3 text-[12px] font-medium tracking-[-0.01em] text-[var(--muted)]">{title}</span> : null}
        </div>
      ) : null}
      <div className="p-0">{children}</div>
    </div>
  );
}

export function OverlayMock({ state = "listening" }: { state?: "listening" | "processing" | "done" }) {
  const map = {
    listening: { dot: "bg-[#FF3B30]", label: "● Listening", sub: '"Can you send the report..."', bar: true },
    processing: { dot: "bg-[#FF9500]", label: "◐ Processing", sub: "Formatting…", bar: false },
    done: { dot: "bg-[var(--success)]", label: "✓ Done", sub: "Inserted", bar: false },
  }[state];
  return (
    <div className="inline-flex items-center gap-3 rounded-[14px] border border-black/[0.08] bg-white/90 backdrop-blur-md px-4 py-3 shadow-[0_12px_24px_rgba(0,0,0,0.12)]">
      <span className={`size-2.5 rounded-full ${map.dot} ${state === "listening" ? "animate-pulse" : ""}`} />
      <div className="leading-none">
        <div className="text-[13px] font-medium tracking-[-0.01em] text-[var(--fg)]">{map.label}</div>
        <div className="text-[11px] text-[var(--muted)] mt-0.5">{map.sub}</div>
      </div>
      {map.bar ? (
        <div className="ml-2 flex gap-[2px] items-end h-4">
          <span className="w-[2px] h-2 bg-[#FF3B30]/60 rounded-full animate-[bar_0.4s_ease-in-out_infinite]" />
          <span className="w-[2px] h-3 bg-[#FF3B30]/60 rounded-full animate-[bar_0.4s_0.1s_ease-in-out_infinite]" />
          <span className="w-[2px] h-2.5 bg-[#FF3B30]/60 rounded-full animate-[bar_0.4s_0.2s_ease-in-out_infinite]" />
        </div>
      ) : null}
      <style>{`@keyframes bar{0%,100%{transform:scaleY(0.6)}50%{transform:scaleY(1)}}`}</style>
    </div>
  );
}
