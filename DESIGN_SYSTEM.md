# Design System Reference

A drop-in design brief for any new project. This file describes the look, feel, and component vocabulary of a personal website — a deliberate antidote to the default "vibe-coded" aesthetic (purple gradients, generic shadcn defaults, square buttons, emoji-heavy UI).

The goal: **calm, neutral, tactile, monochrome.** Restraint over flair. Subtle motion over loud effects. Real depth via layered shadows instead of saturated gradients.

---

## 1. Core Principles

1. **Monochrome first.** The base palette is `zinc` (neutral gray). Color is used sparingly as a single accent — never as the primary visual identity, and never as a multi-stop gradient on big surfaces.
2. **No purple/violet/pink gradients.** No `from-purple-500 to-pink-500`, no `bg-gradient-to-r from-violet-* via-fuchsia-* to-rose-*`. If you reach for a purple gradient, stop and rework the design with neutrals + at most one accent.
3. **Tactile, not flashy.** Depth comes from layered inset/outset box shadows (think Linear / Vercel / Things 3), not glowy neon borders.
4. **Generous radii.** Pills (`rounded-full`), large cards (`rounded-2xl` / `rounded-[24px]`), buttons (`rounded-md` / `rounded-lg`). Never sharp corners on interactive elements.
5. **System font stack.** No custom display fonts unless explicitly requested. Tight tracking on large headings.
6. **Motion is functional.** Reveal-on-scroll, hover scale-ups, magnifying dock, encrypted-text reveal. No bouncy spring overuse, no parallax for parallax's sake.
7. **Icons over emoji.** Always Tabler Icons or Lucide. Never emoji in UI.
8. **Dark mode is a peer, not an afterthought.** Both modes ship together; both look intentional. Dark mode is true near-black (not slate, not navy).

---

## 2. Stack

```
Framework:        React 19 + Vite + react-router-dom
Styling:          Tailwind CSS v4 (@tailwindcss/vite)
Component lib:    shadcn/ui  (style: "new-york", baseColor: "zinc")
Icons:            @tabler/icons-react  +  lucide-react
Motion:           motion (formerly framer-motion) + GSAP for advanced effects
Utility merging:  clsx + tailwind-merge  →  cn() helper
Variants:         class-variance-authority (cva)
Primitives:       @radix-ui/* (dialog, dropdown, popover, separator, slot, tooltip)
```

`components.json`:
```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "new-york",
  "tailwind": { "css": "src/App.css", "baseColor": "zinc", "cssVariables": true },
  "iconLibrary": "lucide",
  "registries": {
    "@aceternity": "https://ui.aceternity.com/registry/{name}.json",
    "@paceui-ui":  "https://ui.paceui.com/r/{name}.json",
    "@kokonutui": "https://kokonutui.com/r/{name}.json",
    "@magicui":   "https://magicui.design/r/{name}.json"
  }
}
```

Always pull animated components from those registries first (Aceternity, Magic UI, Kokonut, PaceUI) before hand-rolling.

---

## 3. Color Tokens (oklch)

Use shadcn CSS variables — never hardcode hex/rgb in components. Both modes are defined together. Notice: **no purples, no warm gradients, hue is a tiny tweak (~285°) on near-zero chroma.**

