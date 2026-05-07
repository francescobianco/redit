use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame, Terminal,
};
use std::io;

use crate::editor::Editor;
use crate::theme::{self, Theme, Version};

// ── Menu definitions ──────────────────────────────────────────────────────────
const MENUS: &[&str] = &["File", "Edit", "Search", "Options", "Help"];

const MENU_ITEMS: &[&[(&str, &str)]] = &[
    &[
        ("New", ""),
        ("Open...", ""),
        ("Save", "Ctrl+S"),
        ("Save As...", ""),
        ("", ""),
        ("Exit", ""),
    ],
    &[
        ("Cut", "Ctrl+X"),
        ("Copy", "Ctrl+C"),
        ("Paste", "Ctrl+V"),
        ("Clear", "Del"),
    ],
    &[
        ("Find...", "Ctrl+F"),
        ("Repeat Last Find", "F3"),
        ("Change...", "Ctrl+H"),
        ("", ""),
        ("Go To Line...", "Ctrl+G"),
    ],
    &[("Scrollbars", "")],
    &[
        ("Getting Started", ""),
        ("Keyboard", ""),
        ("", ""),
        ("About...", ""),
    ],
];

// ── App mode ──────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Normal,
    Menu { menu: usize, item: usize },
    Open(String),
    SaveAs(String),
    Find(String),
    Goto(String),
    Replace { find: String, replace: String, focus: usize },
    ConfirmNew,
    ConfirmExit,
    About,
}

pub struct App {
    editor: Editor,
    mode: Mode,
    last_find: String,
    message: Option<String>,
    page_height: usize,
    pub theme: Theme,
}

