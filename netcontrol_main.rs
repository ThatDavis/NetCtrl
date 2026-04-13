// NET CONTROL — HAM Radio Net Check-in Logger (Rust)
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

use std::{
    io,
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};
use chrono::Utc;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};

// ── Base16 Theme system ───────────────────────────────────────────────────────
//
// Base16 slots:
//   base00  darkest background        base08  red   / delete
//   base01  darker background         base09  orange / amber
//   base02  selection / highlight bg  base0A  yellow
//   base03  comments / dim text       base0B  green
//   base04  dark foreground           base0C  cyan / sapphire
//   base05  default foreground        base0D  blue
//   base06  light foreground          base0E  mauve / purple
//   base07  lightest foreground       base0F  brown / pink accent
//
// Semantic mapping used by netcontrol:
//   bg       = base00   (background)
//   bg_alt   = base01   (slightly lighter bg, unused visually but kept for compat)
//   sel_bg   = base02   (selected item background)
//   dim      = base03   (dim/comments)
//   border   = base04   (normal borders)
//   fg       = base05   (normal text)
//   border_f = base06   (focused border / heading accent)  -- we use base0E
//   bright   = base07   (unused)
//   red      = base08
//   amber    = base09   (frequency, accents)
//   yellow   = base0A   (timestamps)
//   green    = base0B   (net names, ok states)
//   cyan     = base0C   (field labels)
//   blue     = base0D   (info values)
//   mauve    = base0E   (bold/heading fg, focused border)
//   pink     = base0F   (club names, operator dialog)

fn parse_hex_color(h: &str) -> Color {
    let h = h.trim_start_matches('#');
    if h.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&h[0..2], 16),
                                        u8::from_str_radix(&h[2..4], 16),
                                        u8::from_str_radix(&h[4..6], 16),
        ) {
            return Color::Rgb(r, g, b);
        }
    }
    Color::Reset
}

#[derive(Debug, Clone)]
struct Theme {
    name:   String,
    // Base16 slots
    base00: Color, base01: Color, base02: Color, base03: Color,
    base04: Color, base05: Color, base06: Color, base07: Color,
    base08: Color, base09: Color, base0a: Color, base0b: Color,
    base0c: Color, base0d: Color, base0e: Color, base0f: Color,
}

impl Theme {
    // ── Style helpers ──────────────────────────────────────────────────────
    fn bg(&self)     -> Color { self.base00 }
    fn fg(&self)     -> Color { self.base05 }
    fn dim_c(&self)  -> Color { self.base03 }
    fn sel_bg(&self) -> Color { self.base02 }
    fn bord_c(&self) -> Color { self.base04 }
    fn bord_f(&self) -> Color { self.base0e }
    fn accent(&self) -> Color { self.base0e } // mauve/purple — bold/headings
    fn red(&self)    -> Color { self.base08 }
    fn amber(&self)  -> Color { self.base09 }
    fn yellow(&self) -> Color { self.base0a }
    fn green(&self)  -> Color { self.base0b }
    fn cyan(&self)   -> Color { self.base0c }
    fn blue(&self)   -> Color { self.base0d }
    fn pink(&self)   -> Color { self.base0f }

    // ── Named styles ───────────────────────────────────────────────────────
    fn normal(&self)  -> Style { Style::default().fg(self.fg()).bg(self.bg()) }
    fn dim(&self)     -> Style { Style::default().fg(self.dim_c()).bg(self.bg()) }
    fn bold(&self)    -> Style { Style::default().fg(self.accent()).bg(self.bg()).add_modifier(Modifier::BOLD) }
    fn sel(&self)     -> Style { Style::default().fg(self.bg()).bg(self.accent()).add_modifier(Modifier::BOLD) }
    fn sel2(&self)    -> Style { Style::default().fg(self.fg()).bg(self.sel_bg()).add_modifier(Modifier::BOLD) }
    fn amber_s(&self) -> Style { Style::default().fg(self.amber()).bg(self.bg()).add_modifier(Modifier::BOLD) }
    fn cyan_s(&self)  -> Style { Style::default().fg(self.cyan()).bg(self.bg()) }
    fn red_s(&self)   -> Style { Style::default().fg(self.red()).bg(self.bg()).add_modifier(Modifier::BOLD) }
    fn hdr(&self)     -> Style { Style::default().fg(self.bg()).bg(self.accent()).add_modifier(Modifier::BOLD) }
    fn call(&self)    -> Style { Style::default().fg(self.cyan()).bg(self.bg()).add_modifier(Modifier::BOLD) }
    fn time_s(&self)  -> Style { Style::default().fg(self.yellow()).bg(self.bg()) }
    fn green_s(&self) -> Style { Style::default().fg(self.green()).bg(self.bg()) }
    fn blue_s(&self)  -> Style { Style::default().fg(self.blue()).bg(self.bg()) }
    fn border(&self)  -> Style { Style::default().fg(self.bord_c()).bg(self.bg()) }
    fn borderf(&self) -> Style { Style::default().fg(self.bord_f()).bg(self.bg()) }
    fn pink_s(&self)  -> Style { Style::default().fg(self.pink()).bg(self.bg()).add_modifier(Modifier::BOLD) }
}

// ── Built-in themes ───────────────────────────────────────────────────────────
fn theme_catppuccin_mocha() -> Theme {
    Theme {
        name:   "Catppuccin Mocha".into(),
        base00: Color::Rgb(0x1e,0x1e,0x2e), base01: Color::Rgb(0x18,0x18,0x25),
        base02: Color::Rgb(0x31,0x32,0x44), base03: Color::Rgb(0x7f,0x84,0x9c),
        base04: Color::Rgb(0x45,0x47,0x5a), base05: Color::Rgb(0xcd,0xd6,0xf4),
        base06: Color::Rgb(0xf5,0xc2,0xe7), base07: Color::Rgb(0xb4,0xbe,0xfe),
        base08: Color::Rgb(0xf3,0x8b,0xa8), base09: Color::Rgb(0xfa,0xb3,0x87),
        base0a: Color::Rgb(0xf9,0xe2,0xaf), base0b: Color::Rgb(0xa6,0xe3,0xa1),
        base0c: Color::Rgb(0x74,0xc7,0xec), base0d: Color::Rgb(0x89,0xb4,0xfa),
        base0e: Color::Rgb(0xcb,0xa6,0xf7), base0f: Color::Rgb(0xf5,0xc2,0xe7),
    }
}

fn theme_gruvbox_dark() -> Theme {
    Theme {
        name:   "Gruvbox Dark".into(),
        base00: Color::Rgb(0x28,0x28,0x28), base01: Color::Rgb(0x3c,0x38,0x36),
        base02: Color::Rgb(0x50,0x49,0x45), base03: Color::Rgb(0x66,0x5c,0x54),
        base04: Color::Rgb(0xbd,0xae,0x93), base05: Color::Rgb(0xeb,0xdb,0xb2),
        base06: Color::Rgb(0xd5,0xc4,0xa1), base07: Color::Rgb(0xfb,0xf1,0xc7),
        base08: Color::Rgb(0xfb,0x49,0x34), base09: Color::Rgb(0xfe,0x80,0x19),
        base0a: Color::Rgb(0xfa,0xbd,0x2f), base0b: Color::Rgb(0xb8,0xbb,0x26),
        base0c: Color::Rgb(0x8e,0xc0,0x7c), base0d: Color::Rgb(0x83,0xa5,0x98),
        base0e: Color::Rgb(0xd3,0x86,0x9b), base0f: Color::Rgb(0xd6,0x5d,0x0e),
    }
}

fn theme_nord() -> Theme {
    Theme {
        name:   "Nord".into(),
        base00: Color::Rgb(0x2e,0x34,0x40), base01: Color::Rgb(0x3b,0x42,0x52),
        base02: Color::Rgb(0x43,0x4c,0x5e), base03: Color::Rgb(0x4c,0x56,0x6a),
        base04: Color::Rgb(0xd8,0xde,0xe9), base05: Color::Rgb(0xe5,0xe9,0xf0),
        base06: Color::Rgb(0xec,0xef,0xf4), base07: Color::Rgb(0x8f,0xbc,0xbb),
        base08: Color::Rgb(0xbf,0x61,0x6a), base09: Color::Rgb(0xd0,0x87,0x70),
        base0a: Color::Rgb(0xeb,0xcb,0x8b), base0b: Color::Rgb(0xa3,0xbe,0x8c),
        base0c: Color::Rgb(0x88,0xc0,0xd0), base0d: Color::Rgb(0x81,0xa1,0xc1),
        base0e: Color::Rgb(0xb4,0x8e,0xad), base0f: Color::Rgb(0x5e,0x81,0xac),
    }
}

fn theme_solarized_dark() -> Theme {
    Theme {
        name:   "Solarized Dark".into(),
        base00: Color::Rgb(0x00,0x2b,0x36), base01: Color::Rgb(0x07,0x36,0x42),
        base02: Color::Rgb(0x58,0x6e,0x75), base03: Color::Rgb(0x65,0x7b,0x83),
        base04: Color::Rgb(0x83,0x94,0x96), base05: Color::Rgb(0x93,0xa1,0xa1),
        base06: Color::Rgb(0xee,0xe8,0xd5), base07: Color::Rgb(0xfd,0xf6,0xe3),
        base08: Color::Rgb(0xdc,0x32,0x2f), base09: Color::Rgb(0xcb,0x4b,0x16),
        base0a: Color::Rgb(0xb5,0x89,0x00), base0b: Color::Rgb(0x85,0x99,0x00),
        base0c: Color::Rgb(0x2a,0xa1,0x98), base0d: Color::Rgb(0x26,0x8b,0xd2),
        base0e: Color::Rgb(0x6c,0x71,0xc4), base0f: Color::Rgb(0xd3,0x36,0x82),
    }
}

fn theme_dracula() -> Theme {
    Theme {
        name:   "Dracula".into(),
        base00: Color::Rgb(0x28,0x2a,0x36), base01: Color::Rgb(0x34,0x35,0x46),
        base02: Color::Rgb(0x44,0x47,0x5a), base03: Color::Rgb(0x62,0x72,0xa4),
        base04: Color::Rgb(0xa0,0xa8,0xcd), base05: Color::Rgb(0xf8,0xf8,0xf2),
        base06: Color::Rgb(0xf8,0xf8,0xf2), base07: Color::Rgb(0xff,0xff,0xff),
        base08: Color::Rgb(0xff,0x55,0x55), base09: Color::Rgb(0xff,0xb8,0x6c),
        base0a: Color::Rgb(0xf1,0xfa,0x8c), base0b: Color::Rgb(0x50,0xfa,0x7b),
        base0c: Color::Rgb(0x8b,0xe9,0xfd), base0d: Color::Rgb(0xbd,0x93,0xf9),
        base0e: Color::Rgb(0xff,0x79,0xc6), base0f: Color::Rgb(0xff,0x6e,0x6e),
    }
}

fn theme_one_dark() -> Theme {
    Theme {
        name:   "One Dark".into(),
        base00: Color::Rgb(0x28,0x2c,0x34), base01: Color::Rgb(0x35,0x3b,0x45),
        base02: Color::Rgb(0x3e,0x44,0x51), base03: Color::Rgb(0x54,0x5d,0x6e),
        base04: Color::Rgb(0x56,0x5c,0x64), base05: Color::Rgb(0xab,0xb2,0xbf),
        base06: Color::Rgb(0xb6,0xbd,0xca), base07: Color::Rgb(0xc8,0xcb,0xd4),
        base08: Color::Rgb(0xe0,0x6c,0x75), base09: Color::Rgb(0xd1,0x97,0x65),
        base0a: Color::Rgb(0xe5,0xc0,0x7b), base0b: Color::Rgb(0x98,0xc3,0x79),
        base0c: Color::Rgb(0x56,0xb6,0xc2), base0d: Color::Rgb(0x61,0xaf,0xef),
        base0e: Color::Rgb(0xc6,0x78,0xdd), base0f: Color::Rgb(0xbe,0x56,0x46),
    }
}