```css
:root {
  --radius: 0.625rem;

  --background:           oklch(1 0 0);                  /* pure white */
  --foreground:           oklch(0.141 0.005 285.823);    /* near-black, faint cool tint */
  --card:                 oklch(1 0 0);
  --card-foreground:      oklch(0.141 0.005 285.823);
  --primary:              oklch(0.21 0.006 285.885);     /* dark zinc */
  --primary-foreground:   oklch(0.985 0 0);
  --secondary:            oklch(0.967 0.001 286.375);    /* very light zinc */
  --secondary-foreground: oklch(0.21 0.006 285.885);
  --muted:                oklch(0.967 0.001 286.375);
  --muted-foreground:     oklch(0.552 0.016 285.938);    /* mid-gray */
  --accent:               oklch(0.967 0.001 286.375);
  --accent-foreground:    oklch(0.21 0.006 285.885);
  --destructive:          oklch(0.577 0.245 27.325);     /* red — only color allowed at saturation */
  --border:               oklch(0.92 0.004 286.32);
  --input:                oklch(0.92 0.004 286.32);
  --ring:                 oklch(0.705 0.015 286.067);
}

.dark {
  --background:           oklch(0.141 0.005 285.823);    /* near-black, NOT navy/slate */
  --foreground:           oklch(0.985 0 0);
  --card:                 oklch(0.21 0.006 285.885);     /* one notch lighter than bg */
  --primary:              oklch(0.92 0.004 286.32);
  --primary-foreground:   oklch(0.21 0.006 285.885);
  --secondary:            oklch(0.274 0.006 286.033);
  --muted:                oklch(0.274 0.006 286.033);
  --muted-foreground:     oklch(0.705 0.015 286.067);
  --border:               oklch(1 0 0 / 10%);            /* translucent white borders */
  --input:                oklch(1 0 0 / 15%);
}
```

### Allowed accent colors (use ONE per project, sparingly)
- **Light blue** `#ADD8E6` — used for ripple effects on primary CTAs.
- **Lime** `#A1FF40` — alt ripple color.
- **Orange-500** `#f97316` — used on a single playful card component (CardFlip).
- **Cyan→indigo gradient** for thin tracing-beam lines only (never as a fill).

Forbidden by default: purple, violet, fuchsia, pink, magenta gradients. If the user wants color, add **one** of the above. Don't combine two.

---

## 4. Radii

```
--radius: 0.625rem            (10px) — base
rounded-md   → 8px             — buttons, small inputs
rounded-lg   → 10px            — buttons, larger inputs, login cards
rounded-xl   → 14px            — project cards
rounded-2xl  → 16px            — hero containers, the floating dock
rounded-[24px] / rounded-[20px] / rounded-[16px]  — minimal cards (nested radii)
rounded-full → pills           — search bar, social-icon buttons, tags
```

Rule of thumb: **interactive small thing → `rounded-md/lg`; container → `rounded-2xl`; pill or icon-only → `rounded-full`.** Never use sharp corners on hoverable elements.

---

## 5. Shadows (the secret sauce)

The "expensive" feel comes from layered shadows, not gradients. Copy these verbatim onto card-like containers.

**Light-mode card stack (gives a paper-with-bevel feel):**
```
shadow-[0px_1px_1px_0px_rgba(0,0,0,0.05),
        0px_1px_1px_0px_rgba(255,252,240,0.5)_inset,
        0px_0px_0px_1px_hsla(0,0%,100%,0.1)_inset,
        0px_0px_1px_0px_rgba(28,27,26,0.5)]

shadow-[rgba(17,24,28,0.08)_0_0_0_1px,
        rgba(17,24,28,0.08)_0_1px_2px_-1px,
        rgba(17,24,28,0.04)_0_2px_4px]
```

**Dark-mode card stack (multi-layer drop):**
```
dark:shadow-[0_1px_0_0_rgba(255,255,255,0.03)_inset,
             0_0_0_1px_rgba(255,255,255,0.03)_inset,
             0_0_0_1px_rgba(0,0,0,0.1),
             0_2px_2px_0_rgba(0,0,0,0.1),
             0_4px_4px_0_rgba(0,0,0,0.1),
             0_8px_8px_0_rgba(0,0,0,0.1)]
```

**Image inside a card** gets a triple-ring framing shadow (white inner liner, hairline outer):
```
shadow-[0px_0px_0px_1px_rgba(0,0,0,.07),
        0px_0px_0px_3px_#fff,
        0px_0px_0px_4px_rgba(0,0,0,.08)]
```

