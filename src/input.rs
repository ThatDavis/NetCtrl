use chrono::Utc;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::{App, Focus};
use crate::dialogs::{
    CiDlg, ConfirmDlg, ConfirmKind, ExportDlg, Modal, MsgDlg,
    NetDlg, NdMode, ModePick, OperatorDlg, SessionDlg, ThemePickerDlg,
    NF_NAME, NF_CLUB, NF_FREQ, NF_OFFSET, NF_PL, NF_TOGGLE, NF_MODE, NF_NOTES,
    OF_CALL, OF_NAME,
};
use crate::models::{CheckIn, Net, Session, DIGITAL_MODES};
use crate::persistence::{home_dir, new_id, save_data, utc_now};
use crate::theme::all_themes;

// ── Input dispatch ────────────────────────────────────────────────────────────
pub fn on_key(app: &mut App, key: KeyCode, mods: KeyModifiers) -> bool {
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
                Modal::Session(_)     => on_session_dlg(app, key),
            }
        }
    }
    true
}

pub fn on_main(app: &mut App, key: KeyCode, mods: KeyModifiers) -> bool {
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

        // [e] — edit net (Nets focus) or edit session date/time (Sessions/Log focus)
        KeyCode::Char('e') => match app.focus {
            Focus::Nets => {
                if let Some(n) = app.net() { app.modal = Modal::Net(NetDlg::new_edit(n)); }
            }
            Focus::Sessions | Focus::Log => {
                if let (Some(ni), Some(si)) = (app.ni(), app.si()) {
                    let ses = &app.data.nets[ni].sessions[si];
                    app.modal = Modal::Session(SessionDlg::new(ni, si, &ses.date, &ses.net_time));
                }
            }
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
pub fn on_quit_confirm(app: &mut App, key: KeyCode) -> bool {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => return true, // signal quit
        _ => { app.modal = Modal::None; }
    }
    false
}

// ── Operator dialog ───────────────────────────────────────────────────────────
pub fn on_operator_dlg(app: &mut App, key: KeyCode) {
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

pub fn commit_operator(app: &mut App) {
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
    app.screen = crate::dialogs::Screen::Main;
    app.modal  = Modal::None;
}

// ── Net dialog ────────────────────────────────────────────────────────────────
pub fn on_net_dlg(app: &mut App, key: KeyCode) {
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

pub fn commit_net(app: &mut App) {
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
        date:       String::new(),
        net_time:   String::new(),
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
pub fn on_ci_dlg(app: &mut App, key: KeyCode) {
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
            } else if d.focus < 3 {
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
                if d.focus < 3 {
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

pub fn commit_ci(app: &mut App) {
    let Modal::Ci(ref dlg) = app.modal else { return };
    let cs = dlg.callsign.trim().to_uppercase();
    if cs.is_empty() {
        if let Modal::Ci(ref mut d)=app.modal { d.focus=0; }
        return;
    }
    let ci = CheckIn {
        id: new_id(), callsign: cs,
        name:     dlg.name.trim().into(),
        nickname: dlg.nickname.trim().into(),
        remarks:  dlg.remarks.trim().into(),
        time: utc_now(),
    };
    if let Some(ses) = app.active_session_mut() {
        ses.checkins.push(ci);
        let last = ses.checkins.len()-1;
        app.log_ls.select(Some(last));
    }
    // Remember this callsign/name pair
    let (cs2, nm2, nk2) = {
        let Modal::Ci(ref d) = app.modal else { unreachable!() };
        (d.callsign.clone(), d.name.clone(), d.nickname.clone())
    };
    app.data.remember_op(&cs2, &nm2, &nk2);
    save_data(&app.data);
    app.modal = Modal::None;
}

// ── Session date/time edit dialog ────────────────────────────────────────────
pub fn on_session_dlg(app: &mut App, key: KeyCode) {
    if !matches!(app.modal, Modal::Session(_)) { return; }
    match key {
        KeyCode::Esc => { app.modal = Modal::None; }

        KeyCode::Enter | KeyCode::Down => {
            let Modal::Session(ref mut d) = app.modal else { return };
            if d.focus == 0 { d.focus = 1; }
            else { commit_session_dlg(app); }
        }

        KeyCode::Up => {
            let Modal::Session(ref mut d) = app.modal else { return };
            if d.focus > 0 { d.focus -= 1; }
        }

        KeyCode::Backspace => {
            let Modal::Session(ref mut d) = app.modal else { return };
            d.cur_mut().pop();
        }

        KeyCode::Delete => {
            let Modal::Session(ref mut d) = app.modal else { return };
            d.cur_mut().clear();
        }

        KeyCode::Char(c) => {
            let Modal::Session(ref mut d) = app.modal else { return };
            let max = d.max_len();
            if d.cur_mut().len() < max { d.cur_mut().push(c); }
        }

        _ => {}
    }
}

pub fn commit_session_dlg(app: &mut App) {
    let Modal::Session(ref d) = app.modal else { return };
    let date     = d.fields[0].trim().to_string();
    let net_time = d.fields[1].trim().to_string();
    if date.is_empty() {
        if let Modal::Session(ref mut d) = app.modal { d.focus = 0; }
        return;
    }
    let (ni, si) = (d.ni, d.si);
    if let Some(ses) = app.data.nets.get_mut(ni).and_then(|n| n.sessions.get_mut(si)) {
        ses.date     = date;
        ses.net_time = net_time;
    }
    save_data(&app.data);
    app.modal = Modal::None;
}

// ── Mode picker ───────────────────────────────────────────────────────────────
pub fn on_picker(app: &mut App, key: KeyCode) {
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
pub fn on_confirm(app: &mut App, key: KeyCode) {
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
pub fn on_theme_picker(app: &mut App, key: KeyCode) {
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
pub fn do_export(app: &mut App) {
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

pub fn on_export_dlg(app: &mut App, key: KeyCode) {
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

pub fn commit_export(app: &mut App) {
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
    ls.push(format!("{:>3}  {:<7} {:<12} {:<26} REMARKS",
        "#", "TIME", "CALLSIGN", "NAME/NICK"));
    ls.push("-".repeat(74));
    for (i, ci) in ses.checkins.iter().enumerate() {
        let nm_col = match (ci.name.is_empty(), ci.nickname.is_empty()) {
            (true,  true)  => "--".to_string(),
            (false, true)  => ci.name.clone(),
            (true,  false) => format!("({})", ci.nickname),
            (false, false) => format!("{} ({})", ci.name, ci.nickname),
        };
        ls.push(format!("{:>3}  {:<7} {:<12} {:<26} {}",
            i + 1, ci.time, ci.callsign, nm_col, ci.remarks));
    }
    ls.push("-".repeat(74));
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