fn theme_tokyo_night() -> Theme {
    Theme {
        name:   "Tokyo Night".into(),
        base00: Color::Rgb(0x1a,0x1b,0x26), base01: Color::Rgb(0x16,0x17,0x22),
        base02: Color::Rgb(0x2a,0x2b,0x3d), base03: Color::Rgb(0x56,0x5f,0x89),
        base04: Color::Rgb(0xa9,0xb1,0xd6), base05: Color::Rgb(0xc0,0xca,0xf5),
        base06: Color::Rgb(0xcb,0xd5,0xf8), base07: Color::Rgb(0xe5,0xe9,0xfc),
        base08: Color::Rgb(0xf7,0x76,0x8e), base09: Color::Rgb(0xff,0x9e,0x64),
        base0a: Color::Rgb(0xe0,0xaf,0x68), base0b: Color::Rgb(0x9e,0xce,0x6a),
        base0c: Color::Rgb(0x73,0xda,0xca), base0d: Color::Rgb(0x7a,0xa2,0xf7),
        base0e: Color::Rgb(0xbb,0x9a,0xf7), base0f: Color::Rgb(0xb4,0x5b,0xcf),
    }
}

fn theme_mellow() -> Theme {
    Theme {
        name:   "Mellow".into(),
        base00: Color::Rgb(0x1e,0x1e,0x1e), base01: Color::Rgb(0x27,0x27,0x27),
        base02: Color::Rgb(0x3a,0x3a,0x3a), base03: Color::Rgb(0x63,0x63,0x63),
        base04: Color::Rgb(0x8a,0x8a,0x8a), base05: Color::Rgb(0xc5,0xbe,0xb0),
        base06: Color::Rgb(0xd8,0xd2,0xc5), base07: Color::Rgb(0xeb,0xe6,0xdb),
        base08: Color::Rgb(0xe0,0x6c,0x75), base09: Color::Rgb(0xd1,0x97,0x65),
        base0a: Color::Rgb(0xe5,0xc0,0x7b), base0b: Color::Rgb(0x7e,0xc7,0x93),
        base0c: Color::Rgb(0x5e,0xb2,0xcd), base0d: Color::Rgb(0x68,0xa7,0xe8),
        base0e: Color::Rgb(0xb2,0x8d,0xe8), base0f: Color::Rgb(0xc8,0x7e,0xa6),
    }
}

/// All built-in themes in display order
fn builtin_themes() -> Vec<Theme> {
    vec![
        theme_catppuccin_mocha(),
        theme_gruvbox_dark(),
        theme_nord(),
        theme_solarized_dark(),
        theme_dracula(),
        theme_one_dark(),
        theme_tokyo_night(),
        theme_mellow(),
    ]
}

/// Load a theme from a Base16 TOML file.
/// Expected keys: scheme, base00 .. base0F (hex strings, with or without #).
fn load_theme_from_toml(path: &std::path::Path) -> Option<Theme> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut name = path.file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or("custom")
    .to_string();
    let mut slots: Vec<String> = vec![String::new(); 16];
    for line in content.lines() {
        let line = line.trim();
        // Parse scheme name
        if line.to_lowercase().starts_with("scheme") {
            if let Some(val) = line.splitn(2, '=').nth(1) {
                name = val.trim().trim_matches('"').trim_matches('\'').to_string();
            }
        }
        for (i, key) in ["base00","base01","base02","base03","base04","base05",
            "base06","base07","base08","base09","base0a","base0b",
            "base0c","base0d","base0e","base0f"].iter().enumerate() {
                if line.to_lowercase().starts_with(key) {
                    if let Some(val) = line.splitn(2, '=').nth(1) {
                        let v = val.trim().trim_matches('"').trim_matches('\'').to_string();
                        if !v.is_empty() && slots[i].is_empty() { slots[i] = v; }
                    }
                }
            }
    }
    // Require at least base00 and base05
    if slots[0].is_empty() || slots[5].is_empty() { return None; }
    let p = |i: usize| parse_hex_color(if slots[i].is_empty() { "#000000" } else { &slots[i] });
    Some(Theme {
        name,
         base00: p(0),  base01: p(1),  base02: p(2),  base03: p(3),
         base04: p(4),  base05: p(5),  base06: p(6),  base07: p(7),
         base08: p(8),  base09: p(9),  base0a: p(10), base0b: p(11),
         base0c: p(12), base0d: p(13), base0e: p(14), base0f: p(15),
    })
}

/// Collect all available themes: builtins first, then any files in
/// ~/.config/netcontrol/themes/*.toml
fn all_themes() -> Vec<Theme> {
    let mut themes = builtin_themes();
    let theme_dir = home_dir().join(".config").join("netcontrol").join("themes");
    if let Ok(rd) = std::fs::read_dir(&theme_dir) {
        let mut extras: Vec<Theme> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "toml"))
        .filter_map(|e| load_theme_from_toml(&e.path()))
        .collect();
        extras.sort_by(|a,b| a.name.cmp(&b.name));
        themes.extend(extras);
    }
    themes
}

// ── Digital modes ─────────────────────────────────────────────────────────────
const DIGITAL_MODES: &[&str] = &[
    "FT8","FT4","JS8Call","Winlink","APRS","RTTY",
"PSK31","OLIVIA","VARA HF","VARA FM","D-STAR",
"DMR","System Fusion / YSF","P25","NXDN","SSTV",
"WSPR","MSK144","Q65","OTHER",
];

// ── ASCII art logo ───────────────────────────────────────────────────────────
const LOGO: &[&str] = &[
    r" ███╗   ██╗███████╗████████╗     ██████╗████████╗██████╗ ██╗     ",
r" ████╗  ██║██╔════╝╚══██╔══╝    ██╔════╝╚══██╔══╝██╔══██╗██║     ",
r" ██╔██╗ ██║█████╗     ██║       ██║        ██║   ██████╔╝██║     ",
r" ██║╚██╗██║██╔══╝     ██║       ██║        ██║   ██╔══██╗██║     ",
r" ██║ ╚████║███████╗   ██║       ╚██████╗   ██║   ██║  ██║███████╗",
r" ╚═╝  ╚═══╝╚══════╝   ╚═╝        ╚═════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝",
r"",
r"              Amateur Radio Net Check-in Logger  ◈  v1.0          ",
];

// ── Data ──────────────────────────────────────────────────────────────────────
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
struct CheckIn {
    id: String, callsign: String, name: String,
    remarks: String, time: String,
}

/// A single dated occurrence of a net.
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
struct Session {
    id:       String,
    date:     String,
    net_time: String,
    #[serde(default)] checkins: Vec<CheckIn>,
}

impl Session {
    fn new_today() -> Self {
        Session {
            id:       new_id(),
            date:     chrono::Local::now().format("%Y-%m-%d").to_string(),
            net_time: chrono::Local::now().format("%H:%M").to_string(),
            checkins: vec![],
        }
    }
    fn label(&self) -> String {
        let cnt = self.checkins.len();
        format!("{} {:>5}  ({} check-in{})", self.date, self.net_time, cnt, if cnt==1{""} else {"s"})
    }
}

#[derive(Debug,Clone,Serialize,Deserialize,Default)]
struct Net {
    id: String, name: String,
    #[serde(default)] club: String,
    freq: String, offset: String, pl: String,
    // Legacy flat fields kept for migration; new data uses sessions vec.
    #[serde(default)] date: String,
    #[serde(default, rename="time")] net_time: String,
    #[serde(default)] digital: bool,
    #[serde(default)] mode: String,
    #[serde(default)] mode_notes: String,
    // Legacy flat check-ins; migrated to sessions on load.
    #[serde(default)] checkins: Vec<CheckIn>,
    #[serde(default)] sessions: Vec<Session>,
}

impl Net {
    /// Total check-ins across all sessions.
    fn total_checkins(&self) -> usize {
        self.sessions.iter().map(|s| s.checkins.len()).sum()
    }
    /// Migrate legacy flat checkins into a single session if needed.
    fn migrate(&mut self) {
        if !self.checkins.is_empty() && self.sessions.is_empty() {
            let date = if self.date.is_empty() {
                chrono::Local::now().format("%Y-%m-%d").to_string()
            } else { self.date.clone() };
            let net_time = if self.net_time.is_empty() {
                chrono::Local::now().format("%H:%M").to_string()
            } else { self.net_time.clone() };
            self.sessions.push(Session {
                id:       new_id(),
                               date,
                               net_time,
                               checkins: std::mem::take(&mut self.checkins),
            });
        }
    }
}

/// A remembered callsign/name pair, built up from check-in history.
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
struct KnownOp {
    callsign: String,
    name:     String,
}

#[derive(Debug,Serialize,Deserialize,Default)]
struct AppData {
    #[serde(default)] operator_name: String,
    #[serde(default)] operator_call: String,
    #[serde(default)] theme_name:    String,
    #[serde(default)] known_ops:     Vec<KnownOp>,
    nets: Vec<Net>,
}

impl AppData {
    /// Upsert a callsign/name pair into known_ops.
    fn remember_op(&mut self, callsign: &str, name: &str) {
        let cs = callsign.trim().to_uppercase();
        let nm = name.trim().to_string();
        if cs.is_empty() { return; }
        if let Some(op) = self.known_ops.iter_mut().find(|o| o.callsign == cs) {
            if !nm.is_empty() { op.name = nm; }
        } else {
            self.known_ops.push(KnownOp { callsign: cs, name: nm });
        }
    }
    /// Return callsigns that start with the given prefix (case-insensitive).
    fn completions(&self, prefix: &str) -> Vec<&KnownOp> {
        let p = prefix.to_uppercase();
        self.known_ops.iter()
        .filter(|o| o.callsign.starts_with(&p))
        .collect()
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."))
}
fn data_path() -> PathBuf { home_dir().join(".netcontrol_data.json") }
fn load_data() -> AppData {
    let p = data_path();
    if p.exists() {
        if let Ok(s) = std::fs::read_to_string(&p) {
            if let Ok(mut d) = serde_json::from_str::<AppData>(&s) {
                for net in &mut d.nets { net.migrate(); }
                return d;
            }
        }
    }
    AppData::default()
}
fn save_data(d: &AppData) {
    if let Ok(s) = serde_json::to_string_pretty(d) { let _ = std::fs::write(data_path(), s); }
}
fn utc_now() -> String { Utc::now().format("%H:%Mz").to_string() }
fn new_id()  -> String { Utc::now().format("%Y%m%d%H%M%S%6f").to_string() }

// ── App screen ────────────────────────────────────────────────────────────────
// On first launch (no operator info saved) we show a startup profile screen.
#[derive(Debug,PartialEq)]
enum Screen { Startup, Main }

// ── Modal state ───────────────────────────────────────────────────────────────
// Net dialog field indices
const NF_NAME:usize=0; const NF_CLUB:usize=1; const NF_FREQ:usize=2;
const NF_OFFSET:usize=3; const NF_PL:usize=4; const NF_DATE:usize=5;
const NF_TIME:usize=6; const NF_TOGGLE:usize=7; const NF_MODE:usize=8;
const NF_NOTES:usize=9;

#[derive(Debug,PartialEq,Clone,Copy)]
enum NdMode { Add, Edit }

#[derive(Debug)]
struct NetDlg {
    mode:     NdMode,
    fields:   [String;7],  // name club freq offset pl date time
    digital:  bool,
    mode_idx: usize,
    notes:    String,
    focus:    usize,
    edit_id:  Option<String>,
}
impl NetDlg {
    fn new_add() -> Self {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let now   = chrono::Local::now().format("%H:%M").to_string();
        Self {
            mode: NdMode::Add,
            fields: [String::new(), String::new(), String::new(),
            "+0.600".into(), "NONE".into(), today, now],
            digital: false, mode_idx: 0, notes: String::new(),
            focus: NF_NAME, edit_id: None,
        }
    }
    fn new_edit(n: &Net) -> Self {
        let mi = DIGITAL_MODES.iter().position(|&m| m==n.mode).unwrap_or(0);
        Self {
            mode: NdMode::Edit,
            fields: [n.name.clone(), n.club.clone(), n.freq.clone(),
            n.offset.clone(), n.pl.clone(), n.date.clone(), n.net_time.clone()],
            digital: n.digital, mode_idx: mi, notes: n.mode_notes.clone(),
            focus: NF_NAME, edit_id: Some(n.id.clone()),
        }
    }
    fn max_focus(&self) -> usize { if self.digital { NF_NOTES } else { NF_TOGGLE } }
    fn cur_field_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            NF_NAME..=NF_TIME => Some(&mut self.fields[self.focus]),
            NF_NOTES          => Some(&mut self.notes),
            _                 => None,
        }
    }
    fn max_len(&self) -> usize {
        match self.focus {
            NF_NAME=>30, NF_CLUB=>40, NF_FREQ=>12, NF_OFFSET=>12,
            NF_PL=>10, NF_DATE=>12, NF_TIME=>6, NF_NOTES=>50, _=>0,
        }
    }
}

