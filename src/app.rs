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
    toast: Option<(String, ToastKind, std::time::Instant)>,
    export_path: String,
    import_path: String,
    snippet_text: String,
    snippet_tag: String,
    page_started: std::time::Instant,
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
            page_started: std::time::Instant::now(),
            confirm_clear: false,
        }
    }

    fn set_toast(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toast = Some((text.into(), kind, std::time::Instant::now()));
    }

    /// Opacity of the current page's content: fades in over 180ms after a
    /// page switch, instant with reduced motion.
    fn page_fade(&self) -> f32 {
        crate::animation::page_fade(
            self.page_started.elapsed().as_secs_f32(),
            0.18,
            self.settings.reduced_motion,
        )
    }

    fn nav_icon(page: Page) -> &'static str {
        match page {
            Page::Dashboard => "▦",
            Page::History => "↺",
            Page::Favorites => "★",
            Page::Snippets => "✎",
            Page::Search => "⌕",
            Page::Activity => "◷",
            Page::Settings => "⚙",
        }
    }

    fn save_settings(&mut self) {
        if let Err(e) = self.settings.save(&self.store) {
            self.set_toast(format!("Could not save settings: {}", e), ToastKind::Error);
        }
    }
}

impl eframe::App for ClipVaultApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let p = ui::palette();
        let reduced = self.settings.reduced_motion;
        let sidebar_target = if self.sidebar_collapsed { 60.0 } else { 208.0 };
        let sidebar_width =
            crate::animation::animate_to(ctx, "sidebar-width", sidebar_target, 0.22, reduced);
        let fade = self.page_fade();

        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(sidebar_width)
            .show(ctx, |ui| {
                ui::paint_gradient_background(ui);
                ui.add_space(14.0);
                if self.sidebar_collapsed {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("CV")
                                .font(ui::heading_font(20.0))
                                .color(p.accent),
                        );
                    });
                } else {
                    ui.label(egui::RichText::new("ClipVault").font(ui::heading_font(22.0)));
                    ui.label(
                        egui::RichText::new("Clipboard history, kept private.")
                            .small()
                            .color(p.text_dim),
                    );
                }
                ui.add_space(10.0);
                if ui
                    .button(if self.sidebar_collapsed {
                        "›"
                    } else {
                        "‹ Collapse"
                    })
                    .clicked()
                {
                    self.sidebar_collapsed = !self.sidebar_collapsed;
                }
                ui.add_space(12.0);
                ui.separator();
                ui.add_space(6.0);
                for page in PAGES {
                    if ui::nav_button(
                        ui,
                        Self::nav_icon(page),
                        page.label(),
                        self.page == page,
                        self.sidebar_collapsed,
                    )
                    .clicked()
                    {
                        self.page = page;
                        self.page_started = std::time::Instant::now();
                    }
                    ui.add_space(2.0);
                }
                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);
                let paused = PAUSED.load(Ordering::Relaxed);
                let pause_label = if paused {
                    "▶ Resume monitoring"
                } else {
                    "❚❚ Pause monitoring"
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
                    ui.colored_label(
                        if paused { p.warn } else { p.accent },
                        if paused {
                            "○ Paused"
                        } else {
                            "● Monitoring"
                        },
                    );
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui::paint_gradient_background(ui);
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("ClipVault").font(ui::heading_font(16.0)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.monospace(format!("v{}", env!("CARGO_PKG_VERSION")));
                    let paused = PAUSED.load(Ordering::Relaxed);
                    ui.colored_label(
                        if paused { p.warn } else { p.accent },
                        if paused {
                            "○ Paused"
                        } else {
                            "● Monitoring"
                        },
                    );
                });
            });
            ui.add_space(4.0);
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.scope(|ui| {
                    ui.set_opacity(fade);
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
            });
        });

        // Animated toast, bottom-right: slides in, fades out after 4s.
        if let Some((text, kind, since)) = &self.toast {
            let age = since.elapsed().as_secs_f32();
            if age > 4.0 {
                self.toast = None;
            } else {
                let alpha = if age < 0.2 {
                    crate::animation::ease_out_cubic(age / 0.2)
                } else if age > 3.5 {
                    1.0 - (age - 3.5) / 0.5
                } else {
                    1.0
                };
                let slide = if reduced {
                    0.0
                } else {
                    (1.0 - crate::animation::ease_out_cubic(age / 0.25)) * 20.0
                };
                let text = text.clone();
                let kind = *kind;
                egui::Area::new("toast".into())
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0 + slide))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        ui.set_opacity(alpha.clamp(0.0, 1.0));
                        ui::toast(ui, &text, kind);
                    });
            }
        }

        // Repaint only while something is still moving.
        let sidebar_moving = (sidebar_width - sidebar_target).abs() > 0.5;
        let animating = sidebar_moving || fade < 1.0 || self.toast.is_some();
        if animating {
            ctx.request_repaint();
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
            ui::stat_card(ui, "Items stored", &count.to_string());
            ui::stat_card(ui, "Storage used", &format_bytes(bytes));
            ui::stat_card(ui, "Pinned or favorite", &favorites.to_string());
        });
        ui.add_space(12.0);
        if platform::wayland_limited() {
            ui.colored_label(ui::palette().warn, "Wayland detected: persistent clipboard capture is limited on Wayland. History may miss items copied while ClipVault is not the clipboard owner.");
        }
        ui.add_space(8.0);
        ui.label(platform::clipboard_backend_note());
        ui.add_space(16.0);
        ui.label(egui::RichText::new("Recent items").font(ui::heading_font(18.0)));
        ui.add_space(6.0);
        match self.store.list_items(8, 0) {
            Ok(items) if items.is_empty() => {
                ui.label(
                    "Nothing captured yet. Copy some text or an image and it will appear here.",
                );
            }
            Ok(items) => {
                for (i, item) in items.iter().enumerate() {
                    self.item_row(ui, item, i);
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

    fn item_row(&mut self, ui: &mut egui::Ui, item: &crate::storage::Item, index: usize) {
        let p = ui::palette();
        // Staggered entrance: each card fades in a beat after the one above
        // it, driven by the page fade.
        let fade = self.page_fade();
        let alpha =
            crate::animation::ease_out_cubic((fade * 1.4 - index as f32 * 0.08).clamp(0.0, 1.0));
        ui.scope(|ui| {
            ui.set_opacity(alpha);
            ui::card_frame(&p).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.monospace(format!("#{}  {}", item.id, item.kind.as_str()));
                    if item.pinned {
                        ui.colored_label(p.accent, "● Pinned");
                    }
                    if item.favorite {
                        ui.colored_label(p.accent, "★ Favorite");
                    }
                    if item.sensitive {
                        ui.colored_label(p.danger, "Sensitive");
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.monospace(format_time(item.created_at));
                    });
                });
                ui.add_space(4.0);
                ui.label(&item.preview);
                if !item.tags.is_empty() {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(format!("Tags: {}", item.tags.join(", ")))
                            .small()
                            .color(p.text_dim),
                    );
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let id = item.id;
                    if ui::primary_button(ui, "Copy back").clicked() {
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
        });
        ui.add_space(8.0);
    }

    fn show_history(&mut self, ui: &mut egui::Ui) {
        ui::section_title(ui, "Clipboard History");
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.text_edit_singleline(&mut self.history_filter);
            if ui.button("Refresh").clicked() {
                self.page_started = std::time::Instant::now();
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
                for (i, item) in items.iter().enumerate() {
                    self.item_row(ui, item, i);
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
                for (i, item) in items.iter().enumerate() {
                    self.item_row(ui, item, i);
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
                for (i, item) in items.into_iter().filter(|i| i.pinned).enumerate() {
                    self.item_row(ui, &item, i);
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
                self.page_started = std::time::Instant::now();
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
                for (i, item) in items.iter().enumerate() {
                    self.item_row(ui, item, i);
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
