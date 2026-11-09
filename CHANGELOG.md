# Changelog

All notable changes to ClipVault will be documented here.

## [0.1.1] - 2026-11-09

- Windows: double-clicking the release exe no longer opens a console
  window. Running the exe from a terminal with a command still prints
  output (the app re-attaches to the parent console).
- New interface: Space Grotesk headings, Inter body text, JetBrains Mono
  for IDs and timestamps; subtle dark vertical gradient in the sidebar
  and main area with a faint accent tint; rounded cards with soft
  shadows; animated sidebar collapse, page fade, staggered card
  entrance, and slide-in toast notifications. Reduced-motion setting
  makes every transition instant. The app only repaints while an
  animation is active.
- Header bar with the app name, version, and monitoring status.

## [0.1.0] - 2026-11-08

- Initial release.
- Clipboard history for text and images (X11 polling on Linux, native
  viewer on Windows), BLAKE3 deduplication, FTS5 search.
- Favorites, pinning, tags, filters, restore to clipboard.
- Sensitive-content detection with withheld-by-default storage.
- Pause monitoring, retention and storage limits, export/import JSON.
- Desktop GUI (egui) with Dashboard, History, Favorites, Snippets,
  Search, Activity, and Settings pages.
- CLI: status, history, search, copy, pin, delete, clear, pause, resume,
  export, import, doctor.
