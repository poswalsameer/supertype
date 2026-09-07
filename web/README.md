# Supertype — Marketing Site

Premium landing page for the local-first macOS voice-to-text utility. Light, monochrome, typography-first, inspired by Linear / Vercel / Stripe — no gradients, no AI blobs.

Live product truth from `../core` + `../macos` + `../docs/benchmarks/report.md`. No fabricated metrics.

## Stack

- Next.js 16 App Router + TypeScript
- Tailwind CSS 4 (`@import "tailwindcss"`)
- `next/font` Geist Sans / Geist Mono
- `lucide-react` only where semantic
- Static export, no API/DB/auth

## Run locally

```bash
cd web
npm install
npm run dev      # http://localhost:3000
```

## Build & preview production

```bash
cd web
npm run build    # Turbopack, static
npm run start    # http://localhost:3000 (production)
npm run lint     # eslint (next)
```

The site is independent — no `cargo` or macOS app build required.

## Where to edit

**Brand colors / tokens**
`app/globals.css` `:root` — `--bg #FCFCF9` warm off-white, `--fg #0A0A0A` near-black, `--muted #6B6B6B`, `--border #E8E8E3`, `--accent` monochrome (premium). Change these CSS variables; Tailwind `@theme inline` maps them. `tailwind.config.ts` extends if needed.

**Copy**
`lib/copy.ts` — all headlines, subheads, bullets, CTA labels. Edit one file, no component hunting. Source of truth for `Hero`, `Models`, `Privacy`.

**Product mockups**
`components/ui/MockWindow.tsx` — `MockWindow` (macOS traffic lights) + `OverlayMock` (● Listening). `components/HeroVisual.tsx` — typewriter `Can you send the analytics report…` (respects `prefers-reduced-motion`). `components/WorkflowShowcase.tsx` — 4 mocks (VS Code, Slack, Notes, Safari) via `MockWindow`. Replace with real screenshots later: swap `MockWindow` children for `<Image src="/screenshots/..." />` from `public/`.

**Benchmarks**
`lib/benchmarks.ts` — imports real `docs/benchmarks/report.md` values (43/75/142/600 MB, RTF 0.11/0.04/0.02). Do not invent numbers.

**Sections / structure**
`app/page.tsx` composes `Navbar → Hero → ValueStrip → ProblemSolution → HowItWorks → ModelShowcase → PrivacySection → WorkflowShowcase → PerformanceSection → FinalCTA → Footer`. Add/remove by editing this file.

**Navbar & CTA**
`components/Navbar.tsx` — `Download for macOS` href is direct DMG: `https://github.com/poswalsameer/supertype/releases/download/v0.2.0/Supertype-0.2.0.dmg`. Change one place; secondary `View on GitHub` points to `https://github.com/poswalsameer/supertype`.

**SEO**
`app/layout.tsx` `metadata` — title `Supertype — Your voice, everywhere you type.` + Open Graph + Twitter + `favicon.svg` + canonical `https://supertype.app` placeholder. Update `siteUrl` there.

## Add real product screenshots later

1. Export macOS app at 2× (e.g., 1120×700) with overlay visible.
2. Save to `public/screenshots/` as `hero.png`, `overlay.png`, etc.
3. In `components/HeroVisual.tsx`, replace the `MockWindow` inner `<div>` with:
   ```tsx
   import Image from "next/image"
   <Image src="/screenshots/hero.png" alt="Supertype overlay on TextEdit" width={560} height={380} priority />
   ```
4. Keep `MockWindow` chrome as fallback `border` if screenshots are not yet available.

## Design principles

- Typography does the work (Geist, `-0.03em` headings, `52–56px` hero, not oversized).
- Whitespace intentional, no cards-everywhere, moderate radii `8–16px`, buttons `software controls` not pills.
- Monochrome premium: warm off-white `FCFCF9`, near-black `0A0A0A`, single accent = foreground.
- Motion only for state (overlay pulse, typewriter 18ms, hover border). Respects `prefers-reduced-motion`.

## Responsive

`container-1120` `1120px` max, `px-20/24` gutters, `grid lg:grid-cols-2` hero, `md:grid-cols-4` HowItWorks, `md:grid-cols-2` Models/Workflow. Mobile nav collapses to hamburger, type scales `42→52px`, mocks remain legible (scale not shrink). No horizontal overflow (`overflow-hidden` on MockWindow).

## Accessibility

Semantic `header/nav/main/section/footer`, `h1` only hero, `skip to content`, `focus-visible:ring`, `aria-label` on toggles, `alt` on mocks, contrast `19:1`, reduced-motion guard.

## Deploy

Static — deploy `web/` to Vercel (`vercel`), Cloudflare Pages, or any Node host. `next.config.ts` default is standalone; no env vars. Set production domain in `app/layout.tsx` `siteUrl`.
