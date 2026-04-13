use std::sync::mpsc;

use crate::models::{KnownOp, Net};
use crate::theme::Theme;

// ── App screen ────────────────────────────────────────────────────────────────
// On first launch (no operator info saved) we show a startup profile screen.
#[derive(Debug,PartialEq)]
pub enum Screen { Startup, Main }

// ── Modal state ───────────────────────────────────────────────────────────────
// Net dialog field indices
pub const NF_NAME:usize=0; pub const NF_CLUB:usize=1; pub const NF_FREQ:usize=2;
pub const NF_OFFSET:usize=3; pub const NF_PL:usize=4;
pub const NF_TOGGLE:usize=5; pub const NF_MODE:usize=6;
pub const NF_NOTES:usize=7;

#[derive(Debug,PartialEq,Clone,Copy)]
pub enum NdMode { Add, Edit }

#[derive(Debug)]
pub struct NetDlg {
    pub mode:     NdMode,
    pub fields:   [String;5],  // name club freq offset pl
    pub digital:  bool,
    pub mode_idx: usize,
    pub notes:    String,
    pub focus:    usize,
    pub edit_id:  Option<String>,
}
impl NetDlg {
    pub fn new_add() -> Self {
        Self {
            mode: NdMode::Add,
            fields: [String::new(), String::new(), String::new(),
                     "+0.600".into(), "NONE".into()],
            digital: false, mode_idx: 0, notes: String::new(),
            focus: NF_NAME, edit_id: None,
        }
    }
    pub fn new_edit(n: &Net) -> Self {
        use crate::models::DIGITAL_MODES;
        let mi = DIGITAL_MODES.iter().position(|&m| m==n.mode).unwrap_or(0);
        Self {
            mode: NdMode::Edit,
            fields: [n.name.clone(), n.club.clone(), n.freq.clone(),
                     n.offset.clone(), n.pl.clone()],
            digital: n.digital, mode_idx: mi, notes: n.mode_notes.clone(),
            focus: NF_NAME, edit_id: Some(n.id.clone()),
        }
    }
    pub fn max_focus(&self) -> usize { if self.digital { NF_NOTES } else { NF_TOGGLE } }
    pub fn cur_field_mut(&mut self) -> Option<&mut String> {
        match self.focus {
            NF_NAME..=NF_PL => Some(&mut self.fields[self.focus]),
            NF_NOTES         => Some(&mut self.notes),
            _                => None,
        }
    }
    pub fn max_len(&self) -> usize {
        match self.focus {
            NF_NAME=>30, NF_CLUB=>40, NF_FREQ=>12, NF_OFFSET=>12,
            NF_PL=>10, NF_NOTES=>50, _=>0,
        }
    }
}

// Operator profile dialog field indices
pub const OF_CALL:usize=0; pub const OF_NAME:usize=1;

#[derive(Debug)]
pub struct OperatorDlg {
    pub fields: [String;2],   // callsign, name
    pub focus:  usize,
    /// true = came from startup screen (must complete), false = editing from main
    pub required: bool,
}
impl OperatorDlg {
    pub fn new(call: &str, name: &str, required: bool) -> Self {
        Self {
            fields: [call.to_string(), name.to_string()],
            focus: OF_CALL,
            required,
        }
    }
    pub fn cur_mut(&mut self) -> &mut String { &mut self.fields[self.focus] }
    pub fn max_len(&self) -> usize { if self.focus==OF_CALL { 12 } else { 40 } }
}

/// Result of an async FCC lookup.
#[derive(Debug)]
pub enum FccResult { Found(String), NotFound }

