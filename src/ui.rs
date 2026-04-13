use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::{App, Focus};
use crate::dialogs::{
    CiDlg, ExportDlg, Modal, ModePick, NetDlg, NdMode,
    OperatorDlg, SessionDlg, ThemePickerDlg,
    NF_NAME, NF_CLUB, NF_FREQ, NF_OFFSET, NF_PL, NF_TOGGLE, NF_MODE, NF_NOTES,
    OF_CALL, OF_NAME,
};
use crate::models::{CheckIn, DIGITAL_MODES, LOGO};
use crate::theme::Theme;

// ── UI root ───────────────────────────────────────────────────────────────────
pub fn ui(f: &mut Frame, app: &mut App) {
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
    let _is_dig = app.net().map_or(false,|n|n.digital);
    let has_club= app.net().map_or(false,|n|!n.club.is_empty());
    // info height: 5 content rows (freq, pl, mode/voice, name) + 1 if club + 2 (borders)
    // digital MODE row replaces VOICE NET row — same count either way
    let info_h = 5u16
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
        Span::styled(format!("{:>3} ","#"),          t.bold()),
        Span::styled(format!("{:<7}","TIME"),        t.bold()),
        Span::styled(format!("{:<13}","CALLSIGN"),   t.bold()),
        Span::styled(format!("{:<22}","NAME/NICK"),  t.bold()),
        Span::styled("REMARKS",                      t.bold()),
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
        let rem_w = rw.saturating_sub(3+1+7+13+22);
        let rem = &ci.remarks[..ci.remarks.len().min(rem_w)];
        let nm  = if ci.name.is_empty(){"—"}else{&ci.name};
        let nm_col = if !ci.nickname.is_empty() {
            format!("{} ({})", nm, ci.nickname)
        } else {
            nm.to_string()
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("{:>3} ",i+1),        t.dim()),
            Span::styled(format!("{:<7}",ci.time),     t.time_s()),
            Span::styled(format!("{:<13}",ci.callsign),t.call()),
            Span::styled(format!("{:<22}",nm_col),     t.normal()),
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
        Modal::Session(d)     => draw_session_dlg(f, area, d, t),
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
    // Clear all cell characters first, then fill background
    f.render_widget(Clear, area);
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

    let lines: Vec<Line> = vec![
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
    let dh = 24u16 + if d.digital {6} else {0};
    let r = centered(66, dh, area);
    f.render_widget(Clear, r);
    let blk = Block::default().title(Span::styled(title,t.bold()))
        .borders(Borders::ALL).border_style(t.bold()).style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    // labels and field indices aligned with NF_* constants
    // fields array: [name, club, freq, offset, pl]
    let field_defs: &[(&str, usize)] = &[
        ("Net Name",          NF_NAME),
        ("Club / Association",NF_CLUB),
        ("Frequency (MHz)",   NF_FREQ),
        ("Offset",            NF_OFFSET),
        ("PL Tone (Hz)",      NF_PL),
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
    let dh = 21u16 + comp_h;
    let r  = centered(62, dh, area);
    f.render_widget(Clear, r);

    let title = if d.fcc_pending { " ADD CHECK-IN  [Searching…] " } else { " ADD CHECK-IN " };
    let title_style = if d.fcc_pending { t.amber_s() } else { t.bold() };
    let blk = Block::default()
        .title(Span::styled(title, title_style))
        .borders(Borders::ALL).border_style(t.bold()).style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let labels = ["Callsign","Name","Nickname","Remarks"];
    let vals: [&str;4] = [&d.callsign, &d.name, &d.nickname, &d.remarks];
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
        "[↑↓] nav  [TAB] search  [ENTER] confirm  [ESC] cancel",
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

fn draw_session_dlg(f: &mut Frame, area: Rect, d: &SessionDlg, t: &Theme) {
    let r = centered(48, 12, area);
    f.render_widget(Clear, r);
    let blk = Block::default()
        .title(Span::styled(" EDIT SESSION ", t.bold()))
        .borders(Borders::ALL).border_style(t.bold())
        .style(t.normal());
    let inner = blk.inner(r);
    f.render_widget(blk, r);

    let labels = ["Date (YYYY-MM-DD)", "Time (HH:MM)"];
    let mut lines: Vec<Line> = vec![Line::from("")];
    for (i, lbl) in labels.iter().enumerate() {
        lines.push(Line::from(Span::styled(format!("{}:", lbl), t.cyan_s())));
        let cur = if d.focus == i { "_" } else { "" };
        lines.push(Line::from(Span::styled(
            format!(" {}{}", d.fields[i], cur),
            if d.focus == i { t.sel() } else { t.normal() },
        )));
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(
        "[↑↓/ENTER] navigate  [ENTER] confirm  [ESC] cancel", t.dim())));
    f.render_widget(Paragraph::new(lines).style(t.normal()), inner);
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
        entry("e",               "Edit session date / time"),
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
