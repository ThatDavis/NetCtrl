use std::path::PathBuf;

use chrono::Utc;

use crate::models::AppData;

pub fn home_dir() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."))
}

pub fn data_path() -> PathBuf { home_dir().join(".netcontrol_data.json") }

pub fn load_data() -> AppData {
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

pub fn save_data(d: &AppData) {
    if let Ok(s) = serde_json::to_string_pretty(d) { let _ = std::fs::write(data_path(), s); }
}

pub fn utc_now() -> String { Utc::now().format("%H:%Mz").to_string() }
pub fn new_id()  -> String { Utc::now().format("%Y%m%d%H%M%S%6f").to_string() }
