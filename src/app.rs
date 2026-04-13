use std::time::Instant;

use chrono::Utc;
use ratatui::widgets::ListState;

use crate::dialogs::{Modal, OperatorDlg, Screen};
use crate::models::{AppData, Net, Session};
use crate::persistence::load_data;
use crate::theme::{all_themes, theme_catppuccin_mocha, Theme};

// ── App ───────────────────────────────────────────────────────────────────────
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum Focus { Nets, Sessions, Log }

pub struct App {
    pub data:       AppData,
    pub screen:     Screen,
    pub focus:      Focus,
    pub net_ls:     ListState,
    pub ses_ls:     ListState,   // session list (per net)
    pub log_ls:     ListState,
    pub modal:      Modal,
    pub tick:       Instant,
    pub clock:      String,
    pub panel_w:    u16,
    pub ses_pane_h: u16,   // height of sessions pane when log is visible; resizable Ctrl+↑/↓
    pub theme:      Theme,
}
impl App {
    pub fn new() -> Self {
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
    pub fn tick(&mut self) { self.clock = Utc::now().format("%H:%M:%S UTC").to_string(); }
    pub fn net(&self)       -> Option<&Net>      { self.net_ls.selected().and_then(|i| self.data.nets.get(i)) }
    pub fn ni(&self) -> Option<usize> { self.net_ls.selected() }
    pub fn si(&self) -> Option<usize> { self.ses_ls.selected() }
    pub fn ci(&self) -> Option<usize> { self.log_ls.selected() }
    pub fn active_session(&self) -> Option<&Session> {
        let ni = self.ni()?;
        let si = self.si()?;
        self.data.nets.get(ni)?.sessions.get(si)
    }
    pub fn active_session_mut(&mut self) -> Option<&mut Session> {
        let ni = self.ni()?;
        let si = self.si()?;
        self.data.nets.get_mut(ni)?.sessions.get_mut(si)
    }
    pub fn op_str(&self) -> String {
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
