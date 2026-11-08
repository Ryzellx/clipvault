# ClipVault CLI

Run `clipvault --help` for the full list. All commands operate on the same
local data directory as the GUI (`~/.local/share/clipvault` on Linux,
`%LOCALAPPDATA%\ClipVault` on Windows, or `--data-dir <path>`).

| Command | What it does |
| --- | --- |
| `clipvault status` | Item count, storage used, monitoring state, integrity |
| `clipvault history [--limit N]` | List recent items (default 20) |
| `clipvault search "query"` | Full-text search over captured text |
| `clipvault copy ITEM_ID` | Copy the item back to the system clipboard |
| `clipvault pin ITEM_ID` | Toggle the pinned flag on an item |
| `clipvault delete ITEM_ID` | Delete one item |
| `clipvault clear --yes` | Delete all unpinned items (requires `--yes`) |
| `clipvault pause` | Pause monitoring |
| `clipvault resume` | Resume monitoring |
| `clipvault export [--file PATH]` | Export all items as JSON (stdout if no file) |
| `clipvault import FILE` | Import items from a JSON export |
| `clipvault doctor` | Data dir, database integrity, clipboard access |

With no subcommand, `clipvault` opens the desktop GUI.