**Search-bar / pill input** uses a softer 1px-border imitation:
```
shadow-[0px_2px_3px_-1px_rgba(0,0,0,0.1),
        0px_1px_0px_0px_rgba(25,28,33,0.02),
        0px_0px_0px_1px_rgba(25,28,33,0.08)]
```

---

## 6. Typography

- Stack: system default (Tailwind preflight). No Google fonts unless asked.
- Headings: `font-bold`, `tracking-tight` or `tracking-[-0.02em]` on hero text.
- Sizes used: `text-4xl` (page hero), `text-3xl` (section headings), `text-2xl` (sub-section), `text-lg` (card titles), `text-base` (body), `text-sm` (descriptions/meta), `text-xs` (timestamps/tags).
- Body text on cards is `text-muted-foreground` — never pure black on white.
- Hero/title text often gets the **EncryptedText** scramble-reveal animation on first viewport entry.

---

## 7. Component Vocabulary

### 7.1 Buttons

**Default shadcn Button (cva variants):**
```jsx
// inline-flex items-center justify-center gap-2 whitespace-nowrap
// rounded-md text-sm font-medium transition-all
// disabled:pointer-events-none disabled:opacity-50
// focus-visible:ring-ring/50 focus-visible:ring-[3px]
variants:
  default     → bg-primary text-primary-foreground hover:bg-primary/90
  destructive → bg-destructive text-white hover:bg-destructive/90
  outline     → border bg-background shadow-xs hover:bg-accent
                dark:bg-input/30 dark:border-input dark:hover:bg-input/50
  secondary   → bg-secondary text-secondary-foreground hover:bg-secondary/80
  ghost       → hover:bg-accent hover:text-accent-foreground
  link        → text-primary underline-offset-4 hover:underline
sizes:
  default     → h-9 px-4 py-2
  sm          → h-8 px-3 rounded-md
  lg          → h-10 px-6 rounded-md
  icon        → size-9
```

**RippleButton** — used for primary CTAs ("Read more", "Submit"):
- `bg-background` with a 2px border, `rounded-lg`, `px-4 py-2`.
- On click, spawns a circular ripple from cursor position with a soft opacity-30 fill.
- Default ripple color: light-blue `#ADD8E6` or lime `#A1FF40` — **never purple**.
- 600ms duration, ease-out.

**Icon buttons (e.g. social links)** use `rounded-full border border-border hover:bg-accent transition-colors` with `p-3` padding and a `w-5 h-5` icon inside. This is the standard for any external-link circle button.

**No** giant gradient buttons. **No** glow halos. **No** "shimmer" buttons unless explicitly asked.

### 7.2 Cards

**MinimalCard** (the main blog/project card pattern):
```
rounded-[24px]
bg-neutral-50 dark:bg-neutral-800
p-2   ← important: card itself has only 2px padding; image is the focal element
hover:bg-neutral-100 dark:hover:bg-neutral-800/80
+ both layered shadow stacks from §5
inner image:
  rounded-[20px], h-[190px], with the triple-ring framing shadow
title:        text-lg mt-2 font-semibold leading-tight px-1
description:  text-sm text-neutral-500 pb-2 px-1
```

**Project card** (about-page grid):
```
group relative flex flex-col gap-3 p-5 rounded-xl
border border-border bg-card
hover:bg-accent/50 hover:shadow-lg hover:-translate-y-0.5
transition-all duration-200
```
- External-link icon (`IconExternalLink`) in the top right, `opacity-0 group-hover:opacity-100`.
- Tags: `text-xs px-2 py-0.5 rounded-full bg-secondary text-secondary-foreground`.
- Language indicator: a `w-2.5 h-2.5 rounded-full` colored dot (GitHub-style).

**BlurCard** (featured / hero post): a wide card paired with a square image. Image gets a **ProgressiveBlur** at the bottom 25% with text overlaid in white/zinc-300. Excerpt fades to background via `bg-gradient-to-t from-background to-transparent` mask.

