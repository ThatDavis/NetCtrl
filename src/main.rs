// NET CONTROL: A HAM Radio Net Check-in Logger written in Rust
// ratatui + crossterm + serde_json + chrono
//
// Keybindings (main view):
//   Tab      Switch focus Nets <-> Log
//   ↑ ↓      Navigate lists
//   Enter    Select / confirm
//   n        New net        e  Edit net / session / check-in (context)
//   c        Add check-in   d  Delete
//   x        Export log     p  Edit operator profile
//   Ctrl+Q   Quit

mod theme;
mod models;
mod persistence;
mod dialogs;
mod app;
mod input;
mod ui;

use std::{io, time::{Duration, Instant}};
use chrono::TimeZone;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::{App, Focus};
use dialogs::{CiDlg, CountdownState, FccResult, Modal};
use input::on_key;
use persistence::save_data;
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

fn update_countdown(app: &mut App) {
    let now = chrono::Local::now();

    // If a countdown is already active, check if it expired
    if let Some(ref cd) = app.countdown {
        if now >= cd.target {
            // Timer expired — activate the session
            let ni = cd.ni;
            let si = cd.si;
            if let Some(ses) = app.data.nets.get_mut(ni).and_then(|n| n.sessions.get_mut(si)) {
                ses.scheduled_time = None;
                save_data(&app.data);
            }
            // Auto-open the session and launch check-in dialog immediately
            app.net_ls.select(Some(ni));
            app.ses_ls.select(Some(si));
            app.focus = Focus::Log;
            app.modal = Modal::Ci(CiDlg::new());
            app.countdown = None;
        }
    }

    // Always look for the nearest scheduled session within 5 minutes
    // (allows switching to a nearer session if one appears)
    let mut nearest: Option<(chrono::DateTime<chrono::Local>, usize, usize)> = None;
    for (ni, net) in app.data.nets.iter().enumerate() {
        for (si, ses) in net.sessions.iter().enumerate() {
            if let Some(ref st) = ses.scheduled_time {
                if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(st, "%Y-%m-%d %H:%M") {
                    if let Some(dt) = chrono::Local.from_local_datetime(&ndt).single() {
                        let diff = dt.signed_duration_since(now);
                        if diff.num_milliseconds() > 0 && diff.num_seconds() <= 5 * 60 {
                            if let Some((curr_target, _, _)) = nearest {
                                if dt < curr_target {
                                    nearest = Some((dt, ni, si));
                                }
                            } else {
                                nearest = Some((dt, ni, si));
                            }
                        }
                    }
                }
            }
        }
    }

    match nearest {
        Some((target, ni, si)) => {
            let should_update = match &app.countdown {
                Some(cd) => target < cd.target,
                None => true,
            };
            if should_update {
                app.countdown = Some(CountdownState { target, ni, si });
            }
        }
        None => app.countdown = None,
    }
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
        if app.tick.elapsed() >= tick_rate {
            app.tick();
            app.tick = Instant::now();
            update_countdown(&mut app);
        }
        // Poll for FCC lookup results while check-in dialog is open
        if let Modal::Ci(ref mut d) = app.modal {
            if let Some(FccResult::Found(name)) = d.poll_fcc() {
                d.name = name;  // always fill; lookup result wins
            }
        }
    }
}
