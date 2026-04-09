use anyhow::Result;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use search_engine_core::SearchEngine;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

fn main() -> Result<()> {
    println!("[DAEMON] Starting indexer...");

    let engine = SearchEngine::open("search.db")?;
    println!("[DAEMON] Engine ready");

    // ✅ Proper "run once" flag
    if !Path::new("index.built").exists() {
        println!("[INDEX] Running initial full index...");
        engine.build_index()?;
        std::fs::write("index.built", "ok")?;
        println!("[INDEX] Done");
    } else {
        println!("[INDEX] Skipped (already built)");
    }

    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // ⚠️ safer scope (start small)
    watcher.watch(Path::new("C:\\"), RecursiveMode::Recursive)?;

    println!("[WATCHER] Watching filesystem...");

    let mut last_event = Instant::now();

    loop {
        match rx.recv() {
            Ok(event) => {
                let now = Instant::now();

                // ✅ soft debounce (not too aggressive)
                if now.duration_since(last_event) < Duration::from_millis(50) {
                    continue;
                }

                last_event = now;

                handle_event(&engine, event?);
            }
            Err(e) => println!("[ERROR] Watch error: {:?}", e),
        }
    }
}

fn handle_event(engine: &SearchEngine, event: Event) {
    println!("[EVENT] {:?}", event.kind);

    for path in event.paths {
        let path_str = path.to_string_lossy();

        // 🔥 ignore noisy/temp files
        if path_str.contains("~") || path_str.ends_with(".tmp") {
            continue;
        }

        match event.kind {
            EventKind::Create(_) => {
                if path.exists() && path.is_file() {
                    println!("[CREATE] {}", path_str);
                    let _ = engine.index_path(&path_str);
                }
            }

            EventKind::Modify(_) => {
                if path.exists() && path.is_file() {
                    println!("[MODIFY] {}", path_str);
                    let _ = engine.index_path(&path_str);
                }
            }

            EventKind::Remove(_) => {
                println!("[REMOVE] {}", path_str);
                let _ = engine.remove_path(&path_str);
            }

            _ => {}
        }
    }
}