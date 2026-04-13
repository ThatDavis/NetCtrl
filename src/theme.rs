use ratatui::style::{Color, Modifier, Style};

use crate::persistence::home_dir;

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

pub fn parse_hex_color(h: &str) -> Color {
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
#[allow(dead_code)]  // base01/02/06/07 unused in UI but valid in custom theme files
pub struct Theme {
    pub name:   String,
    // Base16 slots
    pub base00: Color, pub base01: Color, pub base02: Color, pub base03: Color,
    pub base04: Color, pub base05: Color, pub base06: Color, pub base07: Color,
    pub base08: Color, pub base09: Color, pub base0a: Color, pub base0b: Color,
    pub base0c: Color, pub base0d: Color, pub base0e: Color, pub base0f: Color,
}

impl Theme {
    // ── Style helpers ──────────────────────────────────────────────────────
    pub fn bg(&self)     -> Color { self.base00 }
    pub fn fg(&self)     -> Color { self.base05 }
    pub fn dim_c(&self)  -> Color { self.base03 }
    pub fn bord_c(&self) -> Color { self.base04 }
    pub fn bord_f(&self) -> Color { self.base0e }
    pub fn accent(&self) -> Color { self.base0e } // mauve/purple — bold/headings
    pub fn red(&self)    -> Color { self.base08 }
    pub fn amber(&self)  -> Color { self.base09 }
    pub fn yellow(&self) -> Color { self.base0a }
    pub fn green(&self)  -> Color { self.base0b }
    pub fn cyan(&self)   -> Color { self.base0c }
    pub fn blue(&self)   -> Color { self.base0d }
    pub fn pink(&self)   -> Color { self.base0f }

    // ── Named styles ───────────────────────────────────────────────────────
    pub fn normal(&self)  -> Style { Style::default().fg(self.fg()).bg(self.bg()) }
    pub fn dim(&self)     -> Style { Style::default().fg(self.dim_c()).bg(self.bg()) }
    pub fn bold(&self)    -> Style { Style::default().fg(self.accent()).bg(self.bg()).add_modifier(Modifier::BOLD) }
    pub fn sel(&self)     -> Style { Style::default().fg(self.bg()).bg(self.accent()).add_modifier(Modifier::BOLD) }
    pub fn amber_s(&self) -> Style { Style::default().fg(self.amber()).bg(self.bg()).add_modifier(Modifier::BOLD) }
    pub fn cyan_s(&self)  -> Style { Style::default().fg(self.cyan()).bg(self.bg()) }
    pub fn red_s(&self)   -> Style { Style::default().fg(self.red()).bg(self.bg()).add_modifier(Modifier::BOLD) }
    pub fn hdr(&self)     -> Style { Style::default().fg(self.bg()).bg(self.accent()).add_modifier(Modifier::BOLD) }
    pub fn call(&self)    -> Style { Style::default().fg(self.cyan()).bg(self.bg()).add_modifier(Modifier::BOLD) }
    pub fn time_s(&self)  -> Style { Style::default().fg(self.yellow()).bg(self.bg()) }
    pub fn green_s(&self) -> Style { Style::default().fg(self.green()).bg(self.bg()) }
    pub fn blue_s(&self)  -> Style { Style::default().fg(self.blue()).bg(self.bg()) }
    pub fn border(&self)  -> Style { Style::default().fg(self.bord_c()).bg(self.bg()) }
    pub fn borderf(&self) -> Style { Style::default().fg(self.bord_f()).bg(self.bg()) }
    pub fn pink_s(&self)  -> Style { Style::default().fg(self.pink()).bg(self.bg()).add_modifier(Modifier::BOLD) }
}

// ── Built-in themes ───────────────────────────────────────────────────────────
pub fn theme_catppuccin_mocha() -> Theme {
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

pub fn theme_gruvbox_dark() -> Theme {
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

pub fn theme_nord() -> Theme {
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

pub fn theme_solarized_dark() -> Theme {
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

pub fn theme_dracula() -> Theme {
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

pub fn theme_one_dark() -> Theme {
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

pub fn theme_tokyo_night() -> Theme {
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

pub fn theme_mellow() -> Theme {
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
pub fn builtin_themes() -> Vec<Theme> {
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
pub fn load_theme_from_toml(path: &std::path::Path) -> Option<Theme> {
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
pub fn all_themes() -> Vec<Theme> {
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
