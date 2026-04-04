use std::sync::Arc;

use crossbeam_channel::unbounded;

use search_engine_core::SearchEngine;

mod hotkey;
mod ui;

fn main() -> eframe::Result<()> {
    let engine = Arc::new(
        SearchEngine::open("search.db").expect("Failed to open search engine")
    );

    SearchEngine::build_index(&engine).expect("Failed to build index");

    let (tx, rx) = unbounded();

    hotkey::start_hotkey_thread(tx);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 200.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Launcher",
        options,
        Box::new(|_cc| Box::new(ui::LauncherApp::new(engine, rx))),
    )
}