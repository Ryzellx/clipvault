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

No purple-blue gradients anywhere (R-01). No glow, no glassmorphism, no
background grid. Flat surfaces, one accent, generous spacing.

## Typography (R-06)

- egui default proportional font for body and headings. Reason: a utility
  app is read in short bursts; the default UI font is the most legible
  choice at small sizes and needs no custom font download.
- Monospace only for hashes and IDs, never for headings (R-06).

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

- Page switch fades in over ~120ms; sidebar collapse is immediate with a
  short ease. Purpose: orient the user after navigation, nothing more.
- No endless pulses or loops. Reduced-motion setting disables the fade.
- No repaint when idle: the app requests repaint on a 33ms timer only
  while animations are active; the setting exists because a background
  monitor should not burn CPU.

## Copy (R-02, R-16)

- Plain, specific language. No em dashes in UI text. No buzzwords ("AI
  powered", "seamless", "cutting edge").
- Privacy wording is literal: sensitive content is "withheld" and only
  its hash is kept. Wayland limitation is stated in the app and the
  README, not hidden.

## Icons

- Text labels only in this release; no icon set is imported. Reason: the
  sidebar has seven short labels that read faster as words, and a generic
  icon font would add a recognizable library look without adding meaning
  (R-04). Icons may be added later if a set is chosen for a real reason.
