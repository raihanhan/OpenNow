use crate::hotkeys::{HotkeyAction, HotkeyManager};
use crate::models::{Shortcut, ShortcutStore};
use crate::tray::{create_tray_icon, TrayAction};
use crate::utils::launch_target;
use eframe::egui::{self, Context, ViewportCommand};
use egui_extras::{Column, TableBuilder};
use std::sync::mpsc;
use std::thread;

pub struct ShortcutManagerApp {
    store: ShortcutStore,
    hotkey_manager: HotkeyManager,
    _tray_icon: tray_icon::TrayIcon,
    tray_rx: mpsc::Receiver<TrayAction>,
    hotkey_rx: mpsc::Receiver<HotkeyAction>,

    // Main window state
    search_query: String,
    selected_category: String,
    selected_shortcut: Option<String>,
    show_add_dialog: bool,
    edit_shortcut: Option<Shortcut>,
    status_message: String,

    // Launcher popup state
    show_launcher: bool,
    launcher_search: String,
    launcher_selected: usize,
    launcher_results: Vec<Shortcut>,
}

impl ShortcutManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> anyhow::Result<Self> {
        // Setup custom fonts
        setup_fonts(&cc.egui_ctx);

        let store = ShortcutStore::new(None)?;

        // Create channels for cross-thread communication
        let (hotkey_tx, hotkey_rx) = mpsc::channel();
        let (tray_tx, tray_rx) = mpsc::channel();

        // Create hotkey manager
        let hotkey_manager = HotkeyManager::new(hotkey_tx)?;
        hotkey_manager.rebuild(&store.get_all())?;

        // Spawn hotkey event listener
        let hotkey_manager_clone = hotkey_manager.clone();
        thread::spawn(move || {
            loop {
                if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().recv() {
                    hotkey_manager_clone.handle_event(event);
                }
            }
        });

        // Create tray icon
        let (tray_icon, _tray_rx_internal) = create_tray_icon(tray_tx)?;

        Ok(Self {
            store,
            hotkey_manager,
            _tray_icon: tray_icon,
            tray_rx,
            hotkey_rx,
            search_query: String::new(),
            selected_category: "All".to_string(),
            selected_shortcut: None,
            show_add_dialog: false,
            edit_shortcut: None,
            status_message: String::new(),
            show_launcher: false,
            launcher_search: String::new(),
            launcher_selected: 0,
            launcher_results: Vec::new(),
        })
    }

    fn refresh_launcher_results(&mut self) {
        self.launcher_results = self.store.search(&self.launcher_search, None);
        self.launcher_selected = 0;
    }

    fn launch_shortcut(&mut self, shortcut: &Shortcut) {
        match launch_target(&shortcut.target, &shortcut.args) {
            Ok(_) => {
                let _ = self.store.bump_launch_count(&shortcut.id);
                self.status_message = format!("Launched: {}", shortcut.name);
            }
            Err(e) => {
                self.status_message = format!("Failed to launch {}: {}", shortcut.name, e);
            }
        }
    }

    fn handle_hotkey_action(&mut self, action: HotkeyAction) {
        match action {
            HotkeyAction::TogglePopup => {
                self.show_launcher = !self.show_launcher;
                if self.show_launcher {
                    self.launcher_search.clear();
                    self.refresh_launcher_results();
                }
            }
            HotkeyAction::LaunchShortcut(id) => {
                if let Some(shortcut) = self.store.get_by_id(&id) {
                    self.launch_shortcut(&shortcut);
                }
            }
        }
    }

    fn handle_tray_action(&mut self, action: TrayAction) {
        match action {
            TrayAction::Show => {
                // Window is already visible, just ensure it's focused
            }
            TrayAction::Quit => {
                std::process::exit(0);
            }
        }
    }
}

impl eframe::App for ShortcutManagerApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Process hotkey events
        while let Ok(action) = self.hotkey_rx.try_recv() {
            self.handle_hotkey_action(action);
        }

        // Process tray events
        while let Ok(action) = self.tray_rx.try_recv() {
            self.handle_tray_action(action);
        }

        // Show/hide main window based on launcher state
        if self.show_launcher {
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
        } else {
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        }

        // Main window
        if !self.show_launcher {
            self.show_main_window(ctx);
        }

        // Launcher popup
        if self.show_launcher {
            self.show_launcher_popup(ctx);
        }

        // Add/Edit dialog
        if self.show_add_dialog || self.edit_shortcut.is_some() {
            self.show_edit_dialog(ctx);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let _ = self.hotkey_manager.unregister_all();
    }
}

