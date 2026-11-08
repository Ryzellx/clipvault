use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub monitoring_enabled: bool,
    pub allow_sensitive: bool,
    pub retention_days: u32,
    pub max_items: u32,
    pub max_bytes: u64,
    pub capture_text: bool,
    pub capture_images: bool,
    pub poll_interval_ms: u64,
    pub reduced_motion: bool,
    pub excluded_apps: Vec<String>,
    pub start_minimized: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            monitoring_enabled: true,
            allow_sensitive: false,
            retention_days: 30,
            max_items: 1000,
            max_bytes: 512 * 1024 * 1024,
            capture_text: true,
            capture_images: true,
            poll_interval_ms: 500,
            reduced_motion: false,
            excluded_apps: Vec::new(),
            start_minimized: false,
        }
    }
}

impl Settings {
    pub fn load(store: &crate::storage::Store) -> Self {
        let mut settings = Self::default();
        if let Ok(Some(raw)) = store.get_setting("settings_json") {
            if let Ok(parsed) = serde_json::from_str::<Settings>(&raw) {
                settings = parsed;
            }
        }
        settings
    }

    pub fn save(&self, store: &crate::storage::Store) -> Result<(), String> {
        let raw = serde_json::to_string(self).map_err(|e| e.to_string())?;
        store
            .set_setting("settings_json", &raw)
            .map_err(|e| e.to_string())
    }
}