// Operator profile dialog field indices
const OF_CALL:usize=0; const OF_NAME:usize=1;

#[derive(Debug)]
struct OperatorDlg {
    fields: [String;2],   // callsign, name
    focus:  usize,
    /// true = came from startup screen (must complete), false = editing from main
    required: bool,
}
impl OperatorDlg {
    fn new(call: &str, name: &str, required: bool) -> Self {
        Self {
            fields: [call.to_string(), name.to_string()],
            focus: OF_CALL,
            required,
        }
    }
    fn cur_mut(&mut self) -> &mut String { &mut self.fields[self.focus] }
    fn max_len(&self) -> usize { if self.focus==OF_CALL { 12 } else { 40 } }
}

/// Result of an async FCC lookup.
#[derive(Debug)]
enum FccResult { Found(String), NotFound, Error }

#[derive(Debug)]
struct CiDlg {
    callsign:     String,
    name:         String,
    remarks:      String,
    focus:        usize,
    /// Filtered completions for the current callsign prefix.
    completions:  Vec<String>,   // (callsign, display label)
    comp_labels:  Vec<String>,
    comp_sel:     Option<usize>,
    /// Channel for FCC lookup results.
    fcc_rx:       Option<mpsc::Receiver<FccResult>>,
    fcc_pending:  bool,
}
impl CiDlg {
    fn new() -> Self {
        Self {
            callsign: String::new(), name: String::new(), remarks: String::new(),
            focus: 0,
            completions: vec![], comp_labels: vec![], comp_sel: None,
            fcc_rx: None, fcc_pending: false,
        }
    }
    fn cur_mut(&mut self) -> &mut String {
        match self.focus { 0=>&mut self.callsign, 1=>&mut self.name, _=>&mut self.remarks }
    }
    fn max_len(&self) -> usize { match self.focus { 0=>12, 1=>30, _=>50 } }
    /// Update completions from known_ops given current callsign prefix.
    fn update_completions(&mut self, ops: &[KnownOp]) {
        let prefix = self.callsign.to_uppercase();
        if prefix.is_empty() {
            self.completions.clear();
            self.comp_labels.clear();
            self.comp_sel = None;
            return;
        }
        let matches: Vec<_> = ops.iter()
        .filter(|o| o.callsign.starts_with(&prefix))
        .take(8)
        .collect();
        self.completions = matches.iter().map(|o| o.callsign.clone()).collect();
        self.comp_labels  = matches.iter().map(|o|
        if o.name.is_empty() { o.callsign.clone() }
        else { format!("{} — {}", o.callsign, o.name) }
        ).collect();
        self.comp_sel = if self.completions.is_empty() { None } else { Some(0) };
    }
    /// Apply the selected completion (fill callsign + name).
    fn apply_completion(&mut self, ops: &[KnownOp]) {
        let Some(idx) = self.comp_sel else { return };
        let Some(cs) = self.completions.get(idx) else { return };
        let cs = cs.clone();
        if let Some(op) = ops.iter().find(|o| o.callsign == cs) {
            self.callsign = op.callsign.clone();
            if !op.name.is_empty() { self.name = op.name.clone(); }
        }
        self.completions.clear();
        self.comp_labels.clear();
        self.comp_sel = None;
    }
    /// Kick off a background FCC lookup for the current callsign.
    fn start_fcc_lookup(&mut self) {
        let cs = self.callsign.trim().to_uppercase();
        if cs.is_empty() || cs.len() < 3 { return; }
        let (tx, rx) = mpsc::channel();
        self.fcc_rx      = Some(rx);
        self.fcc_pending = true;
        std::thread::spawn(move || {
            // callook.info returns a flat JSON object for the callsign.
            // Example: { "status": "VALID", "name": { "full": "HIRAM PERCY MAXIM" }, ... }
            let url = format!("https://callook.info/{}/json", cs);
            let result = (|| -> Option<String> {
                let body = minreq::get(&url)
                .with_timeout(5)
                .send().ok()?;
                let resp: serde_json::Value =
                serde_json::from_str(body.as_str().ok()?).ok()?;
                // callook.info: { "status": "VALID", "name": "HIRAM PERCY MAXIM", ... }
                if resp.get("status")?.as_str()? != "VALID" { return None; }
                let name = resp.get("name")?.as_str()?.trim().to_string();
                if name.is_empty() { None } else { Some(name) }
            })();
            let _ = tx.send(match result {
                Some(name) => FccResult::Found(name),
                            None       => FccResult::NotFound,
            });
        });
    }
    /// Poll the FCC channel; return Some(result) if ready.
    fn poll_fcc(&mut self) -> Option<FccResult> {
        let rx = self.fcc_rx.as_ref()?;
        match rx.try_recv() {
            Ok(r)  => { self.fcc_pending = false; self.fcc_rx = None; Some(r) }
            Err(_) => None,
        }
    }
}

#[derive(Debug)]
struct ModePick { sel: usize, offset: usize }

#[derive(Debug,PartialEq)]
enum ConfirmKind { DelNet, DelSession, DelCi }
#[derive(Debug)]
struct ConfirmDlg { kind: ConfirmKind, msg: String }
#[derive(Debug)]
struct MsgDlg { title: String, msg: String }

#[derive(Debug)]
struct ExportDlg { filename: String }

#[derive(Debug)]
struct ThemePickerDlg {
    themes:  Vec<Theme>,
    sel:     usize,
    offset:  usize,
}
impl ThemePickerDlg {
    fn new(themes: Vec<Theme>, current: &str) -> Self {
        let sel = themes.iter().position(|t| t.name == current).unwrap_or(0);
        let offset = sel.saturating_sub(5);
        Self { themes, sel, offset }
    }
}
impl ExportDlg {
    fn new(default: &str) -> Self { Self { filename: default.to_string() } }
}

#[derive(Debug)]
enum Modal {
    None,
    Operator(OperatorDlg),
    Net(NetDlg),
    Ci(CiDlg),
    Picker { dlg: NetDlg, pick: ModePick },
    Confirm(ConfirmDlg),
    Msg(MsgDlg),
    Export(ExportDlg),
    ThemePicker(ThemePickerDlg),
    QuitConfirm,
    Help,
}

// ── App ───────────────────────────────────────────────────────────────────────
#[derive(Debug,Clone,Copy,PartialEq)]
enum Focus { Nets, Sessions, Log }

struct App {
    data:       AppData,
    screen:     Screen,
    focus:      Focus,
    net_ls:     ListState,
    ses_ls:     ListState,   // session list (per net)
    log_ls:     ListState,
    modal:      Modal,
    tick:       Instant,
    clock:      String,
    panel_w:    u16,
    ses_pane_h: u16,   // height of sessions pane when log is visible; resizable Ctrl+↑/↓
    theme:      Theme,
}
impl App {
    fn new() -> Self {
        let data = load_data();
        let mut net_ls = ListState::default();
        if !data.nets.is_empty() { net_ls.select(Some(0)); }
        // Show startup screen if operator profile is empty
        let screen = if data.operator_call.is_empty() { Screen::Startup } else { Screen::Main };
        let modal = if screen == Screen::Startup {
            Modal::Operator(OperatorDlg::new("", "", true))
        } else {
            Modal::None
        };
        // Load theme: prefer saved name, fall back to Catppuccin Mocha
        let theme = {
            let name = &data.theme_name;
            all_themes().into_iter()
            .find(|t| &t.name == name)
            .unwrap_or_else(theme_catppuccin_mocha)
        };
        Self {
            data, screen, focus: Focus::Nets,
            net_ls, ses_ls: ListState::default(), log_ls: ListState::default(),
            modal, tick: Instant::now(),
            clock: Utc::now().format("%H:%M:%S UTC").to_string(),
            panel_w: 30,
            ses_pane_h: 8,
            theme,
        }
    }
    fn tick(&mut self) { self.clock = Utc::now().format("%H:%M:%S UTC").to_string(); }
    fn net(&self)       -> Option<&Net>      { self.net_ls.selected().and_then(|i| self.data.nets.get(i)) }
    fn net_mut(&mut self) -> Option<&mut Net> { self.net_ls.selected().and_then(|i| self.data.nets.get_mut(i)) }
    fn ni(&self) -> Option<usize> { self.net_ls.selected() }
    fn si(&self) -> Option<usize> { self.ses_ls.selected() }
    fn ci(&self) -> Option<usize> { self.log_ls.selected() }
    fn active_session(&self) -> Option<&Session> {
        let ni = self.ni()?;
        let si = self.si()?;
        self.data.nets.get(ni)?.sessions.get(si)
    }
    fn active_session_mut(&mut self) -> Option<&mut Session> {
        let ni = self.ni()?;
        let si = self.si()?;
        self.data.nets.get_mut(ni)?.sessions.get_mut(si)
    }
    fn op_str(&self) -> String {
        let c = &self.data.operator_call;
        let n = &self.data.operator_name;
        match (c.is_empty(), n.is_empty()) {
            (true,  true)  => String::new(),
            (false, true)  => c.clone(),
            (true,  false) => n.clone(),
            (false, false) => format!("{} ({})", c, n),
        }
    }
}

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
    println!("73 de NET CONTROL — 73!");
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
                if !on_key(&mut app, k.code, k.modifiers) { return Ok(()); }
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

// ── Input dispatch ────────────────────────────────────────────────────────────
fn on_key(app: &mut App, key: KeyCode, mods: KeyModifiers) -> bool {
    match key {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            match &app.modal {
                Modal::None => { app.modal = Modal::QuitConfirm; }
                Modal::QuitConfirm => return false,
                Modal::Operator(d) if d.required => { app.modal = Modal::QuitConfirm; }
                _ => { app.modal = Modal::None; }
            }
        }
        _ => {
            match &app.modal {
                Modal::None        => { if !on_main(app, key, mods) { return false; } }
                Modal::Operator(_) => on_operator_dlg(app, key),
                Modal::Net(_)      => on_net_dlg(app, key),
                Modal::Ci(_)       => on_ci_dlg(app, key),
                Modal::Picker{..}  => on_picker(app, key),
                Modal::Confirm(_)  => on_confirm(app, key),
                Modal::Msg(_)         => { app.modal = Modal::None; }
                Modal::Export(_)      => on_export_dlg(app, key),
                Modal::ThemePicker(_) => on_theme_picker(app, key),
                Modal::QuitConfirm    => { if on_quit_confirm(app, key) { return false; } }
                Modal::Help           => { app.modal = Modal::None; }
            }
        }
    }
    true
}