### 7.3 Floating Dock (signature element)

A bottom-fixed Apple-style dock — the site's identity piece.

- Container: `mx-auto h-16 items-end gap-4 rounded-2xl bg-gray-100 dark:bg-neutral-900 px-4 pb-3` — desktop only (`hidden md:flex`).
- Each item is `rounded-full bg-gray-200 dark:bg-neutral-800`, base size `40×40`, magnifies up to `80×80` as the cursor approaches (motion `useTransform` over a `useMotionValue` of mouseX, with `useSpring`: `mass 0.1, stiffness 150, damping 12`).
- Tooltip on hover: `absolute -top-8 rounded-md border bg-gray-100 dark:bg-neutral-800 px-2 py-0.5 text-xs`.
- Icon color: `text-neutral-500 dark:text-neutral-300`.
- Mobile variant: a single collapsed button (`IconLayoutNavbarCollapse`) that fans the items out via `AnimatePresence` with staggered scale-from-0.5.

This dock is the navigation pattern for the whole site. Don't replace it with a top navbar.

### 7.4 Search input (vanish-on-submit)

- `h-12 rounded-full bg-white dark:bg-zinc-800` with the search-bar shadow stack from §5.
- Submit button on the right is a `h-8 w-8 rounded-full bg-black dark:bg-zinc-900` with an animated arrow (motion path stroke-dashoffset).
- On submit, the typed text "vanishes" — pixel particles drift outward on a canvas overlay. (Component: `PlaceholdersAndVanishInput`.)

### 7.5 Theme toggler (View Transition API)

Use `document.startViewTransition` + a circular `clipPath` animation expanding from the toggle button's center. 400ms `ease-in-out`. Falls back to plain class-swap when unsupported.

```css
::view-transition-old(root),
::view-transition-new(root) { animation: none; mix-blend-mode: normal; }
```

Icon: Lucide `Sun` / `Moon`. Button itself: `p-2 rounded-md border-border bg-background hover:bg-accent transition-colors`.

### 7.6 Text effects

- **EncryptedText** — characters scramble through a charset, then resolve left-to-right. Used on names, titles, hero text. Triggers via `useInView({ once: true })`.
- **TypingAnimation** — typewriter with a blinking cursor. Cursor styles: `"line"` (`|`), `"block"` (`▌`), `"underscore"` (`_`). Default to `block` for hero headings.
- **RevealOnScroll** — IntersectionObserver-driven `opacity 0→1` + `translateY(40px → 0)` over 600–700ms. Stagger lists with `delay: 150 + i*75`.

### 7.7 Decorative

- **DotFlow / DotLoader** — GSAP-driven 7×7 dot grid that animates frame-by-frame to spell short words. Used as a logo / wordmark, not as a busy background.
- **TracingBeam** — thin SVG line with a cyan→indigo→purple gradient stop that follows scroll. Reserved for long-form blog posts only; never as page chrome. (This is the *only* place a multi-color gradient appears.)
- **ProgressiveBlur** — frosted bottom edge on hero images, lets text sit cleanly over photography.

---

## 8. Layout Patterns

- **Container width**: `max-w-7xl px-8 py-16` for content sections, `max-w-[1200px]` for hero blocks, `max-w-4xl` for blog body, `max-w-sm` for forms.
- **Page shell**: `flex flex-col items-center justify-start min-h-screen w-full`.
- **Top bar**: a single flex row with logo (DotFlow) on the left, centered search (absolute positioned), theme toggle on the right.
- **Bottom**: floating dock fixed at `bottom-2 left-1/2 -translate-x-1/2 z-50`.
- **Grids**: blog/project grids are `grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6`.
- **Section headings**: `text-3xl font-bold mb-8` with a `RevealOnScroll` wrapper.

---

## 9. Motion guidelines

