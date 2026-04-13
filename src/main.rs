// NET CONTROL: A HAM Radio Net Check-in Logger written in Rust
// ratatui + crossterm + serde_json + chrono
//
// Keybindings (main view):
//   Tab      Switch focus Nets <-> Log
//   ↑ ↓      Navigate lists
//   Enter    Select / confirm
//   n        New net        e  Edit net
//   c        Add check-in   d  Delete
//   x        Export log     p  Edit operator profile
//   q        Quit

mod theme;
mod models;
mod persistence;
mod dialogs;
mod app;
mod input;
mod ui;

use std::{io, time::{Duration, Instant}};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;
use dialogs::{FccResult, Modal};
use input::on_key;
use ui::ui;

// ── Entry ─────────────────────────────────────────────────────────────────────
fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = io::stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut term = Terminal::new(backend)?;

    let res = run_loop(&mut term);

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;
    println!("73!");
    res
}

fn run_loop<B: ratatui::backend::Backend>(term: &mut Terminal<B>) -> io::Result<()> {
    let mut app = App::new();
    let tick_rate = Duration::from_millis(200);  // shorter for FCC polling
    loop {
        term.draw(|f| ui(f, &mut app))?;
        let timeout = tick_rate.checked_sub(app.tick.elapsed()).unwrap_or_default();
        if event::poll(timeout)? {
            if let Event::Key(k) = event::read()? {
                if k.kind == event::KeyEventKind::Press {
                    if !on_key(&mut app, k.code, k.modifiers) { return Ok(()); }
                }
            }
        }
        if app.tick.elapsed() >= tick_rate { app.tick(); app.tick = Instant::now(); }
        // Poll for FCC lookup results while check-in dialog is open
        if let Modal::Ci(ref mut d) = app.modal {
            if let Some(FccResult::Found(name)) = d.poll_fcc() {
                d.name = name;  // always fill; lookup result wins
            }
        }
    }
}