impl App {
    pub fn new() -> Self {
        let mut editor = Editor::new();
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let _ = editor.load_file(&args[1]);
        }
        Self {
            editor,
            mode: Mode::Normal,
            last_find: String::new(),
            message: None,
            page_height: 20,
            theme: theme::v1(),
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if !event::poll(std::time::Duration::from_millis(50))? {
                continue;
            }

            match event::read()? {
                Event::Key(k) => {
                    if self.handle_key(k) {
                        break;
                    }
                }
                Event::Mouse(m) => self.handle_mouse(m),
                Event::Resize(_, h) => {
                    self.page_height = h.saturating_sub(4) as usize;
                }
                _ => {}
            }
        }
        Ok(())
    }

    // ── Render ────────────────────────────────────────────────────────────────

    fn render(&mut self, f: &mut Frame) {
        let size = f.area();
        let menu_area = Rect::new(0, 0, size.width, 1);
        let edit_area = Rect::new(0, 1, size.width, size.height.saturating_sub(2));
        let stat_area = Rect::new(0, size.height.saturating_sub(1), size.width, 1);

        self.render_editor(f, edit_area);
        self.render_menu_bar(f, menu_area);
        self.render_status_bar(f, stat_area);

        if let Mode::Menu { menu, item } = &self.mode {
            let (m, i) = (*menu, *item);
            self.render_dropdown(f, m, i);
        }

        let mode = self.mode.clone();
        self.render_dialog(f, &mode);
    }

    fn render_menu_bar(&self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let base = Style::default().bg(t.menu_bg).fg(t.menu_fg);
        let sel  = Style::default().bg(t.drop_sel_bg).fg(t.drop_sel_fg);

        let fill = " ".repeat(area.width as usize);
        f.render_widget(Paragraph::new(Span::styled(fill, base)), area);

        let mut spans: Vec<Span> = vec![Span::styled(" ", base)];
        for (i, name) in MENUS.iter().enumerate() {
            let is_sel = matches!(&self.mode, Mode::Menu { menu, .. } if *menu == i);
            spans.push(Span::styled(format!(" {} ", name), if is_sel { sel } else { base }));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    fn menu_x(menu_idx: usize) -> u16 {
        let mut x = 1u16;
        for i in 0..menu_idx {
            x += MENUS[i].len() as u16 + 2;
        }
        x
    }

    fn render_dropdown(&self, f: &mut Frame, menu_idx: usize, selected: usize) {
        let t = &self.theme;
        let items = MENU_ITEMS[menu_idx];
        let max_name = items.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
        let max_sc   = items.iter().map(|(_, s)| s.len()).max().unwrap_or(0);
        let inner_w = if max_sc > 0 { max_name + max_sc + 4 } else { max_name + 2 };
        let width  = (inner_w + 2) as u16;
        let height = (items.len() + 2) as u16;
        let x = Self::menu_x(menu_idx);
        let area = Rect::new(x, 1, width, height);

        f.render_widget(Clear, area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.drop_fg).bg(t.drop_bg))
            .style(Style::default().bg(t.drop_bg).fg(t.drop_fg));
        let inner = block.inner(area);
        f.render_widget(block, area);

        for (i, (name, shortcut)) in items.iter().enumerate() {
            let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            if name.is_empty() {
                let sep = "─".repeat(inner.width as usize);
                f.render_widget(
                    Paragraph::new(Span::styled(
                        sep,
                        Style::default().fg(ratatui::style::Color::DarkGray).bg(t.drop_bg),
                    )),
                    row,
                );
                continue;
            }
            let style = if i == selected {
                Style::default().fg(t.drop_sel_fg).bg(t.drop_sel_bg)
            } else {
                Style::default().fg(t.drop_fg).bg(t.drop_bg)
            };
            let w = inner.width as usize;
            let text = if shortcut.is_empty() {
                format!(" {:<pad$} ", name, pad = w.saturating_sub(2))
            } else {
                let gap = w.saturating_sub(name.len() + shortcut.len() + 3);
                format!(" {}{}{} ", name, " ".repeat(gap), shortcut)
            };
            f.render_widget(Paragraph::new(Span::styled(text, style)), row);
        }
    }

    // ── Editor area ───────────────────────────────────────────────────────────

    fn render_editor(&mut self, f: &mut Frame, area: Rect) {
        if area.height < 2 || area.width < 3 {
            return;
        }
        match self.theme.version {
            Version::V1 => self.render_editor_v1(f, area),
            Version::V2 => self.render_editor_v2(f, area),
        }
    }

    /// V1 — faithful MS-DOS EDIT look:
    ///   top border with centred filename, scrollbar on right, no bottom border.
    fn render_editor_v1(&mut self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let frame_style  = Style::default().bg(t.frame_bg).fg(t.frame_fg);
        let title_style  = Style::default().bg(t.title_bg).fg(t.title_fg);
        let scroll_style = Style::default().bg(t.scroll_bg).fg(t.scroll_fg);
        let base         = Style::default().bg(t.edit_bg).fg(t.edit_fg);
        let cur_style    = Style::default().bg(t.edit_fg).fg(t.edit_bg);

        let w = area.width as usize;

        // ── Top border with centred filename ──────────────────────────────────
        let fname = self.editor.filename
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("UNTITLED1")
            .to_uppercase();
        let title_inner = format!(" {} ", fname);
        let tl = title_inner.len();
        let dashes = w.saturating_sub(2 + tl);
        let left_d  = dashes / 2;
        let right_d = dashes - left_d;

        let mut top_spans: Vec<Span> = Vec::new();
        top_spans.push(Span::styled("┌", frame_style));
        top_spans.push(Span::styled("─".repeat(left_d), frame_style));
        top_spans.push(Span::styled(title_inner, title_style));
        top_spans.push(Span::styled("─".repeat(right_d), frame_style));
        top_spans.push(Span::styled("┐", frame_style));
        f.render_widget(
            Paragraph::new(Line::from(top_spans)),
            Rect::new(area.x, area.y, area.width, 1),
        );

        // ── Content rows (no bottom border) ───────────────────────────────────
        // Layout per row: │ (1) | text (w-2) | scrollbar (1)
        let content_h = (area.height - 1) as usize;  // rows below title border
        let text_w    = w.saturating_sub(2);           // between │ and scrollbar

        let (cx, cy) = self.editor.cursor;
        let total_lines = self.editor.lines.len();

        // Adjust scroll
        if cy < self.editor.scroll.1 as usize {
            self.editor.scroll.1 = cy as u16;
        } else if cy >= self.editor.scroll.1 as usize + content_h {
            self.editor.scroll.1 = (cy + 1 - content_h) as u16;
        }
        if cx < self.editor.scroll.0 as usize {
            self.editor.scroll.0 = cx as u16;
        } else if cx >= self.editor.scroll.0 as usize + text_w {
            self.editor.scroll.0 = (cx + 1 - text_w) as u16;
        }
        let sy = self.editor.scroll.1 as usize;
        let sx = self.editor.scroll.0 as usize;

        for row in 0..content_h {
            let screen_y = area.y + 1 + row as u16;
            let line_idx = sy + row;

            // Left border │
            f.render_widget(
                Paragraph::new(Span::styled("│", frame_style)),
                Rect::new(area.x, screen_y, 1, 1),
            );

            // Scrollbar character on right
            let sc = Self::scrollbar_char(row, content_h, total_lines, sy);
            f.render_widget(
                Paragraph::new(Span::styled(sc, scroll_style)),
                Rect::new(area.x + area.width - 1, screen_y, 1, 1),
            );

            // Text content
            let text_area = Rect::new(area.x + 1, screen_y, text_w as u16, 1);

            if line_idx >= total_lines {
                f.render_widget(
                    Paragraph::new(Span::styled(" ".repeat(text_w), base)),
                    text_area,
                );
                continue;
            }

            let chars: Vec<char> = self.editor.lines[line_idx].chars().collect();
            let on_this_line = line_idx == cy;

            Self::render_text_row(f, text_area, &chars, sx, cx, on_this_line, base, cur_style);
        }
    }

    /// V2 — simplified look: full box border (top + bottom), no title, no scrollbar.
    fn render_editor_v2(&mut self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let border_style = Style::default().bg(t.frame_bg).fg(t.frame_fg);
        let base         = Style::default().bg(t.edit_bg).fg(t.edit_fg);
        let cur_style    = Style::default().bg(t.edit_fg).fg(t.edit_bg);
        let w = area.width as usize;

        // Top border
        let top = format!("┌{}┐", "─".repeat(w.saturating_sub(2)));
        f.render_widget(
            Paragraph::new(Span::styled(top, border_style)),
            Rect::new(area.x, area.y, area.width, 1),
        );
        // Bottom border
        let bot = format!("└{}┘", "─".repeat(w.saturating_sub(2)));
        f.render_widget(
            Paragraph::new(Span::styled(bot, border_style)),
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        );

        let inner = Rect::new(area.x + 1, area.y + 1, (w - 2) as u16, area.height - 2);
        let vh = inner.height as usize;
        let vw = inner.width as usize;
        let (cx, cy) = self.editor.cursor;

        if cy < self.editor.scroll.1 as usize {
            self.editor.scroll.1 = cy as u16;
        } else if cy >= self.editor.scroll.1 as usize + vh {
            self.editor.scroll.1 = (cy + 1 - vh) as u16;
        }
        if cx < self.editor.scroll.0 as usize {
            self.editor.scroll.0 = cx as u16;
        } else if cx >= self.editor.scroll.0 as usize + vw {
            self.editor.scroll.0 = (cx + 1 - vw) as u16;
        }
        let sy = self.editor.scroll.1 as usize;
        let sx = self.editor.scroll.0 as usize;

        for row in 0..vh {
            let screen_y = inner.y + row as u16;
            let line_idx = sy + row;

            // Left │
            f.render_widget(
                Paragraph::new(Span::styled("│", border_style)),
                Rect::new(area.x, screen_y, 1, 1),
            );
            // Right │
            f.render_widget(
                Paragraph::new(Span::styled("│", border_style)),
                Rect::new(area.x + area.width - 1, screen_y, 1, 1),
            );

            let text_area = Rect::new(inner.x, screen_y, vw as u16, 1);

            if line_idx >= self.editor.lines.len() {
                f.render_widget(Paragraph::new(Span::styled(" ".repeat(vw), base)), text_area);
                continue;
            }

            let chars: Vec<char> = self.editor.lines[line_idx].chars().collect();
            let on_this_line = line_idx == cy;
            Self::render_text_row(f, text_area, &chars, sx, cx, on_this_line, base, cur_style);
        }
    }

    /// Render one row of text with an inverted cursor cell.
    fn render_text_row(
        f: &mut Frame,
        area: Rect,
        chars: &[char],
        sx: usize,
        cx: usize,
        cursor_here: bool,
        base: Style,
        cur_style: Style,
    ) {
        let vw = area.width as usize;

        if !cursor_here {
            let s: String = chars.iter().skip(sx).take(vw).collect();
            let pad = vw.saturating_sub(s.chars().count());
            f.render_widget(
                Paragraph::new(Span::styled(format!("{}{}", s, " ".repeat(pad)), base)),
                area,
            );
            return;
        }

        let mut spans: Vec<Span> = Vec::new();

        // before cursor
        if cx > sx {
            let s: String = chars.iter().skip(sx).take(cx - sx).collect();
            spans.push(Span::styled(s, base));
        }

        // cursor cell
        spans.push(Span::styled(
            chars.get(cx).copied().unwrap_or(' ').to_string(),
            cur_style,
        ));

        // after cursor
        let after_start = cx + 1;
        let after_end   = (sx + vw).min(chars.len());
        if after_start < after_end {
            let s: String = chars[after_start..after_end].iter().collect();
            spans.push(Span::styled(s, base));
        }

        // trailing spaces
        let used = (cx.saturating_sub(sx) + 1).min(vw)
            + after_end.saturating_sub(after_start);
        let rem = vw.saturating_sub(used);
        if rem > 0 {
            spans.push(Span::styled(" ".repeat(rem), base));
        }

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    /// Scrollbar character for V1.
    /// `row` is 0-indexed within the content block (0 = top, content_h-1 = bottom).
    fn scrollbar_char(row: usize, content_h: usize, total_doc: usize, scroll_y: usize) -> &'static str {
        if row == 0 {
            return "↑";
        }
        if row == content_h - 1 {
            return "↓";
        }
        if content_h <= 2 {
            return "█";
        }
        let body = content_h - 2; // rows between ↑ and ↓
        if total_doc <= content_h {
            return "█"; // whole document visible
        }
        let thumb_size = (body * content_h / total_doc).max(1).min(body);
        let max_offset = body - thumb_size;
        let max_scroll = total_doc - content_h;
        let thumb_pos  = max_offset * scroll_y / max_scroll;
        let body_row   = row - 1; // 0-indexed within the body
        if body_row >= thumb_pos && body_row < thumb_pos + thumb_size {
            "█"
        } else {
            "░"
        }
    }

    // ── Status bar ────────────────────────────────────────────────────────────

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let t   = &self.theme;
        let (cx, cy) = self.editor.cursor;
        let ovr   = if self.editor.overtype { "OVR" } else { "   " };
        let dirty = if self.editor.dirty { "*" } else { " " };
        let fname = self.editor.filename.as_deref().unwrap_or("Untitled");
        let w = area.width as usize;

        let right = format!("{}  Ln:{:>4}  Col:{:>3}  {}", dirty, cy + 1, cx + 1, ovr);

        let left = if let Some(m) = &self.message {
            format!(" {}", m)
        } else if self.theme.version == Version::V1 {
            format!(" F1=Help")
        } else {
            format!(" F1=Help  {}", fname)
        };

        let right_len = right.len();
        let left = if left.len() + right_len + 3 > w {
            left[..w.saturating_sub(right_len + 3)].to_string()
        } else {
            left
        };

        // V1 adds a │ separator between left info and right info
        let line = if self.theme.version == Version::V1 {
            let mid_pad = w.saturating_sub(left.len() + right_len + 1);
            format!("{}{}│{}", left, " ".repeat(mid_pad), right)
        } else {
            let pad = w.saturating_sub(left.len() + right_len);
            format!("{}{}{}", left, " ".repeat(pad), right)
        };

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{:<w$}", line, w = w),
                Style::default().bg(t.stat_bg).fg(t.stat_fg),
            )),
            area,
        );
    }

    // ── Dialog rendering ──────────────────────────────────────────────────────

    fn render_dialog(&self, f: &mut Frame, mode: &Mode) {
        match mode {
            Mode::Open(s)    => self.input_dialog(f, "Open",        "File Name:",    s),
            Mode::SaveAs(s)  => self.input_dialog(f, "Save As",     "File Name:",    s),
            Mode::Find(s)    => self.input_dialog(f, "Find",        "Find What:",    s),
            Mode::Goto(s)    => self.input_dialog(f, "Go To Line",  "Line Number:",  s),
            Mode::ConfirmNew  => self.confirm_dialog(f, "New File", "Discard unsaved changes?"),
            Mode::ConfirmExit => self.confirm_dialog(f, "Exit",     "Discard unsaved changes and exit?"),
            Mode::About       => self.about_dialog(f),
            Mode::Replace { find, replace, focus } => self.replace_dialog(f, find, replace, *focus),
            _ => {}
        }
    }

    fn center_rect(f: &Frame, w: u16, h: u16) -> Rect {
        let s = f.area();
        Rect::new(
            s.width.saturating_sub(w) / 2,
            s.height.saturating_sub(h) / 2,
            w.min(s.width),
            h.min(s.height),
        )
    }

    fn input_dialog(&self, f: &mut Frame, title: &str, label: &str, input: &str) {
        let t = &self.theme;
        let area = Self::center_rect(f, 52, 7);
        f.render_widget(Clear, area);
        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.dlg_fg).bg(t.dlg_bg))
            .style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg));
        let inner = block.inner(area);
        f.render_widget(block, area);

        f.render_widget(
            Paragraph::new(label).style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg)),
            Rect::new(inner.x + 1, inner.y + 1, inner.width - 2, 1),
        );
        let iw = inner.width.saturating_sub(2);
        f.render_widget(
            Paragraph::new(format!("{:<w$}", input, w = iw as usize))
                .style(Style::default().bg(t.dlg_inp_bg).fg(t.dlg_inp_fg)),
            Rect::new(inner.x + 1, inner.y + 2, iw, 1),
        );
        self.btn(f, inner.x + 1, inner.y + 4, "[ OK ]");
        self.btn(f, inner.x + 9, inner.y + 4, "[ Cancel ]");
    }

    fn confirm_dialog(&self, f: &mut Frame, title: &str, msg: &str) {
        let t = &self.theme;
        let area = Self::center_rect(f, 52, 7);
        f.render_widget(Clear, area);
        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.dlg_fg).bg(t.dlg_bg))
            .style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg));
        let inner = block.inner(area);
        f.render_widget(block, area);

        f.render_widget(
            Paragraph::new(msg).style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg)),
            Rect::new(inner.x + 1, inner.y + 1, inner.width - 2, 1),
        );
        self.btn(f, inner.x + 1, inner.y + 3, "[ Yes ]");
        self.btn(f, inner.x + 10, inner.y + 3, "[ No ]");
        self.btn(f, inner.x + 18, inner.y + 3, "[ Cancel ]");
    }

    fn about_dialog(&self, f: &mut Frame) {
        let t = &self.theme;
        let area = Self::center_rect(f, 42, 9);
        f.render_widget(Clear, area);
        let block = Block::default()
            .title(" About redit ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.dlg_fg).bg(t.dlg_bg))
            .style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let lines = vec![
            Line::from(Span::styled(
                "  redit  v0.1.0",
                Style::default().fg(t.dlg_fg).bg(t.dlg_bg).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  MS-DOS EDIT clone written in Rust",
                Style::default().fg(t.dlg_fg).bg(t.dlg_bg),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Copyright (c) 2026 Francesco Bianco",
                Style::default().fg(t.dlg_fg).bg(t.dlg_bg),
            )),
        ];
        f.render_widget(
            Paragraph::new(lines).style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg)),
            Rect::new(inner.x, inner.y + 1, inner.width, 5),
        );
        self.btn(f, inner.x + (inner.width - 8) / 2, inner.y + 6, "[  OK  ]");
    }

    fn replace_dialog(&self, f: &mut Frame, find: &str, replace: &str, focus: usize) {
        let t = &self.theme;
        let area = Self::center_rect(f, 55, 10);
        f.render_widget(Clear, area);
        let block = Block::default()
            .title(" Change ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(t.dlg_fg).bg(t.dlg_bg))
            .style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let iw = inner.width.saturating_sub(2);

        f.render_widget(
            Paragraph::new("Find What:").style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg)),
            Rect::new(inner.x + 1, inner.y + 1, inner.width - 2, 1),
        );
        let (fbg, ffg) = if focus == 0 { (t.dlg_inp_bg, t.dlg_inp_fg) } else { (t.dlg_bg, t.dlg_fg) };
        f.render_widget(
            Paragraph::new(format!("{:<w$}", find, w = iw as usize))
                .style(Style::default().bg(fbg).fg(ffg)),
            Rect::new(inner.x + 1, inner.y + 2, iw, 1),
        );

        f.render_widget(
            Paragraph::new("Replace With:").style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg)),
            Rect::new(inner.x + 1, inner.y + 4, inner.width - 2, 1),
        );
        let (rbg, rfg) = if focus == 1 { (t.dlg_inp_bg, t.dlg_inp_fg) } else { (t.dlg_bg, t.dlg_fg) };
        f.render_widget(
            Paragraph::new(format!("{:<w$}", replace, w = iw as usize))
                .style(Style::default().bg(rbg).fg(rfg)),
            Rect::new(inner.x + 1, inner.y + 5, iw, 1),
        );

        self.btn(f, inner.x + 1, inner.y + 7, "[ OK ]");
        self.btn(f, inner.x + 9, inner.y + 7, "[ Cancel ]");
    }

    fn btn(&self, f: &mut Frame, x: u16, y: u16, label: &str) {
        let t = &self.theme;
        f.render_widget(
            Paragraph::new(Span::styled(
                label,
                Style::default().bg(t.dlg_btn_bg).fg(t.dlg_btn_fg),
            )),
            Rect::new(x, y, label.len() as u16, 1),
        );
    }

    // ── Key handling ──────────────────────────────────────────────────────────

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match self.mode.clone() {
            Mode::Normal => self.key_normal(key),
            Mode::Menu { menu, item } => self.key_menu(key, menu, item),
            Mode::Open(_)   => self.key_open(key),
            Mode::SaveAs(_) => self.key_save_as(key),
            Mode::Find(_)   => self.key_find(key),
            Mode::Goto(_)   => self.key_goto(key),
            Mode::Replace { find, replace, focus } => self.key_replace(key, find, replace, focus),
            Mode::ConfirmNew  => self.key_confirm_new(key),
            Mode::ConfirmExit => self.key_confirm_exit(key),
            Mode::About => { self.mode = Mode::Normal; false }
        }
    }

    fn key_normal(&mut self, key: KeyEvent) -> bool {
        let m = key.modifiers;
        match key.code {
            // Menu bar
            KeyCode::F(10) => { self.mode = Mode::Menu { menu: 0, item: 0 }; }
            KeyCode::Char('f') | KeyCode::Char('F') if m == KeyModifiers::ALT =>
                { self.mode = Mode::Menu { menu: 0, item: 0 }; }
            KeyCode::Char('e') | KeyCode::Char('E') if m == KeyModifiers::ALT =>
                { self.mode = Mode::Menu { menu: 1, item: 0 }; }
            KeyCode::Char('s') | KeyCode::Char('S') if m == KeyModifiers::ALT =>
                { self.mode = Mode::Menu { menu: 2, item: 0 }; }
            KeyCode::Char('o') | KeyCode::Char('O') if m == KeyModifiers::ALT =>
                { self.mode = Mode::Menu { menu: 3, item: 0 }; }
            KeyCode::Char('h') | KeyCode::Char('H') if m == KeyModifiers::ALT =>
                { self.mode = Mode::Menu { menu: 4, item: 0 }; }

            // File
            KeyCode::Char('s') if m == KeyModifiers::CONTROL => self.do_save(),
            KeyCode::F(2) => self.do_save(),

            // Search
            KeyCode::Char('f') if m == KeyModifiers::CONTROL =>
                { self.mode = Mode::Find(self.last_find.clone()); }
            KeyCode::F(3) => {
                let q = self.last_find.clone();
                if !q.is_empty() && !self.editor.find_next(&q) {
                    self.message = Some(format!("'{}' not found", q));
                }
            }
            KeyCode::Char('h') if m == KeyModifiers::CONTROL => {
                self.mode = Mode::Replace {
                    find: self.last_find.clone(),
                    replace: String::new(),
                    focus: 0,
                };
            }
            KeyCode::Char('g') if m == KeyModifiers::CONTROL =>
                { self.mode = Mode::Goto(String::new()); }

            // Edit
            KeyCode::Char('x') if m == KeyModifiers::CONTROL => { self.editor.cut_line(); }
            KeyCode::Char('c') if m == KeyModifiers::CONTROL => { self.editor.copy_line(); }
            KeyCode::Char('v') if m == KeyModifiers::CONTROL => { self.editor.paste(); }

            // Navigation
            KeyCode::Left  => self.editor.cursor_left(),
            KeyCode::Right => self.editor.cursor_right(),
            KeyCode::Up    => self.editor.cursor_up(),
            KeyCode::Down  => self.editor.cursor_down(),
            KeyCode::Home if m == KeyModifiers::CONTROL =>
                { self.editor.cursor = (0, 0); }
            KeyCode::End if m == KeyModifiers::CONTROL => {
                let last = self.editor.lines.len().saturating_sub(1);
                let col  = self.editor.lines[last].chars().count();
                self.editor.cursor = (col, last);
            }
            KeyCode::Home     => self.editor.home(),
            KeyCode::End      => self.editor.end(),
            KeyCode::PageUp   => self.editor.page_up(self.page_height),
            KeyCode::PageDown => self.editor.page_down(self.page_height),

            // Editing
            KeyCode::Insert => { self.editor.overtype = !self.editor.overtype; }
            KeyCode::Delete    => self.editor.delete(),
            KeyCode::Backspace => self.editor.backspace(),
            KeyCode::Enter     => self.editor.insert_newline(),
            KeyCode::Tab => { for _ in 0..4 { self.editor.insert_char(' '); } }
            KeyCode::Char(c) => { self.message = None; self.editor.insert_char(c); }

            _ => {}
        }
        false
    }

    fn key_menu(&mut self, key: KeyEvent, mi: usize, ii: usize) -> bool {
        match key.code {
            KeyCode::Esc   => { self.mode = Mode::Normal; }
            KeyCode::Left  => {
                let nm = if mi == 0 { MENUS.len() - 1 } else { mi - 1 };
                self.mode = Mode::Menu { menu: nm, item: 0 };
            }
            KeyCode::Right => {
                self.mode = Mode::Menu { menu: (mi + 1) % MENUS.len(), item: 0 };
            }
            KeyCode::Up => {
                let items = MENU_ITEMS[mi];
                let mut ni = if ii == 0 { items.len() - 1 } else { ii - 1 };
                while items[ni].0.is_empty() {
                    ni = if ni == 0 { items.len() - 1 } else { ni - 1 };
                }
                self.mode = Mode::Menu { menu: mi, item: ni };
            }
            KeyCode::Down => {
                let items = MENU_ITEMS[mi];
                let mut ni = (ii + 1) % items.len();
                while items[ni].0.is_empty() { ni = (ni + 1) % items.len(); }
                self.mode = Mode::Menu { menu: mi, item: ni };
            }
            KeyCode::Enter => return self.activate(mi, ii),
            _ => {}
        }
        false
    }

    fn activate(&mut self, mi: usize, ii: usize) -> bool {
        self.mode = Mode::Normal;
        match (mi, ii) {
            (0, 0) => self.do_new(),
            (0, 1) => { self.mode = Mode::Open(String::new()); }
            (0, 2) => self.do_save(),
            (0, 3) => {
                let n = self.editor.filename.clone().unwrap_or_default();
                self.mode = Mode::SaveAs(n);
            }
            (0, 5) => {
                if self.editor.dirty { self.mode = Mode::ConfirmExit; return false; }
                return true;
            }
            (1, 0) => self.editor.cut_line(),
            (1, 1) => self.editor.copy_line(),
            (1, 2) => self.editor.paste(),
            (1, 3) => self.editor.delete(),
            (2, 0) => { self.mode = Mode::Find(self.last_find.clone()); }
            (2, 1) => {
                let q = self.last_find.clone();
                if !q.is_empty() && !self.editor.find_next(&q) {
                    self.message = Some(format!("'{}' not found", q));
                }
            }
            (2, 2) => {
                self.mode = Mode::Replace {
                    find: self.last_find.clone(),
                    replace: String::new(),
                    focus: 0,
                };
            }
            (2, 4) => { self.mode = Mode::Goto(String::new()); }
            (4, 3) => { self.mode = Mode::About; }
            _ => {}
        }
        false
    }

    fn key_open(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => { self.mode = Mode::Normal; }
            KeyCode::Enter => {
                if let Mode::Open(ref s) = self.mode.clone() {
                    let path = s.clone();
                    self.mode = Mode::Normal;
                    if !path.is_empty() {
                        if let Err(e) = self.editor.load_file(&path) {
                            self.message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
            KeyCode::Backspace => { if let Mode::Open(ref mut s) = self.mode { s.pop(); } }
            KeyCode::Char(c)   => { if let Mode::Open(ref mut s) = self.mode { s.push(c); } }
            _ => {}
        }
        false
    }

    fn key_save_as(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => { self.mode = Mode::Normal; }
            KeyCode::Enter => {
                if let Mode::SaveAs(ref s) = self.mode.clone() {
                    let path = s.clone();
                    self.mode = Mode::Normal;
                    if !path.is_empty() {
                        self.editor.filename = Some(path.clone());
                        match self.editor.save_file(&path) {
                            Ok(_) => {
                                self.editor.dirty = false;
                                self.message = Some(format!("Saved: {}", path));
                            }
                            Err(e) => { self.message = Some(format!("Error: {}", e)); }
                        }
                    }
                }
            }
            KeyCode::Backspace => { if let Mode::SaveAs(ref mut s) = self.mode { s.pop(); } }
            KeyCode::Char(c)   => { if let Mode::SaveAs(ref mut s) = self.mode { s.push(c); } }
            _ => {}
        }
        false
    }

    fn key_find(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => { self.mode = Mode::Normal; }
            KeyCode::Enter => {
                if let Mode::Find(ref s) = self.mode.clone() {
                    let q = s.clone();
                    self.last_find = q.clone();
                    self.mode = Mode::Normal;
                    if !q.is_empty() && !self.editor.find_next(&q) {
                        self.message = Some(format!("'{}' not found", q));
                    }
                }
            }
            KeyCode::Backspace => { if let Mode::Find(ref mut s) = self.mode { s.pop(); } }
            KeyCode::Char(c)   => { if let Mode::Find(ref mut s) = self.mode { s.push(c); } }
            _ => {}
        }
        false
    }

    fn key_goto(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => { self.mode = Mode::Normal; }
            KeyCode::Enter => {
                if let Mode::Goto(ref s) = self.mode.clone() {
                    let line = s.parse::<usize>().unwrap_or(0);
                    self.editor.goto_line(line);
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Backspace => { if let Mode::Goto(ref mut s) = self.mode { s.pop(); } }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if let Mode::Goto(ref mut s) = self.mode { s.push(c); }
            }
            _ => {}
        }
        false
    }

    fn key_replace(&mut self, key: KeyEvent, find: String, replace: String, focus: usize) -> bool {
        match key.code {
            KeyCode::Esc => { self.mode = Mode::Normal; }
            KeyCode::Tab => {
                self.mode = Mode::Replace { find, replace, focus: 1 - focus };
            }
            KeyCode::Enter => {
                self.last_find = find.clone();
                let mut count = 0usize;
                for line in &mut self.editor.lines {
                    while let Some(pos) = line.find(&find) {
                        line.replace_range(pos..pos + find.len(), &replace);
                        count += 1;
                        if find.is_empty() { break; }
                    }
                }
                self.editor.dirty = count > 0;
                self.editor.highlight();
                self.mode = Mode::Normal;
                self.message = Some(format!("{} replacement(s) made", count));
            }
            KeyCode::Backspace => {
                if focus == 0 {
                    let mut f2 = find; f2.pop();
                    self.mode = Mode::Replace { find: f2, replace, focus };
                } else {
                    let mut r2 = replace; r2.pop();
                    self.mode = Mode::Replace { find, replace: r2, focus };
                }
            }
            KeyCode::Char(c) => {
                if focus == 0 {
                    let mut f2 = find; f2.push(c);
                    self.mode = Mode::Replace { find: f2, replace, focus };
                } else {
                    let mut r2 = replace; r2.push(c);
                    self.mode = Mode::Replace { find, replace: r2, focus };
                }
            }
            _ => {}
        }
        false
    }

    fn key_confirm_new(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => { self.do_save(); self.new_confirmed(); }
            KeyCode::Char('n') | KeyCode::Char('N') => self.new_confirmed(),
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') =>
                { self.mode = Mode::Normal; }
            _ => {}
        }
        false
    }

    fn key_confirm_exit(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => return true,
            _ => { self.mode = Mode::Normal; }
        }
        false
    }

    // ── Mouse ─────────────────────────────────────────────────────────────────

    fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return;
        }
        let (mx, my) = (mouse.column, mouse.row);

        if my == 0 {
            let mut x = 1u16;
            for (i, name) in MENUS.iter().enumerate() {
                let w = name.len() as u16 + 2;
                if mx >= x && mx < x + w {
                    self.mode = Mode::Menu { menu: i, item: 0 };
                    return;
                }
                x += w;
            }
            return;
        }

        if let Mode::Normal = self.mode {
            let ey = my as usize - 2 + self.editor.scroll.1 as usize; // -2: menu+border
            let ex = mx as usize - 1 + self.editor.scroll.0 as usize; // -1: left │
            if ey < self.editor.lines.len() {
                let ll = self.editor.lines[ey].chars().count();
                self.editor.cursor = (ex.min(ll), ey);
            }
        }
    }

    // ── Actions ───────────────────────────────────────────────────────────────

    fn do_save(&mut self) {
        match self.editor.filename.clone() {
            Some(path) => match self.editor.save_file(&path) {
                Ok(_) => {
                    self.editor.dirty = false;
                    self.message = Some(format!("Saved: {}", path));
                }
                Err(e) => { self.message = Some(format!("Error saving: {}", e)); }
            },
            None => { self.mode = Mode::SaveAs(String::new()); }
        }
    }

    fn do_new(&mut self) {
        if self.editor.dirty { self.mode = Mode::ConfirmNew; } else { self.new_confirmed(); }
    }

    fn new_confirmed(&mut self) {
        self.editor = Editor::new();
        self.mode   = Mode::Normal;
        self.message = None;
    }
}