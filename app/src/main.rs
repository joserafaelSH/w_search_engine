use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    layout::Constraint,
    style::{Style, Stylize},
    widgets::List,
    Frame, Terminal,
};
use search_engine_core::{SearchEngine, SearchResult};
use std::io;
use std::process::Command;

fn main() -> io::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let engine = SearchEngine::open("search.db").expect("Failed to open search database");
    run_app(engine, &mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    Ok(())
}

fn run_app(engine: SearchEngine, terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>) {
    let mut app = App::new(engine);
    app.run(terminal)
}

struct App {
    engine: SearchEngine,
    query: String,
    results: Vec<SearchResult>,
    selected_index: usize,
}

impl App {
    fn new(engine: SearchEngine) -> Self {
        Self {
            engine,
            query: String::new(),
            results: Vec::new(),
            selected_index: 0,
        }
    }

    fn run(&mut self, terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>) {
        loop {
            terminal.draw(|f| self.ui(f)).expect("Failed to draw");

            if let Event::Key(key) = event::read().expect("Failed to read event") {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char(c) => {
                        self.query.push(c);
                        self.search();
                    }
                    KeyCode::Backspace => {
                        self.query.pop();
                        self.search();
                    }
                    KeyCode::Down | KeyCode::Tab => {
                        if !self.results.is_empty() {
                            self.selected_index = (self.selected_index + 1) % self.results.len();
                        }
                    }
                    KeyCode::Up => {
                        if !self.results.is_empty() {
                            self.selected_index = if self.selected_index == 0 {
                                self.results.len() - 1
                            } else {
                                self.selected_index - 1
                            };
                        }
                    }
                    KeyCode::Enter => {
                        if !self.results.is_empty() {
                            let path = self.results[self.selected_index].path.clone();
                            Command::new("cmd")
                                .args(["/C", "start", "", &path])
                                .spawn()
                                .ok();
                        }
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    fn search(&mut self) {
        self.results = if self.query.is_empty() {
            Vec::new()
        } else {
            self.engine.search(&self.query).unwrap_or_default()
        };
        self.results.truncate(20);
        self.selected_index = 0;
    }

    fn ui(&self, f: &mut Frame) {
        let area = f.area();
        let height = area.height as usize;
        let search_height = 3u16;
        let input_height = 3u16;

        // Calculate available height for results
        let results_height = if height > (search_height + input_height) as usize {
            (height - (search_height + input_height) as usize) as u16
        } else {
            1
        };

        let chunks = ratatui::layout::Layout::default()
            .constraints([
                Constraint::Length(search_height),   // Search header
                Constraint::Length(results_height),  // Results list
                Constraint::Length(input_height),    // Input area
            ])
            .split(area);

        // Title
        let title = ratatui::widgets::Paragraph::new("🔍 Search")
            .style(Style::new().cyan().bold());
        f.render_widget(title, chunks[0]);

        // Results list
        if self.results.is_empty() {
            let placeholder = ratatui::widgets::Paragraph::new(
                if self.query.is_empty() {
                    "Start typing to search..."
                } else {
                    "No results found"
                },
            )
            .style(Style::new().dim());
            f.render_widget(placeholder, chunks[1]);
        } else {
            let items: Vec<ratatui::widgets::ListItem> = self
                .results
                .iter()
                .enumerate()
                .map(|(i, result)| {
                    let prefix = if i == self.selected_index { "❯ " } else { "  " };
                    let icon = if result.is_directory { "📁" } else { "📄" };
                    let style = if i == self.selected_index {
                        Style::new().yellow().bold()
                    } else {
                        Style::new().white()
                    };
                    ratatui::widgets::ListItem::new(format!("{}{} {}", prefix, icon, result.path))
                        .style(style)
                })
                .collect();

            let list = List::new(items)
                .block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .title(format!(" Results ({}) ", self.results.len())),
                );
            f.render_widget(list, chunks[1]);
        }

        // Input area
        let input_display = format!("> {}", self.query);
        let input = ratatui::widgets::Paragraph::new(input_display)
            .style(Style::new().green())
            .block(
                ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title(" Type to search "),
            );
        f.render_widget(input, chunks[2]);
    }
}
