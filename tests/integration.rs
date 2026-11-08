use clipvault::clipboard::{capture_image, capture_text};
use clipvault::privacy;
use clipvault::settings::Settings;
use clipvault::storage::{ItemKind, Store};
use std::sync::Arc;

fn test_store() -> (tempfile::TempDir, Arc<Store>) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Arc::new(Store::open(dir.path()).expect("open store"));
    (dir, store)
}

#[test]
fn capture_text_and_dedup() {
    let (_dir, store) = test_store();
    let settings = Settings::default();
    let first = capture_text(&store, &settings, "hello world".to_string()).unwrap();
    assert!(first.is_some());
    let second = capture_text(&store, &settings, "hello world".to_string()).unwrap();
    assert!(
        second.is_none(),
        "duplicate text must not create a second item"
    );
    assert_eq!(store.count().unwrap(), 1);
}

#[test]
fn capture_image_roundtrip() {
    let (_dir, store) = test_store();
    let settings = Settings::default();
    let rgba = vec![
        255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let id = capture_image(&store, &settings, 2, 2, rgba).unwrap();
    assert!(id.is_some());
    let item = store.get_item(id.unwrap()).unwrap().unwrap();
    assert_eq!(item.kind, ItemKind::Image);
    assert_eq!(item.width, Some(2));
    let bytes = store.read_blob(item.blob_path.as_ref().unwrap()).unwrap();
    let (w, h, decoded) = clipvault::clipboard::decode_png(&bytes).unwrap();
    assert_eq!((w, h), (2, 2));
    assert_eq!(decoded[0], 255);
}

#[test]
fn search_finds_text() {
    let (_dir, store) = test_store();
    let settings = Settings::default();
    capture_text(&store, &settings, "the quick brown fox".to_string()).unwrap();
    capture_text(&store, &settings, "unrelated sentence here".to_string()).unwrap();
    let results = store.search("quick brown", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].preview.contains("quick"));
}

#[test]
fn sensitive_content_withheld_by_default() {
    let (_dir, store) = test_store();
    let settings = Settings::default();
    assert!(!settings.allow_sensitive);
    let id = capture_text(
        &store,
        &settings,
        "api_key = sk-abcdefghij1234567890XYZ".to_string(),
    )
    .unwrap()
    .expect("redacted placeholder item");
    let item = store.get_item(id).unwrap().unwrap();
    assert_eq!(item.kind, ItemKind::Redacted);
    assert!(item.sensitive);
    assert!(!item.text.as_deref().unwrap_or("").contains("sk-"));
}

#[test]
fn sensitive_detection_patterns() {
    assert!(privacy::is_sensitive("ghp_abcdefghij1234567890"));
    assert!(privacy::is_sensitive("-----BEGIN RSA PRIVATE KEY-----"));
    assert!(privacy::is_sensitive("password: hunter2hunter"));
    assert!(!privacy::is_sensitive("buy milk and bread tomorrow"));
}

#[test]
fn pin_favorite_and_delete() {
    let (_dir, store) = test_store();
    let settings = Settings::default();
    let id = capture_text(&store, &settings, "keep this".to_string())
        .unwrap()
        .unwrap();
    store.set_flag(id, "pinned", true).unwrap();
    store.set_flag(id, "favorite", true).unwrap();
    let favs = store.list_favorites().unwrap();
    assert_eq!(favs.len(), 1);
    store.delete_item(id).unwrap();
    assert_eq!(store.count().unwrap(), 0);
}

#[test]
fn storage_limits_enforced() {
    let (_dir, store) = test_store();
    let settings = Settings::default();
    for i in 0..5 {
        capture_text(&store, &settings, format!("item number {}", i)).unwrap();
    }
    assert_eq!(store.count().unwrap(), 5);
    store.enforce_limits(3, 1024 * 1024).unwrap();
    assert_eq!(store.count().unwrap(), 3);
}

#[test]
fn expiry_prunes_old_items() {
    let (_dir, store) = test_store();
    let settings = Settings::default();
    let id = capture_text(&store, &settings, "old item".to_string())
        .unwrap()
        .unwrap();
    {
        let conn = rusqlite::Connection::open(_dir.path().join("clipvault.db")).unwrap();
        conn.execute(
            "UPDATE items SET created_at = 0 WHERE id = ?1",
            rusqlite::params![id],
        )
        .unwrap();
    }
    let removed = store.prune_expired(30).unwrap();
    assert_eq!(removed, 1);
    assert_eq!(store.count().unwrap(), 0);
}

#[test]
fn export_import_roundtrip() {
    let (_dir, store) = test_store();
    let settings = Settings::default();
    capture_text(&store, &settings, "export me please".to_string()).unwrap();
    let json = store.export_json().unwrap();
    let (dir2, store2) = test_store();
    let _ = dir2;
    let n = store2.import_json(&json).unwrap();
    assert_eq!(n, 1);
    assert_eq!(store2.count().unwrap(), 1);
}

#[test]
fn db_recovery_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let id = {
        let store = Store::open(dir.path()).unwrap();
        let settings = Settings::default();
        capture_text(&Arc::new(store), &settings, "survive reopen".to_string())
            .unwrap()
            .unwrap()
    };
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(store.integrity_check().unwrap(), "ok");
    assert!(store.get_item(id).unwrap().is_some());
}

#[test]
fn settings_roundtrip() {
    let (_dir, store) = test_store();
    let settings = Settings {
        retention_days: 7,
        allow_sensitive: true,
        ..Settings::default()
    };
    settings.save(&store).unwrap();
    let loaded = Settings::load(&store);
    assert_eq!(loaded.retention_days, 7);
    assert!(loaded.allow_sensitive);
}
