use std::sync::Arc;

use crossbeam_channel::Receiver;
use eframe::egui;

use search_engine_core::{SearchEngine, SearchResult};

pub struct LauncherApp {
    engine: Arc<SearchEngine>,
    rx: Receiver<()>,

    query: String,
    results: Vec<SearchResult>,
    selected: usize,

    visible: bool,
    focus_input: bool,
}

impl LauncherApp {
    pub fn new(engine: Arc<SearchEngine>, rx: Receiver<()>) -> Self {
        Self {
            engine,
            rx,
            query: String::new(),
            results: vec![],
            selected: 0,
            visible: false,
            focus_input: false,
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 🔥 HOTKEY TOGGLE
        if self.rx.try_recv().is_ok() {
            self.visible = !self.visible;

            if self.visible {
                self.focus_input = true;
                self.query.clear();
                self.results.clear();
                self.selected = 0;

                // 🔥 bring window to front
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        if !self.visible {
            return;
        }

        // 🔥 center window (like Spotlight)
        let screen_rect = ctx.input(|i| i.screen_rect());
        let width = 600.0;
        let height = 200.0;

        let pos = egui::pos2(
            (screen_rect.width() - width) / 2.0,
            screen_rect.height() * 0.2,
        );

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {

                egui::Frame::dark_canvas(ui.style())
                    .rounding(12.0)
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {

                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.query)
                                .hint_text("Search...")
                                .desired_width(f32::INFINITY),
                        );

                        // 🔥 auto focus
                        if self.focus_input {
                            response.request_focus();
                            self.focus_input = false;
                        }

                        // 🔍 search
                        if response.changed() {
                            self.results = self.engine.search(&self.query).unwrap_or_default();
                            self.selected = 0;
                        }

                        // ⌨️ navigation
                        if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                            if self.selected + 1 < self.results.len() {
                                self.selected += 1;
                            }
                        }

                        if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                            if self.selected > 0 {
                                self.selected -= 1;
                            }
                        }

                        // 🚀 open
                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            if let Some(item) = self.results.get(self.selected) {
                                let _ = self.engine.open_path(&item.path);
                            }
                            self.visible = false;
                        }

                        // ❌ close
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.visible = false;
                        }

                        ui.add_space(8.0);

                        // 📄 results
                        for (i, item) in self.results.iter().take(6).enumerate() {
                            let selected = i == self.selected;

                            let text = if selected {
                                format!("▶ {}", item.path)
                            } else {
                                format!("  {}", item.path)
                            };

                            ui.label(text);
                        }
                    });
            });
        });

        ctx.request_repaint();
    }
}