fn on_main(app: &mut App, key: KeyCode, mods: KeyModifiers) -> bool {
    match key {
        KeyCode::Char('q') | KeyCode::Char('Q') => return false,

        // Ctrl+Left / Ctrl+Right — resize the nets panel (tmux-style)
        KeyCode::Left if mods.contains(KeyModifiers::CONTROL) => {
            if app.panel_w > 10 { app.panel_w -= 1; }
        }
        KeyCode::Right if mods.contains(KeyModifiers::CONTROL) => {
            if app.panel_w < 60 { app.panel_w += 1; }
        }
        // Ctrl+Up / Ctrl+Down — resize the sessions pane height
        KeyCode::Up if mods.contains(KeyModifiers::CONTROL) => {
            if app.ses_pane_h > 3 { app.ses_pane_h -= 1; }
        }
        KeyCode::Down if mods.contains(KeyModifiers::CONTROL) => {
            if app.ses_pane_h < 30 { app.ses_pane_h += 1; }
        }

        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Nets     => Focus::Sessions,
                Focus::Sessions => Focus::Log,
                Focus::Log      => Focus::Nets,
            };
        }

        // Esc navigates back through the focus hierarchy
        KeyCode::Esc => {
            match app.focus {
                Focus::Log      => { app.focus = Focus::Sessions; }
                Focus::Sessions => { app.focus = Focus::Nets; }
                Focus::Nets     => {}
            }
        }

        KeyCode::Up => match app.focus {
            Focus::Nets => {
                let i = app.net_ls.selected().unwrap_or(0);
                app.net_ls.select(Some(i.saturating_sub(1)));
                app.ses_ls.select(None);
                app.log_ls.select(None);
            }
            Focus::Sessions => {
                let len = app.net().map_or(0, |n| n.sessions.len());
                if len > 0 {
                    let i = app.ses_ls.selected().unwrap_or(0);
                    app.ses_ls.select(Some(i.saturating_sub(1)));
                    app.log_ls.select(None);
                }
            }
            Focus::Log => {
                let len = app.active_session().map_or(0, |s| s.checkins.len());
                if len > 0 {
                    let i = app.log_ls.selected().unwrap_or(0);
                    app.log_ls.select(Some(i.saturating_sub(1)));
                }
            }
        }

        KeyCode::Down => match app.focus {
            Focus::Nets => {
                let len = app.data.nets.len();
                if len > 0 {
                    let i = app.net_ls.selected().unwrap_or(0);
                    app.net_ls.select(Some((i+1).min(len-1)));
                    app.ses_ls.select(None);
                    app.log_ls.select(None);
                }
            }
            Focus::Sessions => {
                let len = app.net().map_or(0, |n| n.sessions.len());
                if len > 0 {
                    let i = app.ses_ls.selected().unwrap_or(0);
                    app.ses_ls.select(Some((i+1).min(len-1)));
                    app.log_ls.select(None);
                }
            }
            Focus::Log => {
                let len = app.active_session().map_or(0, |s| s.checkins.len());
                if len > 0 {
                    let i = app.log_ls.selected().unwrap_or(0);
                    app.log_ls.select(Some((i+1).min(len-1)));
                }
            }
        }

        KeyCode::Enter => match app.focus {
            Focus::Nets => {
                if !app.data.nets.is_empty() {
                    app.focus = Focus::Sessions;
                    if app.ses_ls.selected().is_none() {
                        let len = app.net().map_or(0, |n| n.sessions.len());
                        if len > 0 { app.ses_ls.select(Some(0)); }
                    }
                }
            }
            Focus::Sessions => {
                if app.ses_ls.selected().is_some() {
                    app.focus = Focus::Log;
                    if app.log_ls.selected().is_none() {
                        let len = app.active_session().map_or(0, |s| s.checkins.len());
                        if len > 0 { app.log_ls.select(Some(0)); }
                    }
                }
            }
            Focus::Log => {}
        }

        // [n] — add net (from Nets), or new session (from Sessions/Log)
        KeyCode::Char('n') => match app.focus {
            Focus::Nets => { app.modal = Modal::Net(NetDlg::new_add()); }
            Focus::Sessions | Focus::Log => {
                if app.net().is_some() {
                    if let Some(ni) = app.ni() {
                        let ses = Session::new_today();
                        app.data.nets[ni].sessions.push(ses);
                        let last = app.data.nets[ni].sessions.len() - 1;
                        app.ses_ls.select(Some(last));
                        app.log_ls.select(None);
                        app.focus = Focus::Sessions;
                        save_data(&app.data);
                    }
                }
            }
        }

        // [e] — edit net (any focus)
        KeyCode::Char('e') => {
            if let Some(n) = app.net() { app.modal = Modal::Net(NetDlg::new_edit(n)); }
        }

        // [c] — add check-in (sessions/log focus; auto-creates session if none)
        KeyCode::Char('c') => {
            if let Some(ni) = app.ni() {
                // Ensure there's an active session to add to
                if app.data.nets[ni].sessions.is_empty() {
                    let ses = Session::new_today();
                    app.data.nets[ni].sessions.push(ses);
                    app.ses_ls.select(Some(0));
                    save_data(&app.data);
                }
                if app.ses_ls.selected().is_none() {
                    let last = app.data.nets[ni].sessions.len() - 1;
                    app.ses_ls.select(Some(last));
                }
                app.modal = Modal::Ci(CiDlg::new());
                app.focus = Focus::Log;
            }
        }

        // [d] — delete
        KeyCode::Char('d') => match app.focus {
            Focus::Nets => {
                if let Some(n) = app.net() {
                    let msg = format!("Delete net '{}'?", n.name);
                    app.modal = Modal::Confirm(ConfirmDlg{kind:ConfirmKind::DelNet, msg});
                }
            }
            Focus::Sessions => {
                if let (Some(ni), Some(si)) = (app.ni(), app.si()) {
                    let lbl = app.data.nets[ni].sessions[si].label();
                    let msg = format!("Delete session {}?", lbl);
                    app.modal = Modal::Confirm(ConfirmDlg{kind:ConfirmKind::DelSession, msg});
                }
            }
            Focus::Log => {
                if let (Some(ni), Some(si), Some(ci)) = (app.ni(), app.si(), app.ci()) {
                    if let Some(c) = app.data.nets[ni].sessions[si].checkins.get(ci) {
                        let msg = format!("Remove {} from log?", c.callsign);
                        app.modal = Modal::Confirm(ConfirmDlg{kind:ConfirmKind::DelCi, msg});
                    }
                }
            }
        }

        KeyCode::Char('x') => do_export(app),

        KeyCode::Char('p') | KeyCode::Char('P') => {
            let call = app.data.operator_call.clone();
            let name = app.data.operator_name.clone();
            app.modal = Modal::Operator(OperatorDlg::new(&call, &name, false));
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            let themes = all_themes();
            let current = app.theme.name.clone();
            app.modal = Modal::ThemePicker(ThemePickerDlg::new(themes, &current));
        }
        KeyCode::Char('?') => { app.modal = Modal::Help; }
        _ => {}
    }
    true
}

// ── Quit confirmation ────────────────────────────────────────────────────────
fn on_quit_confirm(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => return true, // signal quit
        _ => { app.modal = Modal::None; }
    }
    false
}

// ── Operator dialog ───────────────────────────────────────────────────────────
fn on_operator_dlg(app: &mut App, key: KeyCode) {
    let Modal::Operator(ref mut d) = app.modal else { return };
    match key {
        KeyCode::Esc => {
            if d.required {
                // Can't dismiss required startup dialog — do nothing
            } else {
                app.modal = Modal::None;
            }
        }
        KeyCode::Enter | KeyCode::Down => {
            if d.focus == 0 {
                // Auto-uppercase the callsign before moving on
                d.fields[OF_CALL] = d.fields[OF_CALL].trim().to_uppercase();
                d.focus = 1;
            } else {
                commit_operator(app);
            }
        }
        KeyCode::Up => {
            let Modal::Operator(ref mut d) = app.modal else { return };
            if d.focus > 0 { d.focus -= 1; }
        }
        KeyCode::Backspace => {
            let Modal::Operator(ref mut d) = app.modal else { return };
            d.cur_mut().pop();
        }
        KeyCode::Delete => {
            let Modal::Operator(ref mut d) = app.modal else { return };
            d.cur_mut().clear();
        }
        KeyCode::Char(c) => {
            let Modal::Operator(ref mut d) = app.modal else { return };
            let max = d.max_len();
            let focus = d.focus;
            if d.cur_mut().len() < max {
                let ch = if focus == OF_CALL { c.to_ascii_uppercase() } else { c };
                d.cur_mut().push(ch);
            }
        }
        _ => {}
    }
}

fn commit_operator(app: &mut App) {
    let Modal::Operator(ref d) = app.modal else { return };
    let call = d.fields[OF_CALL].trim().to_uppercase();
    if call.is_empty() {
        if let Modal::Operator(ref mut d) = app.modal { d.focus = OF_CALL; }
        return;
    }
    let name = d.fields[OF_NAME].trim().to_string();
    app.data.operator_call = call;
    app.data.operator_name = name;
    save_data(&app.data);
    app.screen = Screen::Main;
    app.modal  = Modal::None;
}

// ── Net dialog ────────────────────────────────────────────────────────────────
fn on_net_dlg(app: &mut App, key: KeyCode) {
    if !matches!(app.modal, Modal::Net(_)) { return; }
    match key {
        KeyCode::Esc => { app.modal = Modal::None; }

        KeyCode::Enter => {
            let (focus, max, mode_idx) = {
                let Modal::Net(ref d) = app.modal else { return };
                (d.focus, d.max_focus(), d.mode_idx)
            };
            if focus == NF_MODE {
                let offset = mode_idx.saturating_sub(5);
                let Modal::Net(d) = std::mem::replace(&mut app.modal, Modal::None) else { return };
                app.modal = Modal::Picker { dlg: d, pick: ModePick { sel: mode_idx, offset } };
            } else if focus < max {
                let Modal::Net(ref mut d) = app.modal else { return };
                d.focus += 1;
            } else {
                commit_net(app);
            }
        }

        KeyCode::Down => {
            let Modal::Net(ref mut d) = app.modal else { return };
            let m = d.max_focus();
            if d.focus < m { d.focus += 1; }
        }

        KeyCode::Up => {
            let Modal::Net(ref mut d) = app.modal else { return };
            if d.focus > 0 { d.focus -= 1; }
        }

        KeyCode::Char(' ') => {
            let Modal::Net(ref mut d) = app.modal else { return };
            if d.focus == NF_TOGGLE {
                d.digital = !d.digital;
                if !d.digital && d.focus > NF_TOGGLE { d.focus = NF_TOGGLE; }
            } else if d.focus != NF_MODE {
                let max = d.max_len();
                if let Some(f) = d.cur_field_mut() {
                    if f.len() < max { f.push(' '); }
                }
            }
        }

        KeyCode::Backspace => {
            let Modal::Net(ref mut d) = app.modal else { return };
            if let Some(f) = d.cur_field_mut() { f.pop(); }
        }

        KeyCode::Delete => {
            let Modal::Net(ref mut d) = app.modal else { return };
            if let Some(f) = d.cur_field_mut() { f.clear(); }
        }

        KeyCode::Char(c) => {
            let Modal::Net(ref mut d) = app.modal else { return };
            if d.focus == NF_TOGGLE || d.focus == NF_MODE { return; }
            let max   = d.max_len();
            let focus = d.focus;
            let ch    = if focus == NF_NAME { c.to_ascii_uppercase() } else { c };
            if let Some(f) = d.cur_field_mut() { if f.len() < max { f.push(ch); } }
        }

        _ => {}
    }
}

