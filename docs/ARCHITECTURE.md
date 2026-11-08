# Architecture

## Layout

```
clipvault/
  src/
    main.rs        entry point: CLI dispatch, or GUI + monitor thread
    lib.rs         data-dir resolution, time/byte formatting helpers
    app.rs         egui application: sidebar pages, toasts, actions
    ui.rs          theme palette and small shared components
    clipboard.rs   capture, dedup, restore, PNG encode/decode, monitor thread
    storage.rs     SQLite store (items, FTS5, settings, activity), blob store
    privacy.rs     sensitive-content patterns
    settings.rs    settings load/save (stored as JSON in the settings table)
    platform/      platform notes: backend description, Wayland detection
    cli.rs         clap commands, all backed by the same Store
  tests/           integration tests against temp directories
```

## Data flow

1. The monitor thread polls the system clipboard (arboard) every
   `poll_interval_ms` (default 500ms).
2. New content is hashed with BLAKE3. A matching hash means a duplicate:
   nothing is stored again.
3. Text is checked against the sensitive patterns in `privacy.rs`. If it
   matches and `allow_sensitive` is off, only a redacted placeholder and
   the hash are stored.
4. Images are stored once as PNG bytes under `blobs/<hash>.bin`; metadata
   lives in SQLite.
5. The GUI and CLI both read and write through `Store`, guarded by a
   mutex around the single SQLite connection.

## Storage

- `clipvault.db`: items, FTS5 index with triggers, settings, activity log,
  schema metadata. WAL journal mode, foreign keys on.
- `blobs/`: content-addressed binary store, one file per hash.
- Retention and size limits are enforced by `prune_expired` and
  `enforce_limits`; pinned and favorite items are never pruned.

## Threading

- One monitor thread (std thread, not async) does clipboard polling.
- The GUI runs on the main thread via eframe. No shared mutable state
  besides the `Store` mutex and the `PAUSED` atomic flag.
- Tokio is present for future async work (scheduled cleanup) and is used
  by the runtime setup; clipboard polling itself is intentionally a plain
  thread because it is a blocking poll loop.

## Privacy model

- No network access anywhere in the codebase.
- Logs contain event kinds only, never clipboard content.
- Sensitive detection runs locally before anything is written.
- Unix file permissions: data directory 0700, blob files 0600.
