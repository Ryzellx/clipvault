use crate::now_unix;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemKind {
    Text,
    Image,
    Redacted,
}

impl ItemKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ItemKind::Text => "text",
            ItemKind::Image => "image",
            ItemKind::Redacted => "redacted",
        }
    }

    pub fn from_name(s: &str) -> Self {
        match s {
            "image" => ItemKind::Image,
            "redacted" => ItemKind::Redacted,
            _ => ItemKind::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub uuid: String,
    pub hash: String,
    pub kind: ItemKind,
    pub text: Option<String>,
    pub preview: String,
    pub byte_len: u64,
    pub created_at: i64,
    pub pinned: bool,
    pub favorite: bool,
    pub tags: Vec<String>,
    pub source_app: Option<String>,
    pub sensitive: bool,
    pub blob_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: i64,
    pub kind: String,
    pub detail: String,
    pub created_at: i64,
}

pub struct Store {
    conn: Mutex<Connection>,
    pub data_dir: PathBuf,
}

impl Store {
    pub fn open(data_dir: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(data_dir.join("blobs")).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(data_dir, std::fs::Permissions::from_mode(0o700));
            let _ = std::fs::set_permissions(
                data_dir.join("blobs"),
                std::fs::Permissions::from_mode(0o700),
            );
        }
        let db_path = data_dir.join("clipvault.db");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(|e| e.to_string())?;
        let store = Self {
            conn: Mutex::new(conn),
            data_dir: data_dir.to_path_buf(),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_metadata (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS items (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               uuid TEXT UNIQUE NOT NULL,
               hash TEXT NOT NULL,
               kind TEXT NOT NULL,
               text TEXT,
               preview TEXT NOT NULL DEFAULT '',
               byte_len INTEGER NOT NULL DEFAULT 0,
               created_at INTEGER NOT NULL,
               pinned INTEGER NOT NULL DEFAULT 0,
               favorite INTEGER NOT NULL DEFAULT 0,
               tags TEXT NOT NULL DEFAULT '[]',
               source_app TEXT,
               sensitive INTEGER NOT NULL DEFAULT 0,
               blob_path TEXT,
               width INTEGER,
               height INTEGER
             );
             CREATE INDEX IF NOT EXISTS idx_items_hash ON items(hash);
             CREATE INDEX IF NOT EXISTS idx_items_created ON items(created_at DESC);
             CREATE INDEX IF NOT EXISTS idx_items_pinned ON items(pinned);
             CREATE INDEX IF NOT EXISTS idx_items_favorite ON items(favorite);
             CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(text, content='items', content_rowid='id');
             CREATE TRIGGER IF NOT EXISTS items_ai AFTER INSERT ON items BEGIN
               INSERT INTO items_fts(rowid, text) VALUES (new.id, COALESCE(new.text, ''));
             END;
             CREATE TRIGGER IF NOT EXISTS items_ad AFTER DELETE ON items BEGIN
               INSERT INTO items_fts(items_fts, rowid, text) VALUES('delete', old.id, COALESCE(old.text, ''));
             END;
             CREATE TRIGGER IF NOT EXISTS items_au AFTER UPDATE ON items BEGIN
               INSERT INTO items_fts(items_fts, rowid, text) VALUES('delete', old.id, COALESCE(old.text, ''));
               INSERT INTO items_fts(rowid, text) VALUES (new.id, COALESCE(new.text, ''));
             END;
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS activity (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               kind TEXT NOT NULL,
               detail TEXT NOT NULL,
               created_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_activity_created ON activity(created_at DESC);
             INSERT OR IGNORE INTO schema_metadata(key, value) VALUES('version', '1');",
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn integrity_check(&self) -> Result<String, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let result: String = conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO settings(key, value) VALUES(?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn log_activity(&self, kind: &str, detail: &str) {
        let conn = match self.conn.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = conn.execute(
            "INSERT INTO activity(kind, detail, created_at) VALUES(?1, ?2, ?3)",
            params![kind, detail, now_unix()],
        );
    }

    pub fn insert_item(&self, item: &NewItem) -> Result<Option<i64>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM items WHERE hash = ?1 ORDER BY created_at DESC LIMIT 1",
                params![item.hash],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        if existing.is_some() {
            return Ok(None);
        }
        let uuid = format!(
            "cv-{}-{}",
            now_unix(),
            &blake3::hash(item.hash.as_bytes()).to_hex()[..12]
        );
        conn.execute(
            "INSERT INTO items(uuid, hash, kind, text, preview, byte_len, created_at, pinned, favorite, tags, source_app, sensitive, blob_path, width, height)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, '[]', ?8, ?9, ?10, ?11, ?12)",
            params![
                uuid,
                item.hash,
                item.kind.as_str(),
                item.text,
                item.preview,
                item.byte_len as i64,
                now_unix(),
                item.source_app,
                item.sensitive as i32,
                item.blob_path,
                item.width.map(|w| w as i64),
                item.height.map(|h| h as i64),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(Some(conn.last_insert_rowid()))
    }

    pub fn get_item(&self, id: i64) -> Result<Option<Item>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare("SELECT id, uuid, hash, kind, text, preview, byte_len, created_at, pinned, favorite, tags, source_app, sensitive, blob_path, width, height FROM items WHERE id = ?1")
            .map_err(|e| e.to_string())?;
        let row = stmt
            .query_row(params![id], row_to_item)
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(row)
    }

    pub fn list_items(&self, limit: u32, offset: u32) -> Result<Vec<Item>, String> {
        self.query_items("SELECT id, uuid, hash, kind, text, preview, byte_len, created_at, pinned, favorite, tags, source_app, sensitive, blob_path, width, height FROM items ORDER BY pinned DESC, created_at DESC LIMIT ?1 OFFSET ?2", params![limit as i64, offset as i64])
    }

    pub fn list_favorites(&self) -> Result<Vec<Item>, String> {
        self.query_items("SELECT id, uuid, hash, kind, text, preview, byte_len, created_at, pinned, favorite, tags, source_app, sensitive, blob_path, width, height FROM items WHERE favorite = 1 OR pinned = 1 ORDER BY created_at DESC LIMIT 500", params![])
    }

    pub fn search(&self, query: &str, limit: u32) -> Result<Vec<Item>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let fts_query = query
            .split_whitespace()
            .map(|w| format!("\"{}\"", w.replace('"', "")))
            .collect::<Vec<_>>()
            .join(" ");
        let mut stmt = conn.prepare(
            "SELECT i.id, i.uuid, i.hash, i.kind, i.text, i.preview, i.byte_len, i.created_at, i.pinned, i.favorite, i.tags, i.source_app, i.sensitive, i.blob_path, i.width, i.height
             FROM items i JOIN items_fts f ON f.rowid = i.id WHERE items_fts MATCH ?1 ORDER BY rank LIMIT ?2",
        ).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![fts_query, limit as i64], row_to_item)
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        if out.is_empty() {
            drop(stmt);
            drop(conn);
            let like = format!("%{}%", query);
            return self.query_items("SELECT id, uuid, hash, kind, text, preview, byte_len, created_at, pinned, favorite, tags, source_app, sensitive, blob_path, width, height FROM items WHERE text LIKE ?1 OR preview LIKE ?1 ORDER BY created_at DESC LIMIT ?2", params![like, limit as i64]);
        }
        Ok(out)
    }

    fn query_items(&self, sql: &str, p: impl rusqlite::Params) -> Result<Vec<Item>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt.query_map(p, row_to_item).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn set_flag(&self, id: i64, column: &str, value: bool) -> Result<(), String> {
        let allowed = ["pinned", "favorite"];
        if !allowed.contains(&column) {
            return Err("invalid flag column".to_string());
        }
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = format!("UPDATE items SET {} = ?1 WHERE id = ?2", column);
        conn.execute(&sql, params![value as i32, id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_tags(&self, id: i64, tags: &[String]) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let raw = serde_json::to_string(tags).map_err(|e| e.to_string())?;
        conn.execute("UPDATE items SET tags = ?1 WHERE id = ?2", params![raw, id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_item(&self, id: i64) -> Result<(), String> {
        let item = self.get_item(id)?;
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM items WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        if let Some(item) = item {
            if let Some(blob) = item.blob_path {
                let path = self.data_dir.join(&blob);
                if still_referenced(&conn, &blob).unwrap_or(false) {
                    let _ = std::fs::remove_file(path);
                }
            }
        }
        Ok(())
    }

    pub fn clear_all(&self, keep_pinned: bool) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = if keep_pinned {
            "DELETE FROM items WHERE pinned = 0"
        } else {
            "DELETE FROM items"
        };
        let n = conn.execute(sql, []).map_err(|e| e.to_string())? as u64;
        Ok(n)
    }

    pub fn count(&self) -> Result<i64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .map_err(|e| e.to_string())
    }

    pub fn total_bytes(&self) -> Result<u64, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let v: i64 = conn
            .query_row("SELECT COALESCE(SUM(byte_len), 0) FROM items", [], |r| {
                r.get(0)
            })
            .map_err(|e| e.to_string())?;
        Ok(v.max(0) as u64)
    }

    pub fn list_activity(&self, limit: u32) -> Result<Vec<Activity>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, kind, detail, created_at FROM activity ORDER BY created_at DESC LIMIT ?1").map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok(Activity {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    detail: r.get(2)?,
                    created_at: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn prune_expired(&self, retention_days: u32) -> Result<u64, String> {
        if retention_days == 0 {
            return Ok(0);
        }
        let cutoff = now_unix() - (retention_days as i64) * 86400;
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let n = conn
            .execute(
                "DELETE FROM items WHERE created_at < ?1 AND pinned = 0 AND favorite = 0",
                params![cutoff],
            )
            .map_err(|e| e.to_string())? as u64;
        Ok(n)
    }

    pub fn enforce_limits(&self, max_items: u32, max_bytes: u64) -> Result<u64, String> {
        let mut removed = 0u64;
        let count = self.count()?;
        if count > max_items as i64 {
            let excess = count - max_items as i64;
            let conn = self.conn.lock().map_err(|e| e.to_string())?;
            removed += conn.execute("DELETE FROM items WHERE id IN (SELECT id FROM items WHERE pinned = 0 AND favorite = 0 ORDER BY created_at ASC LIMIT ?1)", params![excess]).map_err(|e| e.to_string())? as u64;
        }
        let mut total = self.total_bytes()?;
        while total > max_bytes {
            let oldest: Option<i64> = {
                let conn = self.conn.lock().map_err(|e| e.to_string())?;
                conn.query_row("SELECT id FROM items WHERE pinned = 0 AND favorite = 0 ORDER BY created_at ASC LIMIT 1", [], |r| r.get(0)).optional().map_err(|e| e.to_string())?
            };
            match oldest {
                Some(id) => {
                    self.delete_item(id)?;
                    removed += 1;
                    total = self.total_bytes()?;
                }
                None => break,
            }
        }
        Ok(removed)
    }

    pub fn blob_path(&self, hash: &str) -> PathBuf {
        self.data_dir.join("blobs").join(format!("{}.bin", hash))
    }

    pub fn write_blob(&self, hash: &str, bytes: &[u8]) -> Result<String, String> {
        let rel = format!("blobs/{}.bin", hash);
        let path = self.data_dir.join(&rel);
        if !path.exists() {
            std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
            }
        }
        Ok(rel)
    }

    pub fn read_blob(&self, rel: &str) -> Result<Vec<u8>, String> {
        std::fs::read(self.data_dir.join(rel)).map_err(|e| e.to_string())
    }

    pub fn export_json(&self) -> Result<String, String> {
        let items = self.list_items(100_000, 0)?;
        serde_json::to_string_pretty(&items).map_err(|e| e.to_string())
    }

    pub fn import_json(&self, raw: &str) -> Result<u64, String> {
        let items: Vec<Item> = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        let mut imported = 0u64;
        for item in items {
            let new = NewItem {
                hash: item.hash,
                kind: item.kind,
                text: item.text,
                preview: item.preview,
                byte_len: item.byte_len,
                source_app: item.source_app,
                sensitive: item.sensitive,
                blob_path: None,
                width: item.width,
                height: item.height,
            };
            if self.insert_item(&new)?.is_some() {
                imported += 1;
            }
        }
        Ok(imported)
    }
}

fn still_referenced(conn: &Connection, blob: &str) -> rusqlite::Result<bool> {
    let n: i64 = conn.query_row(
        "SELECT COUNT(*) FROM items WHERE blob_path = ?1",
        params![blob],
        |r| r.get(0),
    )?;
    Ok(n == 0)
}

fn row_to_item(r: &rusqlite::Row) -> rusqlite::Result<Item> {
    let tags_raw: String = r.get(10)?;
    let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
    Ok(Item {
        id: r.get(0)?,
        uuid: r.get(1)?,
        hash: r.get(2)?,
        kind: ItemKind::from_name(&r.get::<_, String>(3)?),
        text: r.get(4)?,
        preview: r.get(5)?,
        byte_len: r.get::<_, i64>(6)? as u64,
        created_at: r.get(7)?,
        pinned: r.get::<_, i64>(8)? != 0,
        favorite: r.get::<_, i64>(9)? != 0,
        tags,
        source_app: r.get(11)?,
        sensitive: r.get::<_, i64>(12)? != 0,
        blob_path: r.get(13)?,
        width: r.get::<_, Option<i64>>(14)?.map(|v| v as u32),
        height: r.get::<_, Option<i64>>(15)?.map(|v| v as u32),
    })
}

pub struct NewItem {
    pub hash: String,
    pub kind: ItemKind,
    pub text: Option<String>,
    pub preview: String,
    pub byte_len: u64,
    pub source_app: Option<String>,
    pub sensitive: bool,
    pub blob_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
