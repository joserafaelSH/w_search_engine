use search_engine_core::SearchEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = match SearchEngine::open("search_engine.redb") {
        Ok(engine) => engine,
        Err(e) => panic!("Failed to open search engine: {}", e),
    };

    println!("Building index...");
    engine.build_index()?;
    let results = engine.search("chrome")?;

    if let Some(first) = results.first() {
        println!("First result: {}", first.file_name);
    }

    Ok(())
}
