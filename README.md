# ClipVault

Privacy-first, offline clipboard history manager for Windows and Linux.
Built in Rust with egui. No account, no cloud, no telemetry.

![License: MIT](https://img.shields.io/badge/license-MIT-green)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-blue)

## What it is

ClipVault watches your clipboard (only when you let it), keeps a searchable
history of text and images on your own disk, and lets you copy any earlier
item back with one click. Sensitive content such as API keys, tokens, and
passwords is **not stored by default**: ClipVault keeps a redacted
placeholder and the content hash only, and tells you it did so.

## Screenshots

No screenshots yet. Real screenshots of the running app will be added to
`docs/screenshots/` once captured; none are faked here.

## Features

- Clipboard history for text and images, with BLAKE3 hash deduplication
- Full-text search (SQLite FTS5), favorites, pinning, tags, filters
- Restore any item back to the clipboard
- Sensitive-content detection (API keys, tokens, private keys, passwords):
  withheld by default, opt-in to store
- Pause monitoring, per-application exclusions, automatic expiration
- Storage limits with automatic cleanup, database integrity checks
- Export and import as JSON
- CLI with the same storage: `status`, `history`, `search`, `copy`, `pin`,
  `delete`, `clear`, `pause`, `resume`, `export`, `import`, `doctor`
- Private storage permissions (0700 data directory, 0600 files on Unix)

## Install

### From a release

Download the release for your platform from the Releases page and run the
binary. ClipVault stores its data in:

- Linux: `~/.local/share/clipvault/`
- Windows: `%LOCALAPPDATA%\ClipVault\`

### From source

Requires a stable Rust toolchain (1.85 or newer) and, on Linux, X11/GL
development libraries (`libxcb`, `libxkbcommon`, `libGL`).

```sh
git clone https://github.com/ryzellx/clipvault
cd clipvault
cargo build --release
./target/release/clipvault
```

## Quick start

1. Run `clipvault`. Copy some text. It appears in History.
2. Click **Copy back** on any item to put it on the clipboard again.
3. Pin items you want to keep; pinned items survive cleanup and clearing.
4. Open Settings to change retention, storage limits, and whether
   sensitive content may be stored.

## CLI reference

See [docs/CLI.md](docs/CLI.md). Quick examples:

```sh
clipvault status
clipvault history --limit 20
clipvault search "invoice"
clipvault copy 42
clipvault pin 42
clipvault export --file backup.json
clipvault import backup.json
clipvault doctor
```

## Architecture

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). In short: an arboard-based
clipboard monitor thread feeds a SQLite metadata store with a separate
content-addressed blob directory; the egui app and the CLI share the same
`Store` type.

## Privacy and security

- Everything is local. ClipVault makes no network requests.
- Clipboard content never appears in logs.
- Sensitive content is detected with local patterns and withheld unless
  you explicitly allow storing it in Settings.
- No keylogging, no keyboard monitoring of any kind.
- Data files are created with private Unix permissions.

## Known limitations

- **Wayland (Linux):** persistent clipboard access is unavailable on
  Wayland compositors. ClipVault captures reliably on X11; on Wayland it
  can only see clipboard content while it owns the selection, so history
  may be incomplete. This is stated in the app and in `clipvault doctor`.
- **Rich text:** formatted text is stored as plain text in this release.
- **Application exclusion:** on Linux the active application is not
  reliably detectable from the clipboard alone, so exclusion by process
  name is best-effort and currently limited to Windows detection paths.

## Contributing

Issues and pull requests are welcome. Run `cargo fmt`, `cargo clippy
--all-targets`, and `cargo test` before submitting.

## License

MIT. See [LICENSE](LICENSE).
