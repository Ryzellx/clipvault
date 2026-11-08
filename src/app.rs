use crate::clipboard::{restore_to_clipboard, PAUSED};
use crate::platform;
use crate::settings::Settings;
use crate::storage::Store;
use crate::ui::{self, ToastKind};
use crate::{format_bytes, format_time};
use std::sync::atomic::Ordering;
use std::sync::Arc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Dashboard,
    History,
    Favorites,
    Snippets,
    Search,
    Activity,
    Settings,
}

impl Page {
    fn label(&self) -> &'static str {
        match self {
            Page::Dashboard => "Dashboard",
            Page::History => "History",
            Page::Favorites => "Favorites",
            Page::Snippets => "Snippets",
            Page::Search => "Search",
            Page::Activity => "Activity",
            Page::Settings => "Settings",
        }
    }
}

const PAGES: [Page; 7] = [
    Page::Dashboard,
    Page::History,
    Page::Favorites,
    Page::Snippets,
    Page::Search,
    Page::Activity,
    Page::Settings,
];

pub struct ClipVaultApp {
    store: Arc<Store>,
    settings: Settings,
    page: Page,
    sidebar_collapsed: bool,
    search_query: String,
    history_filter: String,
    tag_input: String,
    selected_id: Option<i64>,
    toast: Option<(String, ToastKind, f64)>,
    export_path: String,
    import_path: String,
    snippet_text: String,
    snippet_tag: String,
    page_anim: f32,
    confirm_clear: bool,
}

impl ClipVaultApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, store: Arc<Store>, settings: Settings) -> Self {
        ui::apply_theme(&_cc.egui_ctx);
        Self {
            store,
            settings,
            page: Page::Dashboard,
            sidebar_collapsed: false,
            search_query: String::new(),
            history_filter: String::new(),
            tag_input: String::new(),
            selected_id: None,
            toast: None,
            export_path: String::new(),
            import_path: String::new(),
            snippet_text: String::new(),
            snippet_tag: String::new(),
            page_anim: 0.0,
            confirm_clear: false,
        }
    }

    fn set_toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toast = Some((text.into(), kind, crate::now_unix() as f64));
    }

    fn save_settings(&mut self) {
        if let Err(e) = self.settings.save(&self.store) {
            self.set_toast(format!("Could not save settings: {}", e), ToastKind::Error);
        }
    }
}

impl eframe::App for ClipVaultApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = ctx.input(|i| i.stable_dt).min(0.05);
        let target = 1.0f32;
        let speed = if self.settings.reduced_motion {
            100.0
        } else {
            8.0
        };
        self.page_anim += (target - self.page_anim) * (dt * speed).min(1.0);

        let p = ui::palette();
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(if self.sidebar_collapsed { 56.0 } else { 200.0 })
            .show(ctx, |ui| {
                ui.add_space(12.0);
                if self.sidebar_collapsed {
                    ui.heading("CV");
                } else {
                    ui.heading("ClipVault");
                    ui.label("Clipboard history, kept private.");
                }
                ui.add_space(8.0);
                if ui
                    .button(if self.sidebar_collapsed {
                        ">"
                    } else {
                        "< Collapse"
                    })
                    .clicked()
                {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                }
                ui.add_space(12.0);
                ui.separator();
                for page in PAGES {
                    let selected = self.page == page;
                    let label = if self.sidebar_collapsed {
                        page.label().chars().next().unwrap_or('?').to_string()
                    } else {
                        page.label().to_string()
                    };
                    let btn = egui::Button::new(label)
                        .selected(selected)
                        .min_size(egui::vec2(0.0, 32.0));
                    if ui.add(btn).clicked() {
                        self.page = page;
                        self.page_anim = 0.0;
                    }
                }
                ui.add_space(16.0);
                ui.separator();
                let paused = PAUSED.load(Ordering::Relaxed);
                let pause_label = if paused {
                    "Resume monitoring"
                } else {
                    "Pause monitoring"
                };
                if ui.button(pause_label).clicked() {
                    PAUSED.store(!paused, Ordering::Relaxed);
                    self.store.log_activity(
                        "pause",
                        if paused {
                            "Monitoring resumed"
                        } else {
                            "Monitoring paused"
                        },
                    );
                    self.set_toast(
                        if paused {
                            "Monitoring resumed"
                        } else {
                            "Monitoring paused"
                        },
                        ToastKind::Info,
                    );
                }
                if !self.sidebar_collapsed {
                    ui.add_space(8.0);
                    ui.colored_label(p.text_dim, if paused { "Paused" } else { "Monitoring" });
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(8.0);
                match self.page {
                    Page::Dashboard => self.show_dashboard(ui),
                    Page::History => self.show_history(ui),
                    Page::Favorites => self.show_favorites(ui),
                    Page::Snippets => self.show_snippets(ui),
                    Page::Search => self.show_search(ui),
                    Page::Activity => self.show_activity(ui),
                    Page::Settings => self.show_settings(ui),
                }
            });
            if let Some((text, kind, _)) = &self.toast {
                ui::toast(ui, text, *kind);
            }
        });

        if !self.settings.reduced_motion {
            ctx.request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
}