fn commit_net(app: &mut App) {
    let Modal::Net(ref dlg) = app.modal else { return };
    let name = dlg.fields[NF_NAME].trim().to_uppercase();
    if name.is_empty() {
        if let Modal::Net(ref mut d) = app.modal { d.focus = NF_NAME; }
        return;
    }
    let net = Net {
        id:         dlg.edit_id.clone().unwrap_or_else(new_id),
        name,
        club:       dlg.fields[NF_CLUB].trim().to_string(),
        freq:       dlg.fields[NF_FREQ].trim().to_string(),
        offset:     dlg.fields[NF_OFFSET].trim().to_string(),
        pl:         dlg.fields[NF_PL].trim().to_string(),
        date:       dlg.fields[NF_DATE].trim().to_string(),
        net_time:   dlg.fields[NF_TIME].trim().to_string(),
        digital:    dlg.digital,
        mode:       if dlg.digital { DIGITAL_MODES[dlg.mode_idx].into() } else { String::new() },
        mode_notes: dlg.notes.trim().to_string(),
        checkins:   vec![],
        sessions:   vec![],
    };
    match dlg.mode {
        NdMode::Add => {
            app.data.nets.push(net);
            let idx = app.data.nets.len()-1;
            app.net_ls.select(Some(idx));
            app.log_ls.select(None);
        }
        NdMode::Edit => {
            if let Some(i) = app.ni() {
                let old_sessions = app.data.nets[i].sessions.clone();
                app.data.nets[i] = net;
                app.data.nets[i].sessions = old_sessions;
            }
        }
    }
    save_data(&app.data);
    app.modal = Modal::None;
}

// ── Check-in dialog ───────────────────────────────────────────────────────────
fn on_ci_dlg(app: &mut App, key: KeyCode) {
    if !matches!(app.modal, Modal::Ci(_)) { return; }
    match key {
        KeyCode::Esc => { app.modal = Modal::None; }

        // Tab applies the selected completion or moves focus
        KeyCode::Tab => {
            let Modal::Ci(ref mut d) = app.modal else { return };
            if d.comp_sel.is_some() {
                let ops = app.data.known_ops.clone();
                let Modal::Ci(ref mut d) = app.modal else { return };
                d.apply_completion(&ops);
            } else if d.focus == 0 {
                // Trigger FCC lookup when leaving callsign field via Tab
                let Modal::Ci(ref mut d) = app.modal else { return };
                if !d.fcc_pending {
                    d.start_fcc_lookup();
                }
                d.focus = 1;
            }
        }

        // Up/Down navigate the completion list when it is visible
        KeyCode::Up => {
            let Modal::Ci(ref mut d) = app.modal else { return };
            if d.comp_sel.is_some() && !d.completions.is_empty() {
                let i = d.comp_sel.unwrap_or(0);
                d.comp_sel = Some(if i == 0 { d.completions.len()-1 } else { i-1 });
            } else if d.focus > 0 {
                d.focus -= 1;
            }
        }

        KeyCode::Down => {
            let Modal::Ci(ref mut d) = app.modal else { return };
            if d.comp_sel.is_some() && !d.completions.is_empty() {
                let i = d.comp_sel.unwrap_or(0);
                d.comp_sel = Some((i+1) % d.completions.len());
            } else if d.focus < 2 {
                d.focus += 1;
            }
        }

        KeyCode::Enter => {
            // If a completion is selected, apply it; otherwise advance/confirm
            let has_comp = matches!(&app.modal, Modal::Ci(d) if d.comp_sel.is_some() && !d.completions.is_empty());
            if has_comp {
                let ops = app.data.known_ops.clone();
                let Modal::Ci(ref mut d) = app.modal else { return };
                d.apply_completion(&ops);
            } else {
                let Modal::Ci(ref mut d) = app.modal else { return };
                if d.focus < 2 {
                    if d.focus == 0 && !d.fcc_pending {
                        d.start_fcc_lookup();
                    }
                    d.focus += 1;
                } else {
                    commit_ci(app);
                }
            }
        }

        KeyCode::Backspace => {
            let Modal::Ci(ref mut d) = app.modal else { return };
            d.cur_mut().pop();
            if d.focus == 0 {
                let ops = app.data.known_ops.clone();
                let Modal::Ci(ref mut d) = app.modal else { return };
                d.update_completions(&ops);
            }
        }

        KeyCode::Delete => {
            let Modal::Ci(ref mut d) = app.modal else { return };
            d.cur_mut().clear();
            if d.focus == 0 {
                let Modal::Ci(ref mut d) = app.modal else { return };
                d.completions.clear();
                d.comp_labels.clear();
                d.comp_sel = None;
            }
        }

        KeyCode::Char(c) => {
            let Modal::Ci(ref mut d) = app.modal else { return };
            let max = d.max_len();
            let f   = d.focus;
            if d.cur_mut().len() < max {
                let ch = if f == 0 { c.to_ascii_uppercase() } else { c };
                d.cur_mut().push(ch);
            }
            if f == 0 {
                let ops = app.data.known_ops.clone();
                let Modal::Ci(ref mut d) = app.modal else { return };
                d.update_completions(&ops);
            }
        }

        _ => {}
    }
}

fn commit_ci(app: &mut App) {
    let Modal::Ci(ref dlg) = app.modal else { return };
    let cs = dlg.callsign.trim().to_uppercase();
    if cs.is_empty() {
        if let Modal::Ci(ref mut d)=app.modal { d.focus=0; }
        return;
    }
    let ci = CheckIn {
        id: new_id(), callsign: cs,
        name: dlg.name.trim().into(),
        remarks: dlg.remarks.trim().into(),
        time: utc_now(),
    };
    if let Some(ses) = app.active_session_mut() {
        ses.checkins.push(ci);
        let last = ses.checkins.len()-1;
        app.log_ls.select(Some(last));
    }
    // Remember this callsign/name pair
    let (cs2, nm2) = {
        let Modal::Ci(ref d) = app.modal else { unreachable!() };
        (d.callsign.clone(), d.name.clone())
    };
    app.data.remember_op(&cs2, &nm2);
    save_data(&app.data);
    app.modal = Modal::None;
}

// ── Mode picker ───────────────────────────────────────────────────────────────
fn on_picker(app: &mut App, key: KeyCode) {
    let Modal::Picker{ref mut pick, ..} = app.modal else { return };
    let len = DIGITAL_MODES.len();
    let vis = 12usize;
    match key {
        KeyCode::Esc => {
            let Modal::Picker{dlg,..} = std::mem::replace(&mut app.modal, Modal::None) else{return};
            app.modal = Modal::Net(dlg);
        }
        KeyCode::Up => {
            if pick.sel > 0 { pick.sel -= 1; }
            if pick.sel < pick.offset { pick.offset = pick.sel; }
        }
        KeyCode::Down => {
            if pick.sel+1 < len { pick.sel += 1; }
            if pick.sel >= pick.offset+vis { pick.offset = pick.sel-vis+1; }
        }
        KeyCode::PageUp => {
            pick.sel = pick.sel.saturating_sub(vis);
            if pick.sel < pick.offset { pick.offset = pick.sel; }
        }
        KeyCode::PageDown => {
            pick.sel = (pick.sel+vis).min(len-1);
            if pick.sel >= pick.offset+vis { pick.offset = pick.sel-vis+1; }
        }
        KeyCode::Enter => {
            let chosen = pick.sel;
            let Modal::Picker{mut dlg,..} = std::mem::replace(&mut app.modal, Modal::None) else{return};
            dlg.mode_idx = chosen;
            dlg.focus = NF_NOTES;
            app.modal = Modal::Net(dlg);
        }
        _ => {}
    }
}

// ── Confirm ───────────────────────────────────────────────────────────────────
fn on_confirm(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y')|KeyCode::Char('Y') => {
            let Modal::Confirm(ref dlg) = app.modal else { return };
            match dlg.kind {
                ConfirmKind::DelNet => {
                    if let Some(i) = app.ni() {
                        app.data.nets.remove(i);
                        let new = if app.data.nets.is_empty() { None }
                        else { Some(i.saturating_sub(1).min(app.data.nets.len()-1)) };
                        app.net_ls.select(new);
                        app.ses_ls.select(None);
                        app.log_ls.select(None);
                        save_data(&app.data);
                    }
                }
                ConfirmKind::DelSession => {
                    if let (Some(ni), Some(si)) = (app.ni(), app.si()) {
                        app.data.nets[ni].sessions.remove(si);
                        let len = app.data.nets[ni].sessions.len();
                        app.ses_ls.select(if len==0{None}else{Some(si.saturating_sub(1).min(len-1))});
                        app.log_ls.select(None);
                        save_data(&app.data);
                    }
                }
                ConfirmKind::DelCi => {
                    if let (Some(ni), Some(si), Some(ci)) = (app.ni(), app.si(), app.ci()) {
                        app.data.nets[ni].sessions[si].checkins.remove(ci);
                        let len = app.data.nets[ni].sessions[si].checkins.len();
                        app.log_ls.select(if len==0{None}else{Some(ci.saturating_sub(1).min(len-1))});
                        save_data(&app.data);
                    }
                }
            }
            app.modal = Modal::None;
        }
        KeyCode::Char('n')|KeyCode::Char('N')|KeyCode::Esc => { app.modal = Modal::None; }
        _ => {}
    }
}

// ── Theme picker ─────────────────────────────────────────────────────────────
fn on_theme_picker(app: &mut App, key: KeyCode) {
    let Modal::ThemePicker(ref mut d) = app.modal else { return };
    let len = d.themes.len();
    let vis = 16usize;
    match key {
        KeyCode::Esc => { app.modal = Modal::None; }
        KeyCode::Up => {
            if d.sel > 0 { d.sel -= 1; }
            if d.sel < d.offset { d.offset = d.sel; }
        }
        KeyCode::Down => {
            if d.sel + 1 < len { d.sel += 1; }
            if d.sel >= d.offset + vis { d.offset = d.sel - vis + 1; }
        }
        KeyCode::PageUp => {
            d.sel = d.sel.saturating_sub(vis);
            if d.sel < d.offset { d.offset = d.sel; }
        }
        KeyCode::PageDown => {
            d.sel = (d.sel + vis).min(len.saturating_sub(1));
            if d.sel >= d.offset + vis { d.offset = d.sel - vis + 1; }
        }
        KeyCode::Enter => {
            // Apply chosen theme
            let chosen = d.themes[d.sel].clone();
            app.data.theme_name = chosen.name.clone();
            app.theme = chosen;
            save_data(&app.data);
            app.modal = Modal::None;
        }
        _ => {}
    }
}

// ── Export ────────────────────────────────────────────────────────────────────
/// Open the filename dialog — actual write happens in commit_export.
fn do_export(app: &mut App) {
    let net = match app.net() { Some(n) => n, None => return };
    let safe_name: String = net.name
    .chars()
    .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
    .collect();
    let date_part = app.active_session()
    .map(|s| s.date.clone())
    .unwrap_or_else(|| "undated".to_string());
    let default_name = format!("netlog_{}_{}.txt", safe_name, date_part);
    app.modal = Modal::Export(ExportDlg::new(&default_name));
}

fn on_export_dlg(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => { app.modal = Modal::None; }
        KeyCode::Enter => { commit_export(app); }
        KeyCode::Backspace => {
            if let Modal::Export(ref mut d) = app.modal { d.filename.pop(); }
        }
        KeyCode::Delete => {
            if let Modal::Export(ref mut d) = app.modal { d.filename.clear(); }
        }
        KeyCode::Char(c) => {
            if let Modal::Export(ref mut d) = app.modal {
                if d.filename.len() < 80 { d.filename.push(c); }
            }
        }
        _ => {}
    }
}