impl ShortcutManagerApp {
    fn show_main_window(&mut self, ctx: &Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Shortcut Manager");

            ui.separator();

            // Search and filter bar
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.text_edit_singleline(&mut self.search_query)
                    .on_hover_text("Type to filter shortcuts");
                ui.separator();
                egui::ComboBox::from_label("Category")
                    .selected_text(&self.selected_category)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.selected_category, "All".to_string(), "All");
                        for cat in self.store.categories() {
                            ui.selectable_value(&mut self.selected_category, cat.clone(), cat);
                        }
                    });
            });

            ui.separator();

            // Shortcuts table
            let shortcuts = self.store.search(
                &self.search_query,
                if self.selected_category == "All" {
                    None
                } else {
                    Some(&self.selected_category)
                },
            );

            let available_height = ui.available_height();
            egui::ScrollArea::vertical()
                .max_height(available_height - 60.0)
                .show(ui, |ui| {
                    TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .column(Column::auto().at_least(30.0)) // Icon
                        .column(Column::remainder().at_least(150.0)) // Name
                        .column(Column::auto().at_least(100.0)) // Category
                        .column(Column::auto().at_least(100.0)) // Hotkey
                        .column(Column::auto().at_least(80.0)) // Launch count
                        .header(20.0, |mut header| {
                            header.col(|ui| { ui.strong("#"); });
                            header.col(|ui| { ui.strong("Name"); });
                            header.col(|ui| { ui.strong("Category"); });
                            header.col(|ui| { ui.strong("Hotkey"); });
                            header.col(|ui| { ui.strong("Launches"); });
                        })
                        .body(|mut body| {
                            for (idx, shortcut) in shortcuts.iter().enumerate() {
                                let is_selected = self.selected_shortcut.as_ref() == Some(&shortcut.id);
                                body.row(24.0, |mut row| {
                                    row.set_selected(is_selected);

                                    row.col(|ui| {
                                        ui.label(format!("{}", idx + 1));
                                    });

                                    row.col(|ui| {
                                        let response = ui.selectable_label(is_selected, &shortcut.name);
                                        if response.clicked() {
                                            self.selected_shortcut = Some(shortcut.id.clone());
                                        }
                                        if response.double_clicked() {
                                            self.launch_shortcut(shortcut);
                                        }
                                    });

                                    row.col(|ui| {
                                        ui.label(&shortcut.category);
                                    });

                                    row.col(|ui| {
                                        if let Some(hk) = &shortcut.hotkey {
                                            ui.label(hk);
                                        } else {
                                            ui.label("-");
                                        }
                                    });

                                    row.col(|ui| {
                                        ui.label(format!("{}", shortcut.launch_count));
                                    });
                                });
                            }
                        });
                });

            ui.separator();

            // Action buttons
            ui.horizontal(|ui| {
                if ui.button("➕ Add").clicked() {
                    self.show_add_dialog = true;
                    self.edit_shortcut = None;
                }

                let can_edit = self.selected_shortcut.is_some();
                if ui.add_enabled(can_edit, egui::Button::new("✏️ Edit")).clicked() {
                    if let Some(id) = &self.selected_shortcut {
                        if let Some(s) = self.store.get_by_id(id) {
                            self.edit_shortcut = Some(s.clone());
                        }
                    }
                }

                if ui.add_enabled(can_edit, egui::Button::new("🗑 Delete")).clicked() {
                    if let Some(id) = &self.selected_shortcut {
                        let _ = self.store.delete(id);
                        let _ = self.hotkey_manager.rebuild(&self.store.get_all());
                        self.selected_shortcut = None;
                    }
                }

                if ui.add_enabled(can_edit, egui::Button::new("🚀 Launch")).clicked() {
                    if let Some(id) = &self.selected_shortcut {
                        if let Some(s) = self.store.get_by_id(id) {
                            self.launch_shortcut(&s);
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.status_message.is_empty() {
                        ui.label(&self.status_message);
                    }
                });
            });
        });
    }

    fn show_launcher_popup(&mut self, ctx: &Context) {
        egui::Window::new("Launcher")
            .fixed_pos([200.0, 150.0])
            .fixed_size([480.0, 360.0])
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .frame(egui::Frame::window(&ctx.style()).fill(ctx.style().visuals.window_fill()))
            .show(ctx, |ui| {
                ui.style_mut().visuals.window_fill = egui::Color32::from_rgb(0x20, 0x22, 0x25);

                // Search input
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.launcher_search)
                        .hint_text("Type to search shortcuts…")
                        .font(egui::FontId::proportional(18.0))
                        .desired_width(f32::INFINITY),
                );

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.show_launcher = false;
                    return;
                }

                if response.changed() {
                    self.refresh_launcher_results();
                }

                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(s) = self.launcher_results.get(self.launcher_selected) {
                        let shortcut = s.clone();
                        self.launch_shortcut(&shortcut);
                        self.show_launcher = false;
                    }
                }

                ui.add_space(8.0);

                // Results list
                let mut clicked_shortcut: Option<Shortcut> = None;
                egui::ScrollArea::vertical()
                    .max_height(280.0)
                    .show(ui, |ui| {
                        for (idx, shortcut) in self.launcher_results.iter().enumerate() {
                            let is_selected = idx == self.launcher_selected;
                            let response = ui.selectable_label(is_selected, format!("{}  [{}]", shortcut.name, shortcut.category));

                            if response.hovered() {
                                self.launcher_selected = idx;
                            }

                            if response.clicked() {
                                clicked_shortcut = Some(shortcut.clone());
                                self.show_launcher = false;
                            }
                        }

                        if self.launcher_results.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label("No shortcuts found");
                            });
                        }
                    });

                if let Some(s) = clicked_shortcut {
                    self.launch_shortcut(&s);
                }

                // Keyboard navigation
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    if self.launcher_selected + 1 < self.launcher_results.len() {
                        self.launcher_selected += 1;
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    if self.launcher_selected > 0 {
                        self.launcher_selected -= 1;
                    }
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.show_launcher = false;
                }
            });

        // Auto-focus search on first frame
        ctx.memory_mut(|mem| mem.request_focus(egui::Id::new("launcher_search")));
    }

    fn show_edit_dialog(&mut self, ctx: &Context) {
        let mut is_open = true;
        let is_new = self.edit_shortcut.is_none();

        // Clone the shortcut data to avoid borrow issues in closures
        let mut shortcut = self.edit_shortcut.clone().unwrap_or_default();

        egui::Window::new(if is_new { "Add Shortcut" } else { "Edit Shortcut" })
            .open(&mut is_open)
            .resizable(false)
            .collapsible(false)
            .fixed_size([420.0, 320.0])
            .show(ctx, |ui| {
                egui::Grid::new("edit_grid")
                    .num_columns(2)
                    .spacing([10.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut shortcut.name);
                        ui.end_row();

                        ui.label("Target:");
                        ui.horizontal(|ui| {
                            ui.text_edit_singleline(&mut shortcut.target);
                            if ui.button("Browse…").clicked() {
                                if let Some(path) = rfd::FileDialog::new().pick_file() {
                                    shortcut.target = path.display().to_string();
                                }
                            }
                        });
                        ui.end_row();

                        ui.label("Arguments:");
                        ui.text_edit_singleline(&mut shortcut.args);
                        ui.end_row();

                        ui.label("Category:");
                        egui::ComboBox::from_id_salt("category_combo")
                            .selected_text(&shortcut.category)
                            .show_ui(ui, |ui| {
                                for cat in self.store.categories() {
                                    ui.selectable_value(&mut shortcut.category, cat.clone(), cat);
                                }
                            });
                        ui.end_row();

                        ui.label("Hotkey:");
                        ui.horizontal(|ui| {
                            let mut hk = shortcut.hotkey.clone().unwrap_or_default();
                            ui.add(
                                egui::TextEdit::singleline(&mut hk)
                                    .hint_text("e.g. ctrl+alt+1"),
                            );
                            if hk.is_empty() {
                                shortcut.hotkey = None;
                            } else {
                                shortcut.hotkey = Some(hk);
                            }
                        });
                        ui.end_row();
                    });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        if shortcut.name.trim().is_empty() {
                            shortcut.name = "Untitled".to_string();
                        }
                        shortcut.category = shortcut.category.trim().to_string();
                        if shortcut.category.is_empty() {
                            shortcut.category = "General".to_string();
                        }

                        if is_new {
                            let _ = self.store.add(shortcut.clone());
                        } else {
                            let _ = self.store.update(shortcut.clone());
                        }

                        let _ = self.hotkey_manager.rebuild(&self.store.get_all());
                        self.show_add_dialog = false;
                        self.edit_shortcut = None;
                    }

                    if ui.button("Cancel").clicked() {
                        self.show_add_dialog = false;
                        self.edit_shortcut = None;
                    }
                });
            });

        if !is_open {
            self.show_add_dialog = false;
            self.edit_shortcut = None;
        }
    }
}