#[derive(Debug)]
pub struct CiDlg {
    pub callsign:     String,
    pub name:         String,
    pub nickname:     String,
    pub remarks:      String,
    pub focus:        usize,
    /// Filtered completions for the current callsign prefix.
    pub completions:  Vec<String>,   // (callsign, display label)
    pub comp_labels:  Vec<String>,
    pub comp_sel:     Option<usize>,
    /// Channel for FCC lookup results.
    pub fcc_rx:       Option<mpsc::Receiver<FccResult>>,
    pub fcc_pending:  bool,
}
impl CiDlg {
    pub fn new() -> Self {
        Self {
            callsign: String::new(), name: String::new(),
            nickname: String::new(), remarks: String::new(),
            focus: 0,
            completions: vec![], comp_labels: vec![], comp_sel: None,
            fcc_rx: None, fcc_pending: false,
        }
    }
    pub fn cur_mut(&mut self) -> &mut String {
        match self.focus {
            0 => &mut self.callsign,
            1 => &mut self.name,
            2 => &mut self.nickname,
            _ => &mut self.remarks,
        }
    }
    pub fn max_len(&self) -> usize { match self.focus { 0=>12, 1=>30, 2=>30, _=>50 } }
    /// Update completions from known_ops given current callsign prefix.
    pub fn update_completions(&mut self, ops: &[KnownOp]) {
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
        self.comp_labels  = matches.iter().map(|o| {
            let mut label = o.callsign.clone();
            if !o.name.is_empty() { label.push_str(&format!(" — {}", o.name)); }
            if !o.nickname.is_empty() { label.push_str(&format!(" ({})", o.nickname)); }
            label
        }).collect();
        self.comp_sel = if self.completions.is_empty() { None } else { Some(0) };
    }
    /// Apply the selected completion (fill callsign + name).
    pub fn apply_completion(&mut self, ops: &[KnownOp]) {
        let Some(idx) = self.comp_sel else { return };
        let Some(cs) = self.completions.get(idx) else { return };
        let cs = cs.clone();
        if let Some(op) = ops.iter().find(|o| o.callsign == cs) {
            self.callsign = op.callsign.clone();
            if !op.name.is_empty()     { self.name     = op.name.clone(); }
            if !op.nickname.is_empty() { self.nickname = op.nickname.clone(); }
        }
        self.completions.clear();
        self.comp_labels.clear();
        self.comp_sel = None;
    }
    /// Kick off a background FCC lookup for the current callsign.
    pub fn start_fcc_lookup(&mut self) {
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
    pub fn poll_fcc(&mut self) -> Option<FccResult> {
        let rx = self.fcc_rx.as_ref()?;
        match rx.try_recv() {
            Ok(r)  => { self.fcc_pending = false; self.fcc_rx = None; Some(r) }
            Err(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct ModePick { pub sel: usize, pub offset: usize }

#[derive(Debug,PartialEq)]
pub enum ConfirmKind { DelNet, DelSession, DelCi }
#[derive(Debug)]
pub struct ConfirmDlg { pub kind: ConfirmKind, pub msg: String }
#[derive(Debug)]
pub struct MsgDlg { pub title: String, pub msg: String }

#[derive(Debug)]
pub struct ExportDlg { pub filename: String }

#[derive(Debug)]
pub struct ThemePickerDlg {
    pub themes:  Vec<Theme>,
    pub sel:     usize,
    pub offset:  usize,
}
impl ThemePickerDlg {
    pub fn new(themes: Vec<Theme>, current: &str) -> Self {
        let sel = themes.iter().position(|t| t.name == current).unwrap_or(0);
        let offset = sel.saturating_sub(5);
        Self { themes, sel, offset }
    }
}
impl ExportDlg {
    pub fn new(default: &str) -> Self { Self { filename: default.to_string() } }
}

#[derive(Debug)]
pub struct SessionDlg {
    pub fields:   [String; 2],   // 0 = date, 1 = net_time
    pub focus:    usize,
    pub ni:       usize,         // net index
    pub si:       usize,         // session index
}
impl SessionDlg {
    pub fn new(ni: usize, si: usize, date: &str, net_time: &str) -> Self {
        Self {
            fields: [date.to_string(), net_time.to_string()],
            focus: 0, ni, si,
        }
    }
    pub fn cur_mut(&mut self) -> &mut String { &mut self.fields[self.focus] }
    pub fn max_len(&self) -> usize { if self.focus == 0 { 12 } else { 6 } }
}

#[derive(Debug)]
pub enum Modal {
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
    Session(SessionDlg),
}