fn commit_export(app: &mut App) {
    let Modal::Export(ref d) = app.modal else { return };
    let fname = d.filename.trim().to_string();
    if fname.is_empty() { return; }
    let net = match app.net() { Some(n) => n.clone(), None => return };
    let ses = match app.active_session() { Some(s) => s.clone(), None => return };
    let path = home_dir().join(&fname);

    let mut ls: Vec<String> = vec![];
    ls.push("=".repeat(60));
    ls.push("  NET CONTROL LOG EXPORT".into());
    ls.push("=".repeat(60));
    if !app.data.operator_call.is_empty() {
        ls.push(format!("  Net Control : {} ({})",
                        app.data.operator_call, app.data.operator_name));
    }
    ls.push(format!("  Net    : {}", net.name));
    if !net.club.is_empty() {
        ls.push(format!("  Club   : {}", net.club));
    }
    ls.push(format!("  Freq   : {} MHz   Offset: {}   PL: {}",
                    net.freq, net.offset, net.pl));
    ls.push(format!("  Date   : {}   Time: {}", ses.date, ses.net_time));
    if net.digital {
        let mut ml = format!("  Mode   : DIGITAL -- {}", net.mode);
        if !net.mode_notes.is_empty() { ml.push_str(&format!("  ({})", net.mode_notes)); }
        ls.push(ml);
    } else {
        ls.push("  Type   : Voice".into());
    }
    ls.push(format!("  Total  : {} check-ins", ses.checkins.len()));
    ls.push("=".repeat(60));
    ls.push(format!("{:>3}  {:<7} {:<12} {:<22} REMARKS",
                    "#", "TIME", "CALLSIGN", "NAME"));
    ls.push("-".repeat(70));
    for (i, ci) in ses.checkins.iter().enumerate() {
        ls.push(format!("{:>3}  {:<7} {:<12} {:<22} {}",
                        i + 1, ci.time, ci.callsign,
                        if ci.name.is_empty() { "--" } else { &ci.name },
                            ci.remarks));
    }
    ls.push("-".repeat(70));
    ls.push(format!("Exported: {}",
                    Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    let content = ls.join("\n") + "\n";

    match std::fs::write(&path, content) {
        Ok(_) => app.modal = Modal::Msg(MsgDlg {
            title: "EXPORT OK".into(),
                                        msg:   format!("Saved: {}", path.display()),
        }),
        Err(e) => app.modal = Modal::Msg(MsgDlg {
            title: "EXPORT FAILED".into(),
                                         msg:   e.to_string(),
        }),
    }
}

// ── UI root ───────────────────────────────────────────────────────────────────
fn ui(f: &mut Frame, app: &mut App) {
    let t = app.theme.clone();
    let t = &t;
    let area = f.size();
    f.render_widget(Block::default().style(Style::default().bg(t.bg())), area);
    if area.height<20||area.width<60 {
        f.render_widget(Paragraph::new(
            "Terminal too small! Please resize to at least 60×20.").style(t.red_s()), area);
        return;
    }

    // Always draw main layout underneath
    let vlay = Layout::vertical([
        Constraint::Length(2),
                                Constraint::Min(1),
                                Constraint::Length(2),
    ]).split(area);
    draw_header(f, vlay[0], app, t);
    draw_body(f, vlay[1], app, t);
    draw_status(f, vlay[2], t);
    draw_modal(f, area, app, t);
}

// ── Header ────────────────────────────────────────────────────────────────────
fn draw_header(f: &mut Frame, area: Rect, app: &App, t: &Theme) {
    let title = " NET CONTROL ◈ ";
    let op    = app.op_str();
    let clock = &app.clock;

    // Fill entire header row with header style
    f.render_widget(Block::default().style(t.hdr()), Rect{height:1,..area});

    let left = format!("{}{}", title, if op.is_empty() { String::new() } else { format!("{}  ", op) });
    let right = format!("  {}  ", clock);
    let mid_w = (area.width as usize).saturating_sub(left.len() + right.len());

    let line = Line::from(vec![
        Span::styled(&left,  t.hdr()),
                          Span::styled(" ".repeat(mid_w), t.hdr()),
                          Span::styled(&right, t.hdr()),
    ]);
    f.render_widget(Paragraph::new(line).style(t.hdr()), Rect{height:1,..area});
    f.render_widget(Block::default().borders(Borders::BOTTOM).border_style(t.border()), area);
}

// ── Body ──────────────────────────────────────────────────────────────────────
fn draw_body(f: &mut Frame, area: Rect, app: &mut App, t: &Theme) {
    // Clamp panel_w to sensible bounds relative to current terminal width
    let min_pw: u16 = 10;
    let max_pw: u16 = area.width.saturating_sub(20).max(min_pw);
    let pw = app.panel_w.clamp(min_pw, max_pw);
    let [left, right] = Layout::horizontal([
        Constraint::Length(pw), Constraint::Min(1)
    ]).split(area)[..2] else { return };
    draw_nets(f, left, app, t);
    draw_right(f, right, app, t);
}

fn draw_nets(f: &mut Frame, area: Rect, app: &mut App, t: &Theme) {
    let focused = app.focus==Focus::Nets;
    let ttl = if focused {"▶ NETS"} else {" NETS"};
    let blk = Block::default()
    .title(Span::styled(ttl, if focused{t.bold()}else{t.dim()}))
    .borders(Borders::ALL).border_type(BorderType::Plain)
    .border_style(if focused{t.borderf()}else{t.border()})
    .style(t.normal());
    let inner = blk.inner(area);
    f.render_widget(blk, area);

    if app.data.nets.is_empty() {
        f.render_widget(Paragraph::new(vec![
            Line::from(Span::styled("No nets saved.", t.dim())),
                                       Line::from(""),
                                       Line::from(Span::styled("Press [n] to add.", t.dim())),
        ]).style(t.normal()), inner);
    } else {
        let max_nm = (inner.width as usize).saturating_sub(10);
        let items: Vec<ListItem> = app.data.nets.iter().map(|n| {
            let tag = if n.digital{"◆"}else{" "};
            let nm  = &n.name[..n.name.len().min(max_nm)];
            // Show club in dim if present
            let club_part = if !n.club.is_empty() {
                let max_c = (inner.width as usize).saturating_sub(nm.len()+8);
                format!(" [{}]", &n.club[..n.club.len().min(max_c)])
            } else { String::new() };
            let cnt = n.sessions.len();
            let total_ci = n.total_checkins();
            ListItem::new(Line::from(vec![
                Span::styled(format!("{}{}", tag, nm), t.normal()),
                                     Span::styled(club_part, t.dim()),
                                     Span::styled(format!(" {}s/{}", cnt, total_ci), t.dim()),
            ]))
        }).collect();
        let list = List::new(items)
        .highlight_style(if focused{t.sel()}else{t.bold()})
        .highlight_symbol("► ");
        f.render_stateful_widget(list, inner, &mut app.net_ls);
    }
}

fn draw_right(f: &mut Frame, area: Rect, app: &mut App, t: &Theme) {
    if app.net().is_none() {
        let blk = Block::default().title(" NET INFO / LOG ")
        .borders(Borders::ALL).border_style(t.border()).style(t.normal());
        let inner = blk.inner(area);
        f.render_widget(blk, area);
        // Show logo centred in the empty panel
        let logo_w = LOGO.iter().map(|l| l.len()).max().unwrap_or(0) as u16;
        let logo_h = LOGO.len() as u16;
        if inner.width >= logo_w && inner.height >= logo_h + 3 {
            let lx = inner.x + inner.width.saturating_sub(logo_w) / 2;
            let ly = inner.y + inner.height.saturating_sub(logo_h + 3) / 2;
            for (i, line) in LOGO.iter().enumerate() {
                let row = ly + i as u16;
                if row >= inner.y + inner.height { break; }
                let color = match i {
                    0..=5  => t.amber_s(),
                    7..=12 => t.bold(),
                    _      => t.dim(),
                };
                f.render_widget(
                    Paragraph::new(Span::styled(*line, color)),
                                Rect { x: lx, y: row, width: logo_w.min(inner.width), height: 1 },
                );
            }
            let hint_y = ly + logo_h + 1;
            if hint_y < inner.y + inner.height {
                let hint = "Add or select a net on the left to begin logging.";
                let hx = inner.x + inner.width.saturating_sub(hint.len() as u16) / 2;
                f.render_widget(
                    Paragraph::new(Span::styled(hint, t.dim())),
                                Rect { x: hx, y: hint_y, width: inner.width, height: 1 },
                );
            }
        } else {
            f.render_widget(Paragraph::new(vec![
                Line::from(Span::styled("No net selected.", t.dim())),
                                           Line::from(""),
                                           Line::from(Span::styled("Add or select a net on the left.", t.dim())),
            ]).style(t.normal()), inner);
        }
        return;
    }
    let is_dig  = app.net().map_or(false,|n|n.digital);
    let has_club= app.net().map_or(false,|n|!n.club.is_empty());
    // info height: base 4 rows + 1 if digital + 1 if club + 2 (borders)
    let info_h = 4u16
    + if is_dig   {1} else {0}
    + if has_club {1} else {0}
    + 2;
    // Split: info bar | sessions list | log (log only visible when Focus::Log)
    let show_log = app.focus == Focus::Log;
    let avail = area.height.saturating_sub(info_h);
    let ses_h = if show_log {
        // Clamp ses_pane_h so log always gets at least 4 rows
        app.ses_pane_h.clamp(3, avail.saturating_sub(4))
    } else {
        avail
    };
    let constraints = if show_log {
        vec![
            Constraint::Length(info_h),
            Constraint::Length(ses_h),
            Constraint::Min(4),
        ]
    } else {
        vec![
            Constraint::Length(info_h),
            Constraint::Min(4),
        ]
    };
    let chunks = Layout::vertical(constraints).split(area);
    draw_net_info(f, chunks[0], app, t);
    draw_sessions(f, chunks[1], app, t);
    if show_log {
        draw_log(f, chunks[2], app, t);
    }
}

fn draw_net_info(f: &mut Frame, area: Rect, app: &App, t: &Theme) {
    let net = match app.net() { Some(n)=>n, None=>return };
    let blk = Block::default().title(" NET INFO ")
    .borders(Borders::ALL).border_style(t.border()).style(t.normal());
    let inner = blk.inner(area);
    f.render_widget(blk, area);
    if inner.height==0 { return; }

    let ses_count = net.sessions.len();
    let ci_count  = net.total_checkins();
    let count_str = format!("{} session{} / {} check-in{}",
                            ses_count, if ses_count==1{""} else {"s"},
                            ci_count,  if ci_count==1{""} else {"s"});
    let mut rows: Vec<Line> = vec![];

    // Row: freq + offset
    rows.push(Line::from(vec![
        Span::styled("FREQ: ",t.cyan_s()),
                         Span::styled(format!("{} MHz",net.freq),t.amber_s()),
                         Span::raw("   "),
                         Span::styled("OFFSET: ",t.cyan_s()),
                         Span::styled(net.offset.clone(),t.blue_s()),
    ]));
    // Row: PL only (date/time now per-session)
    rows.push(Line::from(vec![
        Span::styled("PL: ",t.cyan_s()),
                         Span::styled(net.pl.clone(),t.blue_s()),
    ]));
    // Club row (if set)
    if !net.club.is_empty() {
        rows.push(Line::from(vec![
            Span::styled("CLUB: ",t.cyan_s()),
                             Span::styled(net.club.clone(),t.pink_s()),
        ]));
    }
    // Mode / voice row
    if net.digital {
        rows.push(Line::from(vec![
            Span::styled("MODE: ",t.cyan_s()),
                             Span::styled(format!("◆ {}",net.mode),t.amber_s()),
                             if !net.mode_notes.is_empty() {
                                 Span::styled(format!("  ({})",net.mode_notes),t.dim())
                             } else { Span::raw("") },
        ]));
    } else {
        rows.push(Line::from(Span::styled("VOICE NET",t.dim())));
    }
    // Net name + badge + count (last row)
    let badge = if net.digital {" ◆ DIGITAL"} else {""};
    let pad = inner.width.saturating_sub(
        (net.name.len()+badge.len()+count_str.len()+2) as u16) as usize;
        rows.push(Line::from(vec![
            Span::styled(net.name.clone(),t.green_s()),
                             Span::styled(badge,t.amber_s()),
                             Span::raw(" ".repeat(pad)),
                             Span::styled(count_str,t.amber_s()),
        ]));

        f.render_widget(Paragraph::new(rows).style(t.normal()), inner);
}

fn draw_sessions(f: &mut Frame, area: Rect, app: &mut App, t: &Theme) {
    let focused = app.focus == Focus::Sessions;
    let ttl = if focused { "▶ SESSIONS" } else { " SESSIONS" };
    let blk = Block::default()
    .title(Span::styled(ttl, if focused { t.bold() } else { t.dim() }))
    .borders(Borders::ALL).border_type(BorderType::Plain)
    .border_style(if focused { t.borderf() } else { t.border() })
    .style(t.normal());
    let inner = blk.inner(area);
    f.render_widget(blk, area);

    let net = match app.net() { Some(n) => n, None => return };

    if net.sessions.is_empty() {
        f.render_widget(Paragraph::new(vec![
            Line::from(Span::styled("No sessions yet.", t.dim())),
                                       Line::from(Span::styled("Press [n] to start a new session.", t.dim())),
        ]).style(t.normal()), inner);
    } else {
        let items: Vec<ListItem> = net.sessions.iter().map(|s| {
            ListItem::new(Span::styled(s.label(), t.normal()))
        }).collect();
        let list = List::new(items)
        .highlight_style(if focused { t.sel() } else { t.bold() })
        .highlight_symbol("► ");
        f.render_stateful_widget(list, inner, &mut app.ses_ls);
    }

}

fn draw_log(f: &mut Frame, area: Rect, app: &mut App, t: &Theme) {
    let focused = app.focus == Focus::Log;
    let ttl = if focused{"▶ CHECK-IN LOG"}else{" CHECK-IN LOG"};
    let blk = Block::default()
    .title(Span::styled(ttl,if focused{t.bold()}else{t.dim()}))
    .borders(Borders::ALL).border_type(BorderType::Plain)
    .border_style(if focused{t.borderf()}else{t.border()})
    .style(t.normal());
    let inner = blk.inner(area);
    f.render_widget(blk, area);
    if inner.height<3 { return; }

    let hdr = Rect{y:inner.y,height:1,..inner};
    f.render_widget(Paragraph::new(Line::from(vec![
        Span::styled(format!("{:>3} ","#"),        t.bold()),
                                              Span::styled(format!("{:<7}","TIME"),      t.bold()),
                                              Span::styled(format!("{:<13}","CALLSIGN"), t.bold()),
                                              Span::styled(format!("{:<20}","NAME"),     t.bold()),
                                              Span::styled("REMARKS",                    t.bold()),
    ])), hdr);
    f.render_widget(Block::default().borders(Borders::BOTTOM).border_style(t.border()),
                    Rect{y:inner.y+1,height:1,..inner});

    let list_area = Rect{y:inner.y+2,height:inner.height.saturating_sub(3),..inner};

    // Show session date/time as subtitle
    if let Some(ses) = app.active_session() {
        let sub = format!(" {} {} ", ses.date, ses.net_time);
        let sub_area = Rect { x: area.x+2, y: area.y, width: sub.len() as u16, height: 1 };
        f.render_widget(Paragraph::new(Span::styled(sub, t.amber_s())), sub_area);
    }

    let checkins: Vec<CheckIn> = app.active_session()
    .map(|s| s.checkins.clone())
    .unwrap_or_default();

    if checkins.is_empty() {
        f.render_widget(Paragraph::new(
            Span::styled("No check-ins yet.  Press [c] to add.",t.dim())), list_area);
        return;
    }

    let rw = list_area.width as usize;
    let items: Vec<ListItem> = checkins.iter().enumerate().map(|(i,ci)|{
        let rem_w = rw.saturating_sub(3+1+7+13+20);
        let rem = &ci.remarks[..ci.remarks.len().min(rem_w)];
        let nm  = if ci.name.is_empty(){"—"}else{&ci.name};
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:>3} ",i+1),        t.dim()),
                                 Span::styled(format!("{:<7}",ci.time),     t.time_s()),
                                 Span::styled(format!("{:<13}",ci.callsign),t.call()),
                                 Span::styled(format!("{:<20}",nm),         t.normal()),
                                 Span::styled(rem.to_string(),               t.dim()),
        ]))
    }).collect();

    let list = List::new(items)
    .highlight_style(if focused{t.sel()}else{t.bold()})
    .highlight_symbol("► ");
    f.render_stateful_widget(list, list_area, &mut app.log_ls);

}

