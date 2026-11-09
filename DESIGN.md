# ClipVault DESIGN.md

Design direction, written from the PRD's own visual direction. Antislop
filter applied on top (see `antislop.md` in the workspace skill folder).

Reading this as: a desktop utility for people who copy things all day, in a
quiet minimal dark style, dials ENERGY 2 / RHYTHM 2 / MOTION 2.

## Palette (R-29: 2 core colors + 1 accent)

- Core 1, near-black `#0E1114`: window background. Reason: a clipboard
  manager sits in the background all day; a dark neutral base stays out of
  the way and matches the "minimal dark productivity" direction.
- Core 2, charcoal `#161A1F` / `#1C2127`: panels and cards. Reason: one
  step lighter than the background so content cards read as surfaces
  without borders shouting.
- Accent, muted green `#3EC88F`: pinned, favorite, success, and the one
  focal action per screen. Reason: green reads as "kept / safe" for a
  privacy tool, and it is used only at the key moment, never as decoration.
- Neutrals (not counted in the palette): text `#E8EAED`, dim text
  `#9AA3AD`, borders `#2A3037`. Danger red `#E05A4E` is reserved for
  destructive actions and sensitive-content warnings only.

A subtle dark vertical gradient runs behind the sidebar and the main
area (near-black top fading slightly darker toward the bottom, with a
faint green tint along the very top edge). It is the only gradient in
the app: no purple-blue gradients anywhere (R-01), no glow, no
glassmorphism, no background grid. Cards are rounded (12px) with a
1px border and a soft shadow, so surfaces read as lifted, not flat.

## Typography (R-06)

- Headings: Space Grotesk Bold, embedded from `assets/fonts/`. Reason:
  the app's display face; it reads as a designed product instead of a
  default egui demo, while staying neutral and legible.
- Body: Inter Regular (15px), with Inter Medium available for emphasis.
  Reason: a utility app is read in short bursts; Inter stays legible at
  small sizes.
- Monospace: JetBrains Mono (13px) for IDs, timestamps, and hashes,
  never for headings (R-06).

## Layout (R-05, C-3)

- Left sidebar (collapsible to icons) plus one content column. Reason:
  the user's job on each page is one decision: dashboard = "what did I
  just copy", history = "find and copy back", settings = "change what is
  kept". The layout follows that job, not a dashboard template.
- No stat-card row with invented numbers (R-17). Dashboard shows only
  counts computed from the local database: items stored, bytes used,
  pinned/favorite count.
- Clipboard items are rows with a real preview, time, and working buttons
  (Copy back, Pin, Favorite, Delete). Every button does something (R-26).
- Empty states name the cause and the next action (R-27): "Nothing
  captured yet. Copy some text or an image and it will appear here."

## Motion (R-19)

- Page switch fades in over ~180ms; cards on the new page stagger in
  behind it. Sidebar collapse animates its width over ~220ms instead of
  snapping. Toasts slide in from the bottom-right and fade out after 4s.
  Purpose: orient the user after navigation and confirm actions,
  nothing more.
- No endless pulses or loops. Reduced-motion setting makes every
  transition instant.
- No repaint when idle: the app requests a repaint only while an
  animation is in progress (page fade, sidebar width, visible toast).
  The setting exists because a background monitor should not burn CPU.

## Copy (R-02, R-16)

- Plain, specific language. No em dashes in UI text. No buzzwords ("AI
  powered", "seamless", "cutting edge").
- Privacy wording is literal: sensitive content is "withheld" and only
  its hash is kept. Wayland limitation is stated in the app and the
  README, not hidden.

## Icons

- Each sidebar item has one small text glyph next to its label
  (Dashboard ▦, History ↺, Favorites ★, Snippets ✎, Search ⌕,
  Activity ◷, Settings ⚙). Reason: the glyphs mark each destination at
  a glance; they render from the bundled egui fallback fonts, so no icon
  set is imported (R-04). No emoji.