fn setup_fonts(ctx: &Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Use system fonts as fallback - no embedded font needed
    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            let segoe_path = format!("{}\\Fonts\\segoeui.ttf", windir);
            if std::path::Path::new(&segoe_path).exists() {
                if let Ok(font_data) = std::fs::read(&segoe_path) {
                    fonts.font_data.insert(
                        "segoe".to_owned(),
                        egui::FontData::from_owned(font_data).into(),
                    );
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "segoe".to_owned());
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .push("segoe".to_owned());
                    ctx.set_fonts(fonts);
                    return;
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let sf_path = "/System/Library/Fonts/SFNS.ttf";
        if std::path::Path::new(sf_path).exists() {
            if let Ok(font_data) = std::fs::read(sf_path) {
                fonts.font_data.insert(
                    "sf".to_owned(),
                    egui::FontData::from_owned(font_data).into(),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "sf".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("sf".to_owned());
                ctx.set_fonts(fonts);
                return;
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try common Linux font paths
        for path in [
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
        ] {
            if std::path::Path::new(path).exists() {
                if let Ok(font_data) = std::fs::read(path) {
                    fonts.font_data.insert(
                        "system".to_owned(),
                        egui::FontData::from_owned(font_data).into(),
                    );
                    fonts
                        .families
                        .entry(egui::FontFamily::Proportional)
                        .or_default()
                        .insert(0, "system".to_owned());
                    fonts
                        .families
                        .entry(egui::FontFamily::Monospace)
                        .or_default()
                        .push("system".to_owned());
                    ctx.set_fonts(fonts);
                    return;
                }
            }
        }
    }

    // Fallback to default egui fonts
    ctx.set_fonts(fonts);
}