- Library: `motion` (the new framer-motion). Use GSAP only for complex sequencing (DotFlow).
- Default easing: `power2.out` (GSAP) or `ease-out` (CSS).
- Default durations: 200ms for hover state, 400–700ms for entrance, 700ms for card flips.
- Spring for cursor-following effects only: `{ mass: 0.1, stiffness: 150, damping: 12 }`.
- `transition-all duration-200` is fine for hover; `duration-700 ease-out-expo` for showcase transforms.
- **Hover transforms** on cards: `hover:-translate-y-0.5 hover:shadow-lg`. Subtle, not bouncy.

---

## 10. Iconography

- Primary set: `@tabler/icons-react` — `IconHome`, `IconUser`, `IconPencil`, `IconBrandGithub`, `IconBrandX`, `IconBrandLinkedin`, `IconExternalLink`, `IconLayoutNavbarCollapse`.
- Secondary: `lucide-react` — used inside shadcn primitives, theme toggle (`Sun`, `Moon`), and decorative arrows (`ArrowRight`).
- Never mix more than these two. **Never use emoji as UI affordances.**
- Standard sizes: `w-4 h-4` (inline), `w-5 h-5` (button), `w-6 h-6` (large hit-target). For Tabler in the dock the icon stretches via `h-full w-full` inside a sized parent.
- Icon color in neutral context: `text-neutral-500 dark:text-neutral-300` or `text-muted-foreground`.

---

## 11. The `cn` helper

Mandatory. Every component that takes `className` does:

```js
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
export const cn = (...inputs) => twMerge(clsx(inputs));
```

Then `className={cn("base classes", conditional && "...", className)}`.

---

## 12. Anti-patterns (do NOT generate these)

- ❌ `bg-gradient-to-r from-purple-500 to-pink-500` or any violet/fuchsia/indigo gradient as a fill.
- ❌ Glowing neon borders (`shadow-[0_0_20px_purple]`).
- ❌ Square-cornered cards or buttons (`rounded-none`, `rounded-sm`).
- ❌ Emoji in headings, buttons, or card titles. Use Tabler/Lucide icons.
- ❌ Centered single-column layouts that span the whole viewport — content lives in `max-w-[1200px]` or smaller.
- ❌ Default shadcn-only output with no shadow layering (looks flat and AI-generated).
- ❌ Custom display fonts as default (Inter, Geist Mono are fine if requested; never Pacifico/Lobster/etc.).
- ❌ Toast popups, confetti, modal overuse.
- ❌ Generic Hero → Features → Pricing → CTA SaaS layout. Personal sites lead with content (latest post, projects), not a marketing pitch.
- ❌ Putting the navigation in a top bar across the whole width — use the FloatingDock instead.

---

## 13. Quick-start checklist for a new project

1. `tailwindcss@^4 + @tailwindcss/vite`, `clsx`, `tailwind-merge`, `class-variance-authority`.
2. Drop in `App.css` from §3 verbatim (oklch zinc tokens, both modes, view-transition reset).
3. `npx shadcn@latest init` → choose **new-york** style, **zinc** base color, CSS variables on.
4. Install: `motion`, `@tabler/icons-react`, `lucide-react`, the relevant `@radix-ui/*` primitives.
5. Set up `ThemeProvider` (localStorage key `vite-ui-theme`, default `"system"`, toggles `.dark` class on `<html>`).
6. Add `lib/utils.js` with `cn()`.
7. Pull these components from registries: `floating-dock`, `minimal-card`, `ripple-button`, `encrypted-text`, `typing-animation`, `animated-theme-toggler`, `placeholders-and-vanish-input`, `progressive-blur`, `reveal-on-scroll`.
8. Build the page with: floating dock at bottom, content `max-w-[1200px]` centered, sections wrapped in `RevealOnScroll`, all cards = MinimalCard with the layered shadow stack from §5.
9. Use **one** accent color (light-blue ripple is the safe default). Stop there.

If anything in the above conflicts with the user's explicit instruction, the user wins — but don't drift back to the purple-gradient default just because it's easy.