fn draw_status(f: &mut Frame, area: Rect, t: &Theme) {
    let blk = Block::default().borders(Borders::TOP).border_style(t.border()).style(t.normal());
    let inner = blk.inner(area);
    f.render_widget(blk, area);
    f.render_widget(Paragraph::new(Span::styled(
        " [?] Help   [q] Quit",
        t.dim())), inner);
}

// ── Modals ────────────────────────────────────────────────────────────────────
fn draw_modal(f: &mut Frame, area: Rect, app: &App, t: &Theme) {
    match &app.modal {
        Modal::None     => {}
        Modal::Operator(d) => draw_operator_dlg(f, area, d, t),
        Modal::Net(d)   => draw_net_dlg(f, area, d, t),
        Modal::Ci(d)    => draw_ci_dlg(f, area, d, t),
        Modal::Picker{dlg,pick} => {
            draw_net_dlg(f, area, dlg, t);
            draw_picker(f, area, pick, t);
        }
        Modal::Confirm(d) => draw_confirm(f, area, &d.msg, t),
        Modal::Msg(d)     => draw_msg(f, area, &d.title, &d.msg, t),
        Modal::Export(d)  => draw_export_dlg(f, area, d, t),
        Modal::ThemePicker(d) => draw_theme_picker(f, area, d, t),
        Modal::QuitConfirm    => draw_quit_confirm(f, area, t),
        Modal::Help           => draw_help(f, area, t),
    }
}

fn centered(w: u16, h: u16, area: Rect) -> Rect {
    Rect {
        x: area.x + area.width.saturating_sub(w)/2,
        y: area.y + area.height.saturating_sub(h)/2,
        width:  w.min(area.width),
        height: h.min(area.height),
    }
}

fn draw_operator_dlg(f: &mut Frame, area: Rect, d: &OperatorDlg, t: &Theme) {
    // On startup (required=true), show full-screen splash with logo.
    // On profile-edit (required=false), show a compact centered dialog.
    if d.required {
        draw_operator_splash(f, area, d, t);
    } else {
        draw_operator_edit(f, area, d, t);
    }
}

fn draw_operator_splash(f: &mut Frame, area: Rect, d: &OperatorDlg, t: &Theme) {
    // Fill background
    f.render_widget(Block::default().style(Style::default().bg(t.bg())), area);

    // Logo — centred horizontally, near the top
    let logo_w = LOGO.iter().map(|l| l.len()).max().unwrap_or(0) as u16;
    let logo_h = LOGO.len() as u16;
    let logo_x = area.x + area.width.saturating_sub(logo_w) / 2;
    let logo_y = area.y + 1;
    for (i, line) in LOGO.iter().enumerate() {
        let row = logo_y + i as u16;
        if row >= area.y + area.height { break; }
        let color = match i {
            0..=5  => t.amber_s(),  // NET CTRL block — peach
            7..=12 => t.bold(), // HAM RAD block — lavender
            14     => t.dim(),  // tagline — overlay
            _      => t.dim(),
        };
        f.render_widget(
            Paragraph::new(Span::styled(*line, color)),
                        Rect { x: logo_x, y: row, width: logo_w.min(area.width), height: 1 },
        );
    }

    // Form box below logo
    let form_y = logo_y + logo_h + 1;
    let form_h = 12u16;
    let form_w = 52u16;
    let r = centered(form_w, form_h, Rect {
        y: form_y,
        height: area.height.saturating_sub(form_y - area.y),
                     ..area
    });
    if r.height < 4 { return; }
    f.render_widget(Clear, r);
    let blk = Block::default()
    .title(Span::styled(" OPERATOR PROFILE ", t.pink_s().add_modifier(Modifier::BOLD)))
    .borders(Borders::ALL).border_style(t.pink_s())
    .style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled("Operator Callsign:", t.cyan_s())),
        Line::from(Span::styled(
            format!(" {}{}", d.fields[OF_CALL], if d.focus==OF_CALL{"_"}else{""}),
                if d.focus==OF_CALL{t.sel()}else{t.normal()})),
                    Line::from(""),
                    Line::from(Span::styled("Operator Name:", t.cyan_s())),
                    Line::from(Span::styled(
                        format!(" {}{}", d.fields[OF_NAME], if d.focus==OF_NAME{"_"}else{""}),
                            if d.focus==OF_NAME{t.sel()}else{t.normal()})),
                                Line::from(""),
                                Line::from(Span::styled(
                                    "[ENTER] next/confirm", t.dim())),
    ];
    f.render_widget(Paragraph::new(lines).style(t.normal()), inner);
}

fn draw_operator_edit(f: &mut Frame, area: Rect, d: &OperatorDlg, t: &Theme) {
    let r = centered(56, 14, area);
    f.render_widget(Clear, r);
    let blk = Block::default()
    .title(Span::styled(" EDIT OPERATOR PROFILE ", t.pink_s().add_modifier(Modifier::BOLD)))
    .borders(Borders::ALL).border_style(t.pink_s())
    .style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled("Operator Callsign:", t.cyan_s())),
        Line::from(Span::styled(
            format!(" {}{}", d.fields[OF_CALL], if d.focus==OF_CALL{"_"}else{""}),
                if d.focus==OF_CALL{t.sel()}else{t.normal()})),
                    Line::from(""),
                    Line::from(Span::styled("Operator Name:", t.cyan_s())),
                    Line::from(Span::styled(
                        format!(" {}{}", d.fields[OF_NAME], if d.focus==OF_NAME{"_"}else{""}),
                            if d.focus==OF_NAME{t.sel()}else{t.normal()})),
                                Line::from(""),
                                Line::from(""),
                                Line::from(Span::styled(
                                    "[↑↓/ENTER] navigate  [ENTER on Name] confirm  [ESC] cancel",
                                    t.dim())),
    ];
    f.render_widget(Paragraph::new(lines).style(t.normal()), inner);
}

