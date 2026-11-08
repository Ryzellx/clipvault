use clap::Parser;
use clipvault::app::ClipVaultApp;
use clipvault::cli::{run, Cli};
use clipvault::clipboard::start_monitor;
use clipvault::settings::Settings;
use clipvault::storage::Store;
use std::sync::Arc;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let cli = Cli::parse();
    if cli.command.is_some() {
        match run(cli) {
            Ok(code) => std::process::exit(code),
            Err(e) => {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        }
    }
    let data_dir = cli
        .data_dir
        .clone()
        .unwrap_or_else(clipvault::default_data_dir);
    let store = match Store::open(&data_dir) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!(
                "error: could not open ClipVault data at {}: {}",
                data_dir.display(),
                e
            );
            std::process::exit(1);
        }
    };
    let settings = Settings::load(&store);
    if settings.monitoring_enabled {
        let _ = store.prune_expired(settings.retention_days);
        let _ = store.enforce_limits(settings.max_items, settings.max_bytes);
    }
    start_monitor(Arc::clone(&store), settings.clone());
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1020.0, 680.0])
            .with_title("ClipVault"),
        ..Default::default()
    };
    let result = eframe::run_native(
        "ClipVault",
        options,
        Box::new(move |cc| Ok(Box::new(ClipVaultApp::new(cc, store, settings)))),
    );
    if let Err(e) = result {
        eprintln!(
            "error: ClipVault window could not start (no display available?): {}",
            e
        );
        std::process::exit(1);
    }
}
