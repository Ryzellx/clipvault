use crate::clipboard::{capture_text, restore_to_clipboard, PAUSED};
use crate::platform;
use crate::settings::Settings;
use crate::storage::Store;
use crate::{format_bytes, format_time};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "clipvault",
    version,
    about = "Privacy-first offline clipboard history manager"
)]
pub struct Cli {
    #[arg(long, global = true, help = "Data directory override")]
    pub data_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Status,
    History {
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    Search {
        query: String,
    },
    Copy {
        item_id: i64,
    },
    Pin {
        item_id: i64,
    },
    Delete {
        item_id: i64,
    },
    Clear {
        #[arg(long)]
        yes: bool,
    },
    Pause,
    Resume,
    Export {
        #[arg(long)]
        file: Option<PathBuf>,
    },
    Import {
        file: PathBuf,
    },
    Doctor,
}

pub fn run(cli: Cli) -> Result<i32, String> {
    let data_dir = cli.data_dir.clone().unwrap_or_else(crate::default_data_dir);
    let store = Arc::new(Store::open(&data_dir)?);
    let settings = Settings::load(&store);
    match cli.command {
        None => Ok(2),
        Some(Commands::Status) => {
            let count = store.count()?;
            let bytes = store.total_bytes()?;
            println!("ClipVault status");
            println!("  platform: {}", platform::platform_name());
            println!("  data dir: {}", data_dir.display());
            println!("  items: {}", count);
            println!("  storage used: {}", format_bytes(bytes));
            println!(
                "  monitoring: {}",
                if settings.monitoring_enabled && !PAUSED.load(Ordering::Relaxed) {
                    "on"
                } else {
                    "paused"
                }
            );
            println!(
                "  sensitive content stored: {}",
                if settings.allow_sensitive {
                    "allowed"
                } else {
                    "withheld by default"
                }
            );
            println!("  integrity: {}", store.integrity_check()?);
            if platform::wayland_limited() {
                println!("  note: Wayland detected, persistent clipboard capture is limited");
            }
            Ok(0)
        }
        Some(Commands::History { limit }) => {
            for item in store.list_items(limit, 0)? {
                println!(
                    "#{} [{}] {}  {}",
                    item.id,
                    item.kind.as_str(),
                    format_time(item.created_at),
                    item.preview
                );
            }
            Ok(0)
        }
        Some(Commands::Search { query }) => {
            let items = store.search(&query, 50)?;
            if items.is_empty() {
                println!("No results for \"{}\".", query);
            }
            for item in items {
                println!(
                    "#{} [{}] {}  {}",
                    item.id,
                    item.kind.as_str(),
                    format_time(item.created_at),
                    item.preview
                );
            }
            Ok(0)
        }
        Some(Commands::Copy { item_id }) => {
            println!("{}", restore_to_clipboard(&store, item_id)?);
            Ok(0)
        }
        Some(Commands::Pin { item_id }) => {
            let item = store
                .get_item(item_id)?
                .ok_or_else(|| format!("no item with id {}", item_id))?;
            store.set_flag(item_id, "pinned", !item.pinned)?;
            println!("Item {} pinned: {}", item_id, !item.pinned);
            Ok(0)
        }
        Some(Commands::Delete { item_id }) => {
            store.delete_item(item_id)?;
            println!("Deleted item {}", item_id);
            Ok(0)
        }
        Some(Commands::Clear { yes }) => {
            if !yes {
                return Err("refusing to clear without --yes".to_string());
            }
            let n = store.clear_all(true)?;
            println!("Deleted {} items (pinned items kept)", n);
            Ok(0)
        }
        Some(Commands::Pause) => {
            PAUSED.store(true, Ordering::Relaxed);
            store.set_setting("paused", "1")?;
            store.log_activity("pause", "Monitoring paused from CLI");
            println!("Monitoring paused");
            Ok(0)
        }
        Some(Commands::Resume) => {
            PAUSED.store(false, Ordering::Relaxed);
            store.set_setting("paused", "0")?;
            store.log_activity("pause", "Monitoring resumed from CLI");
            println!("Monitoring resumed");
            Ok(0)
        }
        Some(Commands::Export { file }) => {
            let json = store.export_json()?;
            match file {
                Some(path) => {
                    std::fs::write(&path, &json).map_err(|e| e.to_string())?;
                    println!("Exported to {}", path.display());
                }
                None => println!("{}", json),
            }
            Ok(0)
        }
        Some(Commands::Import { file }) => {
            let raw = std::fs::read_to_string(&file).map_err(|e| e.to_string())?;
            let n = store.import_json(&raw)?;
            println!("Imported {} items", n);
            Ok(0)
        }
        Some(Commands::Doctor) => {
            println!("ClipVault doctor");
            println!("  data dir exists: {}", data_dir.exists());
            println!("  database integrity: {}", store.integrity_check()?);
            println!("  items: {}", store.count()?);
            println!("  platform: {}", platform::platform_name());
            println!("  backend: {}", platform::clipboard_backend_note());
            match arboard::Clipboard::new() {
                Ok(_) => println!("  clipboard access: ok"),
                Err(e) => println!("  clipboard access: unavailable ({})", e),
            }
            Ok(0)
        }
    }
}

pub fn capture_stdin_text(
    store: &Arc<Store>,
    settings: &Settings,
    text: &str,
) -> Result<(), String> {
    capture_text(store, settings, text.to_string())?;
    Ok(())
}