fn draw_net_dlg(f: &mut Frame, area: Rect, d: &NetDlg, t: &Theme) {
    let title = if d.mode==NdMode::Add {" ADD NET "} else {" EDIT NET "};
    let dh = 30u16 + if d.digital {6} else {0};
    let r = centered(66, dh, area);
    f.render_widget(Clear, r);
    let blk = Block::default().title(Span::styled(title,t.bold()))
    .borders(Borders::ALL).border_style(t.bold()).style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    // labels and field indices aligned with NF_* constants
    // fields array: [name, club, freq, offset, pl, date, time]
    let field_defs: &[(&str, usize)] = &[
        ("Net Name",          NF_NAME),
        ("Club / Association",NF_CLUB),
        ("Frequency (MHz)",   NF_FREQ),
        ("Offset",            NF_OFFSET),
        ("PL Tone (Hz)",      NF_PL),
        ("Date (YYYY-MM-DD)", NF_DATE),
        ("Time (HH:MM)",      NF_TIME),
    ];

    let mut lines: Vec<Line> = vec![Line::from("")];
    for &(lbl, fi) in field_defs {
        lines.push(Line::from(Span::styled(format!("{}:",lbl), t.cyan_s())));
        let cur = if d.focus==fi{"_"}else{""};
        let val = format!(" {}{}",&d.fields[fi],cur);
        let attr = if d.focus==fi{t.sel()}else{t.normal()};
        // Club field gets pink accent when populated
        let attr = if fi==NF_CLUB && !d.fields[NF_CLUB].is_empty() && d.focus!=fi {
            t.pink_s()
        } else { attr };
        lines.push(Line::from(Span::styled(val, attr)));
        lines.push(Line::from(""));
    }
    // Digital toggle
    let tv = if d.digital{"[ YES ]"}else{"[  NO ]"};
    let tc = if d.digital{t.green_s()}else{t.dim()};
    let ta = if d.focus==NF_TOGGLE{t.sel()}else{t.normal()};
    lines.push(Line::from(vec![
        Span::styled("  [DIGITAL NET]: ",ta),
                          Span::styled(tv, tc.add_modifier(Modifier::BOLD)),
                          Span::styled("  (SPACE to toggle)",t.dim()),
    ]));
    lines.push(Line::from(""));
    if d.digital {
        lines.push(Line::from(Span::styled("Digital Mode:",t.cyan_s())));
        let cm = DIGITAL_MODES[d.mode_idx];
        let ma = if d.focus==NF_MODE{t.sel()}else{t.amber_s()};
        let mh = if d.focus==NF_MODE{"  [ENTER to open picker]"}else{""};
        lines.push(Line::from(vec![
            Span::styled(format!(" ▶ {}",cm),ma),
                              Span::styled(mh,t.dim()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Mode Notes / Params:",t.cyan_s())));
        lines.push(Line::from(Span::styled(
            format!(" {}{}",&d.notes,if d.focus==NF_NOTES{"_"}else{""}),
                if d.focus==NF_NOTES{t.sel()}else{t.normal()})));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        "[↑↓] navigate  [ENTER] next/confirm  [ESC] cancel",t.dim())));
    f.render_widget(Paragraph::new(lines).style(t.normal()).wrap(Wrap{trim:false}), inner);
}

fn draw_ci_dlg(f: &mut Frame, area: Rect, d: &CiDlg, t: &Theme) {
    let has_comp = !d.comp_labels.is_empty();
    let comp_h   = if has_comp { d.comp_labels.len().min(6) as u16 + 2 } else { 0 };
    let dh = 18u16 + comp_h;
    let r  = centered(58, dh, area);
    f.render_widget(Clear, r);

    let title = if d.fcc_pending { " ADD CHECK-IN  [Searching…] " } else { " ADD CHECK-IN " };
    let title_style = if d.fcc_pending { t.amber_s() } else { t.bold() };
    let blk = Block::default()
    .title(Span::styled(title, title_style))
    .borders(Borders::ALL).border_style(t.bold()).style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let labels = ["Callsign","Name","Remarks"];
    let vals: [&str;3] = [&d.callsign, &d.name, &d.remarks];
    let mut lines: Vec<Line> = vec![Line::from("")];

    for (i, (lbl, val)) in labels.iter().zip(vals.iter()).enumerate() {
        lines.push(Line::from(Span::styled(format!("{}:", lbl), t.cyan_s())));
        let cur = if d.focus == i { "_" } else { "" };
        lines.push(Line::from(Span::styled(format!(" {}{}", val, cur),
                                           if d.focus == i { t.sel() } else { t.normal() })));
        lines.push(Line::from(""));
        // Insert completion dropdown right after the callsign field
        if i == 0 && has_comp {
            for (ci, lbl) in d.comp_labels.iter().enumerate() {
                let is_sel = d.comp_sel == Some(ci);
                lines.push(Line::from(vec![
                    Span::styled("  ", t.dim()),
                                      Span::styled(
                                          format!("{:<50}", lbl),
                                              if is_sel { t.sel() } else { t.dim() },
                                      ),
                ]));
            }
            lines.push(Line::from(Span::styled(
                "  [TAB/ENTER] apply  [↑↓] cycle", t.dim())));
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(Span::styled(
        "[↑↓] navigate  [TAB] complete/search  [ENTER] confirm  [ESC] cancel",
        t.dim())));
    f.render_widget(Paragraph::new(lines).style(t.normal()), inner);
}

fn draw_picker(f: &mut Frame, area: Rect, p: &ModePick, t: &Theme) {
    let vis = 12usize;
    let mw  = DIGITAL_MODES.iter().map(|m|m.len()).max().unwrap_or(10);
    let r   = centered((mw+10) as u16, (vis+4) as u16, area);
    f.render_widget(Clear, r);
    let blk = Block::default().title(Span::styled(" SELECT DIGITAL MODE ",t.bold()))
    .borders(Borders::ALL).border_style(t.bold()).style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let items: Vec<ListItem> = DIGITAL_MODES.iter().enumerate()
    .skip(p.offset).take(vis)
    .map(|(i,m)| {
        if i==p.sel {
            ListItem::new(Line::from(vec![
                Span::styled("▶ ",t.amber_s()),
                                     Span::styled(*m,t.sel()),
            ]))
        } else {
            ListItem::new(Span::styled(format!("  {}",m),t.normal()))
        }
    }).collect();

    let mut ls = ListState::default();
    ls.select(Some(p.sel.saturating_sub(p.offset)));
    f.render_stateful_widget(List::new(items).style(t.normal()), inner, &mut ls);

    if p.offset > 0 {
        let top_ind=Rect{x:r.x+r.width-3,y:r.y+1,width:2,height:1};
        f.render_widget(Paragraph::new(Span::styled("▲",t.dim())),top_ind);
    }
    if p.offset+vis < DIGITAL_MODES.len() {
        let b=Rect{x:r.x+r.width-3,y:r.y+r.height-2,width:2,height:1};
        f.render_widget(Paragraph::new(Span::styled("▼",t.dim())),b);
    }
    let h=Rect{x:inner.x,y:inner.y+inner.height.saturating_sub(1),width:inner.width,height:1};
    f.render_widget(Paragraph::new(Span::styled(
        "[↑↓] scroll  [ENTER] pick  [ESC] cancel",t.dim())),h);
}

fn draw_export_dlg(f: &mut Frame, area: Rect, d: &ExportDlg, t: &Theme) {
    let dw = 62u16;
    let dh = 9u16;
    let r  = centered(dw, dh, area);
    f.render_widget(Clear, r);
    let blk = Block::default()
    .title(Span::styled(" SAVE EXPORT — Enter filename ", t.bold()))
    .borders(Borders::ALL).border_style(t.bold())
    .style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let cursor = "_";
    let field  = format!(" {}{}", d.filename, cursor);
    let lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled("Filename (saved to home directory):", t.cyan_s())),
        Line::from(""),
        Line::from(Span::styled(field, t.sel())),
        Line::from(""),
        Line::from(Span::styled(
            "[ENTER] save   [ESC] cancel   [DEL] clear",
            t.dim())),
    ];
    f.render_widget(Paragraph::new(lines).style(t.normal()), inner);
}

fn draw_theme_picker(f: &mut Frame, area: Rect, d: &ThemePickerDlg, t: &Theme) {
    let vis  = 16usize;
    let max_nm = d.themes.iter().map(|th| th.name.len()).max().unwrap_or(20);
    let dw = (max_nm as u16 + 10).max(36);
    let dh = (vis + 5) as u16;
    let r  = centered(dw.min(area.width), dh.min(area.height), area);
    f.render_widget(Clear, r);
    let blk = Block::default()
    .title(Span::styled(" SELECT THEME ", t.bold()))
    .borders(Borders::ALL).border_style(t.bold())
    .style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let items: Vec<ListItem> = d.themes.iter().enumerate()
    .skip(d.offset).take(vis)
    .map(|(i, th)| {
        if i == d.sel {
            ListItem::new(Line::from(vec![
                Span::styled("▶ ", t.amber_s()),
                                     Span::styled(th.name.clone(), t.sel()),
            ]))
        } else {
            ListItem::new(Span::styled(format!("  {}", th.name), t.normal()))
        }
    }).collect();

    let mut ls = ListState::default();
    ls.select(Some(d.sel.saturating_sub(d.offset)));
    f.render_stateful_widget(List::new(items).style(t.normal()), inner, &mut ls);

    if d.offset > 0 {
        let top = Rect { x: r.x + r.width - 3, y: r.y + 1, width: 2, height: 1 };
        f.render_widget(Paragraph::new(Span::styled("▲", t.dim())), top);
    }
    if d.offset + vis < d.themes.len() {
        let bot = Rect { x: r.x + r.width - 3, y: r.y + r.height - 2, width: 2, height: 1 };
        f.render_widget(Paragraph::new(Span::styled("▼", t.dim())), bot);
    }
    let hint_area = Rect { x: inner.x, y: inner.y + inner.height.saturating_sub(1),
        width: inner.width, height: 1 };
        f.render_widget(Paragraph::new(Span::styled(
            "[↑↓] scroll  [ENTER] apply  [ESC] cancel", t.dim())), hint_area);
}

fn draw_help(f: &mut Frame, area: Rect, t: &Theme) {
    let dw = 62u16.min(area.width);
    let dh = 36u16.min(area.height);
    let r  = centered(dw, dh, area);
    f.render_widget(Clear, r);
    let blk = Block::default()
    .title(Span::styled(" KEY BINDINGS ", t.bold()))
    .borders(Borders::ALL).border_style(t.bold())
    .style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let section = |s: &str| Line::from(vec![
        Span::styled(format!(" {}", s), t.bold()),
    ]);
    let entry = |key: &str, desc: &str| Line::from(vec![
        Span::styled(format!("  {:<18}", key), t.amber_s()),
                                                   Span::styled(desc.to_string(),         t.normal()),
    ]);
    let blank = || Line::from("");

    let lines: Vec<Line> = vec![
        blank(),
        section("NAVIGATION"),
        entry("Tab",              "Cycle focus: Nets → Sessions → Log"),
        entry("↑ / ↓",           "Move selection"),
        entry("Enter",           "Open selected item"),
        entry("Esc",             "Go back one level"),
        blank(),
        section("NETS"),
        entry("n",               "Add new net"),
        entry("e",               "Edit selected net"),
        entry("d",               "Delete selected net"),
        blank(),
        section("SESSIONS"),
        entry("n",               "New session for today"),
        entry("d",               "Delete selected session"),
        entry("Ctrl+↑ / Ctrl+↓", "Resize sessions pane"),
        blank(),
        section("CHECK-INS"),
        entry("c",               "Add check-in to active session"),
        entry("d",               "Delete selected check-in"),
        entry("Tab",             "Autocomplete callsign / search"),
        blank(),
        section("GENERAL"),
        entry("x",               "Export active session to file"),
        entry("p",               "Edit operator profile"),
        entry("t",               "Change theme"),
        entry("Ctrl+← / Ctrl+→", "Resize nets panel"),
        entry("?",               "Show this help"),
        entry("q",               "Quit"),
        blank(),
        Line::from(Span::styled("  Press any key to close", t.dim())),
    ];

    f.render_widget(Paragraph::new(lines).style(t.normal()), inner);
}

fn draw_quit_confirm(f: &mut Frame, area: Rect, t: &Theme) {
    let r = centered(24, 5, area);
    f.render_widget(Clear, r);
    let blk = Block::default()
    .title(Span::styled(" QUIT ", t.bold()))
    .borders(Borders::ALL).border_style(t.bold())
    .style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);
    f.render_widget(Paragraph::new(vec![
        Line::from(""),
                                   Line::from(Span::styled("  [Y] Yes   [N] No  ", t.normal())),
    ]).alignment(Alignment::Center).style(t.normal()), inner);
}

fn draw_confirm(f: &mut Frame, area: Rect, msg: &str, t: &Theme) {
    let dw = (msg.len()+10).max(34) as u16;
    let r = centered(dw, 7, area);
    f.render_widget(Clear, r);
    let blk = Block::default().title(Span::styled(" CONFIRM ",t.red_s()))
    .borders(Borders::ALL).border_style(t.red_s()).style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);
    f.render_widget(Paragraph::new(vec![
        Line::from(""),
                                   Line::from(Span::styled(msg,t.amber_s())),
                                   Line::from(""),
                                   Line::from(Span::styled("[Y] Yes   [N / ESC] No",t.dim())),
    ]).alignment(Alignment::Center).style(t.normal()), inner);
}

fn draw_msg(f: &mut Frame, area: Rect, title: &str, msg: &str, t: &Theme) {
    let dw = (msg.len()+10).max(34) as u16;
    let r = centered(dw, 7, area);
    f.render_widget(Clear, r);
    let blk = Block::default().title(Span::styled(format!(" {} ",title),t.green_s()))
    .borders(Borders::ALL).border_style(t.bold()).style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);
    f.render_widget(Paragraph::new(vec![
        Line::from(""),
                                   Line::from(Span::styled(msg,t.green_s())),
                                   Line::from(""),
                                   Line::from(Span::styled("Press any key...",t.dim())),
    ]).alignment(Alignment::Center).style(t.normal()), inner);
}