impl ClipVaultApp {
    fn show_dashboard(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Dashboard");
        let count = self.store.count().unwrap_or(0);
        let bytes = self.store.total_bytes().unwrap_or(0);
        let favorites = self.store.list_favorites().map(|v| v.len()).unwrap_or(0);
        ui.horizontal(|ui| {
            ui.label(format!("Items stored: {}", count));
            ui.label(format!("Storage used: {}", format_bytes(bytes)));
            ui.label(format!("Pinned or favorite: {}", favorites));
        });
        ui.add_space(12.0);
        if platform::wayland_limited() {
            ui.colored_label(ui::palette().danger, "Wayland detected: persistent clipboard capture is limited on Wayland. History may miss items copied while ClipVault is not the clipboard owner.");
        }
        ui.add_space(8.0);
        ui.label(platform::clipboard_backend_note());
        ui.add_space(16.0);
        ui.heading("Recent items");
        match self.store.list_items(8, 0) {
            Ok(items) if items.is_empty() => {
                ui.label(
                    "Nothing captured yet. Copy some text or an image and it will appear here.",
                );
            }
            Ok(items) => {
                for item in items {
                    self.item_row(ui, &item);
                }
            }
            Err(e) => {
                ui.colored_label(
                    ui::palette().danger,
                    format!("Could not load history: {}", e),
                );
            }
        }
    }

