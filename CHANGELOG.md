# Changelog

All notable changes to ClipVault will be documented here.

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
