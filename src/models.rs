use serde::{Deserialize, Serialize};

use crate::persistence::new_id;

// ── Digital modes ─────────────────────────────────────────────────────────────
pub const DIGITAL_MODES: &[&str] = &[
    "FT8","FT4","JS8Call","Winlink","APRS","RTTY",
    "PSK31","OLIVIA","VARA HF","VARA FM","D-STAR",
    "DMR","System Fusion / YSF","P25","NXDN","SSTV",
    "WSPR","MSK144","Q65","OTHER",
];

// ── ASCII art logo ───────────────────────────────────────────────────────────
pub const LOGO: &[&str] = &[
    r" ░░░░    ░░░░              ░░░░   ░░░░░░░░░░   ░░░░              ░░░░ ",
    r" ░████░  ░████             ░████  ░██████████  ░████             ░████",
    r" ░██████░░████░░░░░░░░░░ ░░░████░ ░██████████░░░████░ ░░░░░░░░░░ ░████",
    r" ░████████████░██████████░████████░████  ████░████████░██████████░████",
    r" ░████████████░████░░████ ████████░████ ░░░░  ████████░██████████░████",
    r" ░████████████░██████████  ░████  ░████░░████  ░████  ░████  ████░████",
    r" ░████  ██████░████░░░░░   ░████░ ░██████████  ░████░ ░████      ░████",
    r"  ████    ████ ██████████   ██████ ██████████   ██████ ████       ████",
    r"",
    r"              Amateur Radio Net Check-in Logger  ◈  v1.0          ",
];

// ── Data ──────────────────────────────────────────────────────────────────────
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
pub struct CheckIn {
    pub id: String, pub callsign: String, pub name: String,
    #[serde(default)] pub nickname: String,
    pub remarks: String, pub time: String,
}

/// A single dated occurrence of a net.
#[derive(Debug,Clone,Serialize,Deserialize,Default)]
pub struct Session {
    pub id:       String,
    pub date:     String,
    pub net_time: String,
    #[serde(default)] pub checkins: Vec<CheckIn>,
}

impl Session {
    pub fn new_today() -> Self {
        Session {
            id:       new_id(),
            date:     chrono::Local::now().format("%Y-%m-%d").to_string(),
            net_time: chrono::Local::now().format("%H:%M").to_string(),
            checkins: vec![],
        }
    }
    pub fn label(&self) -> String {
        let cnt = self.checkins.len();
        format!("{} {:>5}  ({} check-in{})", self.date, self.net_time, cnt, if cnt==1{""} else {"s"})
    }
}

#[derive(Debug,Clone,Serialize,Deserialize,Default)]
pub struct Net {
    pub id: String, pub name: String,
    #[serde(default)] pub club: String,
    pub freq: String, pub offset: String, pub pl: String,
    // Legacy flat fields kept for migration; new data uses sessions vec.
    #[serde(default)] pub date: String,
    #[serde(default, rename="time")] pub net_time: String,
    #[serde(default)] pub digital: bool,
    #[serde(default)] pub mode: String,
    #[serde(default)] pub mode_notes: String,
    // Legacy flat check-ins; migrated to sessions on load.
    #[serde(default)] pub checkins: Vec<CheckIn>,
    #[serde(default)] pub sessions: Vec<Session>,
}

impl Net {
    /// Total check-ins across all sessions.
    pub fn total_checkins(&self) -> usize {
        self.sessions.iter().map(|s| s.checkins.len()).sum()
    }
    /// Migrate legacy flat checkins into a single session if needed.
    pub fn migrate(&mut self) {
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
pub struct KnownOp {
    pub callsign: String,
    pub name:     String,
    #[serde(default)] pub nickname: String,
}

#[derive(Debug,Serialize,Deserialize,Default)]
pub struct AppData {
    #[serde(default)] pub operator_name: String,
    #[serde(default)] pub operator_call: String,
    #[serde(default)] pub theme_name:    String,
    #[serde(default)] pub known_ops:     Vec<KnownOp>,
    pub nets: Vec<Net>,
}

impl AppData {
    /// Upsert a callsign/name/nickname into known_ops.
    pub fn remember_op(&mut self, callsign: &str, name: &str, nickname: &str) {
        let cs = callsign.trim().to_uppercase();
        let nm = name.trim().to_string();
        let nk = nickname.trim().to_string();
        if cs.is_empty() { return; }
        if let Some(op) = self.known_ops.iter_mut().find(|o| o.callsign == cs) {
            if !nm.is_empty() { op.name     = nm; }
            if !nk.is_empty() { op.nickname = nk; }
        } else {
            self.known_ops.push(KnownOp { callsign: cs, name: nm, nickname: nk });
        }
    }
}