    fn item_row(&mut self, ui: &mut egui::Ui, item: &crate::storage::Item) {
        let p = ui::palette();
        egui::Frame::new()
            .fill(p.panel_alt)
            .stroke(egui::Stroke::new(1.0f32, p.border))
            .corner_radius(6)
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("#{}  {}", item.id, item.kind.as_str()));
                    if item.pinned {
                        ui.colored_label(p.accent, "Pinned");
                    }
                    if item.favorite {
                        ui.colored_label(p.accent, "Favorite");
                    }
                    if item.sensitive {
                        ui.colored_label(p.danger, "Sensitive");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format_time(item.created_at));
                    });
                });
                ui.label(&item.preview);
                if !item.tags.is_empty() {
                    ui.label(format!("Tags: {}", item.tags.join(", ")));
                }
                ui.horizontal(|ui| {
                    let id = item.id;
                    if ui.button("Copy back").clicked() {
                        match restore_to_clipboard(&self.store, id) {
                            Ok(msg) => self.set_toast(msg, ToastKind::Ok),
                            Err(e) => self.set_toast(e, ToastKind::Error),
                        }
                    }
                    if ui
                        .button(if item.pinned { "Unpin" } else { "Pin" })
                        .clicked()
                    {
                        let _ = self.store.set_flag(id, "pinned", !item.pinned);
                    }
                    if ui
                        .button(if item.favorite {
                            "Unfavorite"
                        } else {
                            "Favorite"
                        })
                        .clicked()
                    {
                        let _ = self.store.set_flag(id, "favorite", !item.favorite);
                    }
                    if ui.button("Delete").clicked() {
                        let _ = self.store.delete_item(id);
                        self.set_toast(format!("Deleted item {}", id), ToastKind::Info);
                    }
                    if ui.button("Select").clicked() {
                        self.selected_id = Some(id);
                    }
                });
            });
        ui.add_space(6.0);
    }

    fn show_history(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Clipboard History");
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.history_filter);
            if ui.button("Refresh").clicked() {
                self.page_anim = 0.0;
            }
        });
        ui.add_space(8.0);
        match self.store.list_items(100, 0) {
            Ok(items) => {
                let items: Vec<_> = items
                    .into_iter()
                    .filter(|it| {
                        self.history_filter.is_empty()
                            || it
                                .preview
                                .to_lowercase()
                                .contains(&self.history_filter.to_lowercase())
                    })
                    .collect();
                if items.is_empty() {
                    ui.label("No items match. Copy something, or clear the filter.");
                }
                for item in items {
                    self.item_row(ui, &item);
                }
            }
            Err(e) => {
                ui.colored_label(
                    ui::palette().danger,
                    format!("Could not load history: {}", e),
                );
            }
        }
        ui.add_space(12.0);
        if self.confirm_clear {
            ui.horizontal(|ui| {
                ui.label("Delete all unpinned history? This cannot be undone.");
                if ui.button("Yes, delete all").clicked() {
                    match self.store.clear_all(true) {
                        Ok(n) => self.set_toast(format!("Deleted {} items", n), ToastKind::Ok),
                        Err(e) => self.set_toast(e, ToastKind::Error),
                    }
                    self.confirm_clear = false;
                }
                if ui.button("Cancel").clicked() {
                    self.confirm_clear = false;
                }
            });
        } else if ui.button("Clear history (keep pinned)").clicked() {
            self.confirm_clear = true;
        }
    }

    fn show_favorites(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Favorites");
        match self.store.list_favorites() {
            Ok(items) if items.is_empty() => {
                ui.label("No favorites yet. Pin or favorite an item from History and it will show up here.");
            }
            Ok(items) => {
                for item in items {
                    self.item_row(ui, &item);
                }
            }
            Err(e) => {
                ui.colored_label(
                    ui::palette().danger,
                    format!("Could not load favorites: {}", e),
                );
            }
        }
    }

    fn show_snippets(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Saved Snippets");
        ui.label("Snippets are clipboard items you keep on purpose. Save the text below as a new pinned item.");
        ui.add_space(8.0);
        ui.label("Snippet text:");
        ui.text_edit_multiline(&mut self.snippet_text);
        ui.label("Tag (optional):");
        ui.text_edit_singleline(&mut self.snippet_tag);
        if ui.button("Save snippet").clicked() {
            if self.snippet_text.trim().is_empty() {
                self.set_toast("Snippet text is empty.", ToastKind::Error);
            } else {
                let text = self.snippet_text.clone();
                match crate::clipboard::capture_text(&self.store, &self.settings, text) {
                    Ok(Some(id)) => {
                        let _ = self.store.set_flag(id, "pinned", true);
                        if !self.snippet_tag.trim().is_empty() {
                            let _ = self
                                .store
                                .set_tags(id, &[self.snippet_tag.trim().to_string()]);
                        }
                        self.snippet_text.clear();
                        self.snippet_tag.clear();
                        self.set_toast(format!("Snippet saved as item {}", id), ToastKind::Ok);
                    }
                    Ok(None) => self.set_toast("That text is already saved.", ToastKind::Info),
                    Err(e) => self.set_toast(e, ToastKind::Error),
                }
            }
        }
        ui.add_space(16.0);
        ui.heading("Pinned snippets");
        match self.store.list_favorites() {
            Ok(items) => {
                for item in items.into_iter().filter(|i| i.pinned) {
                    self.item_row(ui, &item);
                }
            }
            Err(e) => {
                ui.colored_label(
                    ui::palette().danger,
                    format!("Could not load snippets: {}", e),
                );
            }
        }
    }

    fn show_search(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Search");
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut self.search_query);
            if ui.button("Search").clicked() {
                self.page_anim = 0.0;
            }
        });
        ui.add_space(8.0);
        if self.search_query.trim().is_empty() {
            ui.label("Type a word or phrase to search captured text. Search covers text items only; images are found by their size label.");
            return;
        }
        match self.store.search(&self.search_query, 100) {
            Ok(items) if items.is_empty() => {
                ui.label(format!("No results for \"{}\".", self.search_query));
            }
            Ok(items) => {
                ui.label(format!("{} result(s).", items.len()));
                for item in items {
                    self.item_row(ui, &item);
                }
            }
            Err(e) => {
                ui.colored_label(ui::palette().danger, format!("Search failed: {}", e));
            }
        }
    }

    fn show_activity(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Activity");
        match self.store.list_activity(100) {
            Ok(entries) if entries.is_empty() => {
                ui.label("No activity yet. Captures, restores, and pauses are recorded here.");
            }
            Ok(entries) => {
                for entry in entries {
                    ui.label(format!(
                        "{}  {}  {}",
                        format_time(entry.created_at),
                        entry.kind,
                        entry.detail
                    ));
                }
            }
            Err(e) => {
                ui.colored_label(
                    ui::palette().danger,
                    format!("Could not load activity: {}", e),
                );
            }
        }
    }

    fn show_settings(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Settings");
        ui.checkbox(&mut self.settings.monitoring_enabled, "Monitor clipboard");
        ui.checkbox(&mut self.settings.capture_text, "Capture text");
        ui.checkbox(&mut self.settings.capture_images, "Capture images");
        ui.checkbox(&mut self.settings.allow_sensitive, "Store sensitive content (off by default; sensitive items are withheld and only their hash is kept)");
        ui.checkbox(&mut self.settings.reduced_motion, "Reduce motion");
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Keep history for (days, 0 keeps everything):");
            ui.add(egui::DragValue::new(&mut self.settings.retention_days).range(0..=365));
        });
        ui.horizontal(|ui| {
            ui.label("Maximum items:");
            ui.add(egui::DragValue::new(&mut self.settings.max_items).range(10..=100000));
        });
        ui.horizontal(|ui| {
            ui.label("Maximum storage (MB):");
            let mut mb = self.settings.max_bytes / (1024 * 1024);
            if ui
                .add(egui::DragValue::new(&mut mb).range(16..=8192))
                .changed()
            {
                self.settings.max_bytes = mb * 1024 * 1024;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Poll interval (ms):");
            ui.add(egui::DragValue::new(&mut self.settings.poll_interval_ms).range(200..=5000));
        });
        ui.add_space(8.0);
        ui.label("Exclude applications (process names, one per line, comma separated):");
        let mut excluded = self.settings.excluded_apps.join(", ");
        if ui.text_edit_singleline(&mut excluded).changed() {
            self.settings.excluded_apps = excluded
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if ui.button("Save settings").clicked() {
            self.save_settings();
            self.set_toast("Settings saved.", ToastKind::Ok);
        }
        ui.add_space(16.0);
        ui.heading("Data");
        ui.label(format!("Data folder: {}", self.store.data_dir.display()));
        ui.horizontal(|ui| {
            ui.label("Export path:");
            ui.text_edit_singleline(&mut self.export_path);
            if ui.button("Export JSON").clicked() {
                match self.store.export_json() {
                    Ok(json) => {
                        let path = if self.export_path.trim().is_empty() {
                            self.store.data_dir.join("export.json")
                        } else {
                            std::path::PathBuf::from(self.export_path.trim())
                        };
                        match std::fs::write(&path, json) {
                            Ok(()) => self.set_toast(
                                format!("Exported to {}", path.display()),
                                ToastKind::Ok,
                            ),
                            Err(e) => self.set_toast(e.to_string(), ToastKind::Error),
                        }
                    }
                    Err(e) => self.set_toast(e, ToastKind::Error),
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Import path:");
            ui.text_edit_singleline(&mut self.import_path);
            if ui.button("Import JSON").clicked() {
                match std::fs::read_to_string(self.import_path.trim()) {
                    Ok(raw) => match self.store.import_json(&raw) {
                        Ok(n) => self.set_toast(format!("Imported {} items", n), ToastKind::Ok),
                        Err(e) => self.set_toast(e, ToastKind::Error),
                    },
                    Err(e) => self.set_toast(e.to_string(), ToastKind::Error),
                }
            }
        });
        ui.add_space(8.0);
        if ui.button("Run storage cleanup now").clicked() {
            match self.store.prune_expired(self.settings.retention_days) {
                Ok(n) => self.set_toast(format!("Removed {} expired items", n), ToastKind::Ok),
                Err(e) => self.set_toast(e, ToastKind::Error),
            }
        }
        if ui.button("Check database integrity").clicked() {
            match self.store.integrity_check() {
                Ok(result) => self.set_toast(format!("Integrity check: {}", result), ToastKind::Ok),
                Err(e) => self.set_toast(e, ToastKind::Error),
            }
        }
        ui.add_space(8.0);
        ui.label("Selected item tags:");
        ui.text_edit_singleline(&mut self.tag_input);
        if ui.button("Apply tags to selected item").clicked() {
            if let Some(id) = self.selected_id {
                let tags: Vec<String> = self
                    .tag_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                match self.store.set_tags(id, &tags) {
                    Ok(()) => self.set_toast("Tags saved.", ToastKind::Ok),
                    Err(e) => self.set_toast(e, ToastKind::Error),
                }
            } else {
                self.set_toast(
                    "Select an item first (use Select on its card).",
                    ToastKind::Error,
                );
            }
        }
    }
}
