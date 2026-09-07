import { copy } from "@/lib/copy";
import { MockWindow } from "./ui/MockWindow";

type CodeLine = { n: number; t: string; hl?: boolean };
type ChatLine = { u: string; m: string };

type AppMock =
  | { title: string; lines: CodeLine[]; isChat?: false }
  | { title: string; lines: ChatLine[]; isChat: true };

const apps: AppMock[] = [
  {
    title: "VS Code — supertype.ts",
    lines: [
      { n: 1, t: "// dictating a comment" },
      { n: 2, t: 'const message = "Can you send the Q4 numbers tomorrow?";', hl: true },
      { n: 3, t: "// typed by voice, formatted locally" },
    ],
  },
  {
    title: "Slack — #design",
    lines: [
      { u: "you", m: "Hey team — can we move review to 10am tomorrow?" },
      { u: "maya", m: "Works for me. I’ll bring the metrics." },
    ],
    isChat: true,
  },
  {
    title: "Notes — Ideas",
    lines: [{ n: 1, t: "Speak naturally. Supertype handles punctuation — comma, period, new line — without thinking about keys." }],
  },
  {
    title: "Safari — Google Docs",
    lines: [{ n: 1, t: "Proposal: migrate analytics to ClickHouse for faster queries and lower cost. Next step: prototype this week." }],
  },
];

export function WorkflowShowcase() {
  return (
    <section className="container-1120 py-14 md:py-20">
      <div className="max-w-[720px]">
        <div className="text-[11px] font-medium tracking-[0.12em] uppercase text-[var(--muted-2)]">{copy.workflow.eyebrow}</div>
        <h2 className="mt-2 text-[28px] md:text-[36px] font-semibold leading-[1.05] tracking-[-0.03em] whitespace-pre-line">{copy.workflow.title}</h2>
        <p className="mt-3 max-w-[60ch] text-[15px] leading-7 text-[var(--muted)]">{copy.workflow.sub}</p>
      </div>

      <div className="mt-8 grid md:grid-cols-2 gap-4">
        {apps.map((a) => (
          <MockWindow key={a.title} title={a.title}>
            <div className="p-5">
              {a.isChat ? (
                <div className="space-y-3">
                  {(a.lines as ChatLine[]).map((l, i) => (
                    <div key={i} className="flex gap-3">
                      <span
                        className={`size-6 rounded-full grid place-items-center text-[10px] font-medium ${l.u === "you" ? "bg-[var(--fg)] text-white" : "bg-[var(--border)] text-[var(--muted)]"}`}
                      >
                        {l.u[0].toUpperCase()}
                      </span>
                      <div className={`rounded-[12px] px-3 py-2 text-[13px] leading-5 ${l.u === "you" ? "bg-[var(--fg)] text-white" : "bg-[var(--bg-2)] border border-[var(--border)]"}`}>
                        {l.m}
                      </div>
                    </div>
                  ))}
                  <div className="text-[11px] text-[var(--muted-2)]">Spoken once → inserted in Slack</div>
                </div>
              ) : (
                <div className="font-mono text-[12.5px] leading-5">
                  {(a.lines as CodeLine[]).map((l) => (
                    <div key={l.n} className="flex gap-3">
                      <span className="text-[var(--muted-2)] select-none w-5 text-right">{l.n}</span>
                      <span className={`${l.hl ? "text-[var(--fg)]" : "text-[var(--muted)]"}`}>{l.t}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </MockWindow>
        ))}
      </div>

      <div className="mt-4 text-center text-[12px] text-[var(--muted-2)]">Native text fields, Electron apps and browsers — if you can type there, you can speak there.</div>
    </section>
  );
}
