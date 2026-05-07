use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers, ModifierKeyCode, MouseButton, MouseEventKind,
};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::io;

use crate::editor::Editor;
use crate::theme::{self, Theme, Version};

mod v1;
mod v2;

// ── Menu definitions ──────────────────────────────────────────────────────────
const MENUS: &[&str] = &["File", "Edit", "Search", "Options", "Help"];
// Index of the accelerator letter within each menu name (shown underlined/bright).
const MENU_ACCELS: &[usize] = &[0, 0, 0, 0, 0]; // F, E, S, O, H

#[derive(Clone, Copy)]
struct MenuItem {
    name: &'static str,
    shortcut: &'static str,
    accel: Option<usize>,
    help: &'static str,
}

const fn item(
    name: &'static str,
    shortcut: &'static str,
    accel: Option<usize>,
    help: &'static str,
) -> MenuItem {
    MenuItem {
        name,
        shortcut,
        accel,
        help,
    }
}

const fn sep() -> MenuItem {
    item("", "", None, "")
}

const MENU_ITEMS: &[&[MenuItem]] = &[
    &[
        item(
            "New",
            "",
            Some(0),
            "Removes currently loaded file from memory",
        ),
        item("Open...", "", Some(0), "Opens a file"),
        item("Save", "", Some(0), "Saves current file"),
        item(
            "Save As...",
            "",
            Some(5),
            "Saves current file under a new name",
        ),
        sep(),
        item("Print...", "", Some(0), "Prints current file"),
        sep(),
        item("Exit", "", Some(1), "Exits the MS-DOS Editor"),
    ],
    &[
        item(
            "Cut",
            "Shift+Del",
            Some(0),
            "Deletes selected text and copies it to buffer",
        ),
        item(
            "Copy",
            "Ctrl+Ins",
            Some(0),
            "Copies selected text to buffer",
        ),
        item("Paste", "Shift+Ins", Some(0), "Inserts text from buffer"),
        item("Clear", "Del", Some(0), "Deletes selected text"),
    ],
    &[
        item("Find...", "", Some(0), "Finds specified text"),
        item(
            "Repeat Last Find",
            "F3",
            Some(0),
            "Finds next occurrence of text",
        ),
        item("Change...", "", Some(0), "Changes specified text"),
    ],
    &[item(
        "Scrollbars",
        "",
        Some(0),
        "Shows or hides scroll bars",
    )],
    &[
        item("Getting Started", "", Some(0), "Displays basic help"),
        item("Keyboard", "", Some(0), "Displays keyboard help"),
        sep(),
        item("About...", "", Some(0), "Displays program information"),
    ],
];

// ── App mode ──────────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Mode {
    Normal,
    Welcome,
    Menu {
        menu: usize,
        item: usize,
    },
    Open(String),
    SaveAs(String),
    Find(String),
    Goto(String),
    Replace {
        find: String,
        replace: String,
        focus: usize,
    },
    ConfirmNew,
    ConfirmExit,
    About,
}

pub struct App {
    pub(super) editor: Editor,
    pub(super) mode: Mode,
    pub(super) last_find: String,
    pub(super) message: Option<String>,
    pub(super) page_height: usize,
    pub theme: Theme,
}

impl App {
    pub fn new() -> Self {
        let mut editor = Editor::new();
        let args: Vec<String> = std::env::args().collect();
        if args.len() > 1 {
            let _ = editor.load_file(&args[1]);
        }
        let initial_mode = if args.len() <= 1 {
            Mode::Welcome
        } else {
            Mode::Normal
        };
        Self {
            editor,
            mode: initial_mode,
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
        let sel = Style::default().bg(t.drop_sel_bg).fg(t.drop_sel_fg);
        let accel = if self.theme.version == Version::V1 {
            Style::default()
                .bg(t.menu_bg)
                .fg(ratatui::style::Color::White)
        } else {
            Style::default()
                .bg(t.menu_bg)
                .fg(t.menu_fg)
                .add_modifier(Modifier::UNDERLINED)
        };
        let accel_sel = if self.theme.version == Version::V1 {
            Style::default()
                .bg(t.drop_sel_bg)
                .fg(ratatui::style::Color::White)
        } else {
            Style::default()
                .bg(t.drop_sel_bg)
                .fg(t.drop_sel_fg)
                .add_modifier(Modifier::UNDERLINED)
        };

        let fill = " ".repeat(area.width as usize);
        f.render_widget(Paragraph::new(Span::styled(fill, base)), area);

        let mut spans: Vec<Span> = vec![Span::styled(
            if self.theme.version == Version::V1 {
                "  "
            } else {
                " "
            },
            base,
        )];
        for (i, name) in MENUS.iter().enumerate() {
            if self.theme.version == Version::V1 && i == MENUS.len() - 1 {
                let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                let help_w = name.len() + 2;
                if area.width as usize > used + help_w {
                    spans.push(Span::styled(
                        " ".repeat(area.width as usize - used - help_w - 1),
                        base,
                    ));
                }
            }
            let is_sel = matches!(&self.mode, Mode::Menu { menu, .. } if *menu == i);
            let (bg, fg, ac) = if is_sel {
                (sel, sel, accel_sel)
            } else {
                (base, base, accel)
            };
            let acc_pos = MENU_ACCELS[i];
            spans.push(Span::styled(" ", bg));
            for (j, ch) in name.char_indices() {
                let st = if j == acc_pos { ac } else { fg };
                spans.push(Span::styled(ch.to_string(), st));
            }
            spans.push(Span::styled(" ", bg));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    pub(super) fn menu_x(menu_idx: usize) -> u16 {
        let mut x = 1u16;
        for i in 0..menu_idx {
            x += MENUS[i].len() as u16 + 2;
        }
        x
    }

    fn render_dropdown(&self, f: &mut Frame, menu_idx: usize, selected: usize) {
        let t = &self.theme;
        let items = MENU_ITEMS[menu_idx];
        let max_name = items.iter().map(|it| it.name.len()).max().unwrap_or(4);
        let max_sc = items.iter().map(|it| it.shortcut.len()).max().unwrap_or(0);
        let inner_w = if max_sc > 0 {
            max_name + max_sc + 4
        } else {
            max_name + 2
        };
        let inner_w = if self.theme.version == Version::V1 {
            inner_w.max(match menu_idx {
                0 => 16,
                1 => 22,
                2 => 26,
                _ => inner_w,
            })
        } else {
            inner_w
        };
        let width = (inner_w + 2) as u16;
        let height = (items.len() + 2) as u16;
        let x = Self::menu_x(menu_idx);
        let area = Rect::new(x, 1, width, height);

        f.render_widget(Clear, area);
        let drop_style = Style::default().fg(t.drop_fg).bg(t.drop_bg);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(drop_style)
            .style(drop_style);
        let inner = block.inner(area);
        f.render_widget(block, area);

        for (i, it) in items.iter().enumerate() {
            let row = Rect::new(inner.x, inner.y + i as u16, inner.width, 1);
            if it.name.is_empty() {
                if self.theme.version == Version::V1 {
                    let sep = format!("├{}┤", "─".repeat(inner.width as usize));
                    f.render_widget(
                        Paragraph::new(Span::styled(sep, drop_style)),
                        Rect::new(area.x, row.y, area.width, 1),
                    );
                } else {
                    let sep = "─".repeat(inner.width as usize);
                    f.render_widget(Paragraph::new(Span::styled(sep, drop_style)), row);
                }
                continue;
            }
            let style = if i == selected {
                Style::default().fg(t.drop_sel_fg).bg(t.drop_sel_bg)
            } else {
                drop_style
            };
            let w = inner.width as usize;
            let text = if it.shortcut.is_empty() {
                format!(" {:<pad$} ", it.name, pad = w.saturating_sub(2))
            } else {
                let gap = w.saturating_sub(it.name.len() + it.shortcut.len() + 3);
                format!(" {}{}{} ", it.name, " ".repeat(gap), it.shortcut)
            };
            if self.theme.version == Version::V1 {
                let mut item_spans = Vec::new();
                for (pos, ch) in text.char_indices() {
                    let name_pos = pos.checked_sub(1);
                    let st = if !selected.eq(&i) && name_pos.is_some() && it.accel == name_pos {
                        Style::default()
                            .fg(ratatui::style::Color::White)
                            .bg(t.drop_bg)
                    } else {
                        style
                    };
                    item_spans.push(Span::styled(ch.to_string(), st));
                }
                f.render_widget(Paragraph::new(Line::from(item_spans)), row);
            } else {
                f.render_widget(Paragraph::new(Span::styled(text, style)), row);
            }
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

    /// Render one row of text with an inverted cursor cell.
    pub(super) fn render_text_row(
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

        if cx > sx {
            let s: String = chars.iter().skip(sx).take(cx - sx).collect();
            spans.push(Span::styled(s, base));
        }

        spans.push(Span::styled(
            chars.get(cx).copied().unwrap_or(' ').to_string(),
            cur_style,
        ));

        let after_start = cx + 1;
        let after_end = (sx + vw).min(chars.len());
        if after_start < after_end {
            let s: String = chars[after_start..after_end].iter().collect();
            spans.push(Span::styled(s, base));
        }

        let used = (cx.saturating_sub(sx) + 1).min(vw) + after_end.saturating_sub(after_start);
        let rem = vw.saturating_sub(used);
        if rem > 0 {
            spans.push(Span::styled(" ".repeat(rem), base));
        }

        f.render_widget(Paragraph::new(Line::from(spans)), area);
    }

    // ── Status bar ────────────────────────────────────────────────────────────

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let (cx, cy) = self.editor.cursor;
        let ovr = if self.editor.overtype { "OVR" } else { "   " };
        let fname = self.editor.filename.as_deref().unwrap_or("Untitled");
        let w = area.width as usize;

        if self.theme.version == Version::V1 {
            let left = if self.mode == Mode::Welcome {
                " F1=Help   Enter=Execute   Esc=Cancel   Tab=Next Field   Arrow=Next Item"
                    .to_string()
            } else if matches!(self.mode, Mode::ConfirmNew | Mode::ConfirmExit) {
                " F1=Help   Enter=Execute   Esc=Cancel   Tab=Next Field   Arrow=Next Item"
                    .to_string()
            } else if let Mode::Menu { menu, item } = self.mode {
                let help = MENU_ITEMS[menu][item].help;
                format!(" F1=Help │ {}", help)
            } else if let Some(m) = &self.message {
                format!(" {}", m)
            } else {
                " MS-DOS Editor  <F1=Help> Press ALT to activate menus".to_string()
            };
            if matches!(self.mode, Mode::Welcome | Mode::ConfirmNew | Mode::ConfirmExit) {
                f.render_widget(
                    Paragraph::new(Span::styled(
                        format!("{:<w$}", left, w = w),
                        Style::default().bg(t.stat_bg).fg(t.stat_fg),
                    )),
                    area,
                );
                return;
            }
            let right = format!("{:05}:{:03}", cy + 1, cx + 1);
            let sep = "│";
            let pad = w.saturating_sub(left.len() + sep.len() + right.len());
            let line = format!("{}{}{}{}", left, " ".repeat(pad), sep, right);
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!("{:<w$}", line, w = w),
                    Style::default().bg(t.stat_bg).fg(t.stat_fg),
                )),
                area,
            );
            return;
        }

        let dirty = if self.editor.dirty { "*" } else { " " };
        let right = format!("{}  Ln:{:>4}  Col:{:>3}  {}", dirty, cy + 1, cx + 1, ovr);

        let left = if self.mode == Mode::Welcome {
            format!(" F1=Help   Enter=Execute   Esc=Cancel")
        } else if let Some(m) = &self.message {
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

        let pad = w.saturating_sub(left.len() + right_len);
        let line = format!("{}{}{}", left, " ".repeat(pad), right);

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
            Mode::Welcome => self.welcome_dialog(f),
            Mode::Open(s) => self.input_dialog(f, "Open", "File Name:", s),
            Mode::SaveAs(s) => self.input_dialog(f, "Save As", "File Name:", s),
            Mode::Find(s) => self.input_dialog(f, "Find", "Find What:", s),
            Mode::Goto(s) => self.input_dialog(f, "Go To Line", "Line Number:", s),
            Mode::ConfirmNew | Mode::ConfirmExit => self.confirm_save_dialog(f),
            Mode::About => self.about_dialog(f),
            Mode::Replace {
                find,
                replace,
                focus,
            } => self.replace_dialog(f, find, replace, *focus),
            _ => {}
        }
    }

    pub(super) fn center_rect(f: &Frame, w: u16, h: u16) -> Rect {
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

    fn confirm_save_dialog(&self, f: &mut Frame) {
        if self.theme.version == Version::V1 {
            self.confirm_save_dialog_v1(f);
            return;
        }

        self.confirm_dialog(f, "Save", "Loaded file is not saved. Save it now?");
    }

    fn confirm_save_dialog_v1(&self, f: &mut Frame) {
        let size = f.area();
        let area = Rect::new(
            (size.width.saturating_sub(46) / 2).saturating_sub(1),
            (size.height.saturating_sub(7) / 2).saturating_sub(1),
            46.min(size.width),
            7.min(size.height),
        );
        f.render_widget(Clear, area);

        let box_style = Style::default()
            .bg(ratatui::style::Color::Gray)
            .fg(ratatui::style::Color::Black);
        let hilite_style = Style::default()
            .bg(ratatui::style::Color::Gray)
            .fg(ratatui::style::Color::White);
        let shadow_style = Style::default()
            .bg(ratatui::style::Color::Black)
            .fg(ratatui::style::Color::DarkGray);
        let iw = area.width as usize - 2;

        for y in area.y + 1..area.y + area.height {
            f.render_widget(
                Paragraph::new(Span::styled("  ", shadow_style)),
                Rect::new(area.x + area.width, y, 2, 1),
            );
        }
        f.render_widget(
            Paragraph::new(Span::styled(
                " ".repeat(area.width as usize),
                shadow_style,
            )),
            Rect::new(area.x + 2, area.y + area.height, area.width, 1),
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("┌{}┐", "─".repeat(iw)), box_style)),
            Rect::new(area.x, area.y, area.width, 1),
        );

        let rows = [
            "",
            "   Loaded file is not saved. Save it now?   ",
            "",
        ];
        for (i, row) in rows.iter().enumerate() {
            let y = area.y + 1 + i as u16;
            f.render_widget(
                Paragraph::new(Span::styled("│", box_style)),
                Rect::new(area.x, y, 1, 1),
            );
            f.render_widget(
                Paragraph::new(Span::styled(format!("{:<iw$}", row), box_style)),
                Rect::new(area.x + 1, y, iw as u16, 1),
            );
            f.render_widget(
                Paragraph::new(Span::styled("│", box_style)),
                Rect::new(area.x + area.width - 1, y, 1, 1),
            );
        }

        let sep_y = area.y + 4;
        f.render_widget(
            Paragraph::new(Span::styled(format!("├{}┤", "─".repeat(iw)), box_style)),
            Rect::new(area.x, sep_y, area.width, 1),
        );

        let mut spans = Vec::new();
        for ch in "  < Yes >   <  No  >   <Cancel>   < Help >  ".chars() {
            let style = if ch == '<' || ch == '>' {
                hilite_style
            } else {
                box_style
            };
            spans.push(Span::styled(ch.to_string(), style));
        }
        f.render_widget(
            Paragraph::new(Span::styled("│", box_style)),
            Rect::new(area.x, sep_y + 1, 1, 1),
        );
        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(area.x + 1, sep_y + 1, iw as u16, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled("│", box_style)),
            Rect::new(area.x + area.width - 1, sep_y + 1, 1, 1),
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(iw)), box_style)),
            Rect::new(area.x, sep_y + 2, area.width, 1),
        );
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
                Style::default()
                    .fg(t.dlg_fg)
                    .bg(t.dlg_bg)
                    .add_modifier(Modifier::BOLD),
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
        let (fbg, ffg) = if focus == 0 {
            (t.dlg_inp_bg, t.dlg_inp_fg)
        } else {
            (t.dlg_bg, t.dlg_fg)
        };
        f.render_widget(
            Paragraph::new(format!("{:<w$}", find, w = iw as usize))
                .style(Style::default().bg(fbg).fg(ffg)),
            Rect::new(inner.x + 1, inner.y + 2, iw, 1),
        );

        f.render_widget(
            Paragraph::new("Replace With:").style(Style::default().bg(t.dlg_bg).fg(t.dlg_fg)),
            Rect::new(inner.x + 1, inner.y + 4, inner.width - 2, 1),
        );
        let (rbg, rfg) = if focus == 1 {
            (t.dlg_inp_bg, t.dlg_inp_fg)
        } else {
            (t.dlg_bg, t.dlg_fg)
        };
        f.render_widget(
            Paragraph::new(format!("{:<w$}", replace, w = iw as usize))
                .style(Style::default().bg(rbg).fg(rfg)),
            Rect::new(inner.x + 1, inner.y + 5, iw, 1),
        );

        self.btn(f, inner.x + 1, inner.y + 7, "[ OK ]");
        self.btn(f, inner.x + 9, inner.y + 7, "[ Cancel ]");
    }

    pub(super) fn btn(&self, f: &mut Frame, x: u16, y: u16, label: &str) {
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
            Mode::Welcome => {
                self.mode = Mode::Normal;
                false
            }
            Mode::Menu { menu, item } => self.key_menu(key, menu, item),
            Mode::Open(_) => self.key_open(key),
            Mode::SaveAs(_) => self.key_save_as(key),
            Mode::Find(_) => self.key_find(key),
            Mode::Goto(_) => self.key_goto(key),
            Mode::Replace {
                find,
                replace,
                focus,
            } => self.key_replace(key, find, replace, focus),
            Mode::ConfirmNew => self.key_confirm_new(key),
            Mode::ConfirmExit => self.key_confirm_exit(key),
            Mode::About => {
                self.mode = Mode::Normal;
                false
            }
        }
    }

    fn key_normal(&mut self, key: KeyEvent) -> bool {
        let m = key.modifiers;
        match key.code {
            // Bare Alt key (requires kitty keyboard protocol support in the terminal)
            KeyCode::Modifier(ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt) => {
                self.mode = Mode::Menu { menu: 0, item: 0 };
            }
            KeyCode::Char('f') | KeyCode::Char('F') if m.contains(KeyModifiers::ALT) => {
                self.mode = Mode::Menu { menu: 0, item: 0 };
            }
            KeyCode::Char('e') | KeyCode::Char('E') if m.contains(KeyModifiers::ALT) => {
                self.mode = Mode::Menu { menu: 1, item: 0 };
            }
            KeyCode::Char('s') | KeyCode::Char('S') if m.contains(KeyModifiers::ALT) => {
                self.mode = Mode::Menu { menu: 2, item: 0 };
            }
            KeyCode::Char('o') | KeyCode::Char('O') if m.contains(KeyModifiers::ALT) => {
                self.mode = Mode::Menu { menu: 3, item: 0 };
            }
            KeyCode::Char('h') | KeyCode::Char('H') if m.contains(KeyModifiers::ALT) => {
                self.mode = Mode::Menu { menu: 4, item: 0 };
            }

            KeyCode::Char('s') if m == KeyModifiers::CONTROL => self.do_save(),
            KeyCode::F(2) => self.do_save(),

            KeyCode::Char('f') if m == KeyModifiers::CONTROL => {
                self.mode = Mode::Find(self.last_find.clone());
            }
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
            KeyCode::Char('g') if m == KeyModifiers::CONTROL => {
                self.mode = Mode::Goto(String::new());
            }

            KeyCode::Char('x') if m == KeyModifiers::CONTROL => {
                self.editor.cut_line();
            }
            KeyCode::Char('c') if m == KeyModifiers::CONTROL => {
                self.editor.copy_line();
            }
            KeyCode::Char('v') if m == KeyModifiers::CONTROL => {
                self.editor.paste();
            }
            KeyCode::Delete if m == KeyModifiers::SHIFT => {
                self.editor.cut_line();
            }
            KeyCode::Insert if m == KeyModifiers::CONTROL => {
                self.editor.copy_line();
            }
            KeyCode::Insert if m == KeyModifiers::SHIFT => {
                self.editor.paste();
            }

            KeyCode::Left => self.editor.cursor_left(),
            KeyCode::Right => self.editor.cursor_right(),
            KeyCode::Up => self.editor.cursor_up(),
            KeyCode::Down => self.editor.cursor_down(),
            KeyCode::Home if m == KeyModifiers::CONTROL => {
                self.editor.cursor = (0, 0);
            }
            KeyCode::End if m == KeyModifiers::CONTROL => {
                let last = self.editor.lines.len().saturating_sub(1);
                let col = self.editor.lines[last].chars().count();
                self.editor.cursor = (col, last);
            }
            KeyCode::Home => self.editor.home(),
            KeyCode::End => self.editor.end(),
            KeyCode::PageUp => self.editor.page_up(self.page_height),
            KeyCode::PageDown => self.editor.page_down(self.page_height),

            KeyCode::Insert => {
                self.editor.overtype = !self.editor.overtype;
            }
            KeyCode::Delete => self.editor.delete(),
            KeyCode::Backspace => self.editor.backspace(),
            KeyCode::Enter => self.editor.insert_newline(),
            KeyCode::Tab => {
                for _ in 0..4 {
                    self.editor.insert_char(' ');
                }
            }
            KeyCode::Char(c) => {
                self.message = None;
                self.editor.insert_char(c);
            }

            _ => {}
        }
        false
    }

    fn key_menu(&mut self, key: KeyEvent, mi: usize, ii: usize) -> bool {
        match key.code {
            // Bare Alt or Esc closes the menu
            KeyCode::Modifier(ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt)
            | KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Left => {
                let nm = if mi == 0 { MENUS.len() - 1 } else { mi - 1 };
                self.mode = Mode::Menu { menu: nm, item: 0 };
            }
            KeyCode::Right => {
                self.mode = Mode::Menu {
                    menu: (mi + 1) % MENUS.len(),
                    item: 0,
                };
            }
            KeyCode::Up => {
                let items = MENU_ITEMS[mi];
                let mut ni = if ii == 0 { items.len() - 1 } else { ii - 1 };
                while items[ni].name.is_empty() {
                    ni = if ni == 0 { items.len() - 1 } else { ni - 1 };
                }
                self.mode = Mode::Menu { menu: mi, item: ni };
            }
            KeyCode::Down => {
                let items = MENU_ITEMS[mi];
                let mut ni = (ii + 1) % items.len();
                while items[ni].name.is_empty() {
                    ni = (ni + 1) % items.len();
                }
                self.mode = Mode::Menu { menu: mi, item: ni };
            }
            KeyCode::Enter => return self.activate(mi, ii),
            KeyCode::Char(c) => {
                let needle = c.to_ascii_lowercase();
                if let Some((idx, _)) = MENU_ITEMS[mi]
                    .iter()
                    .enumerate()
                    .find(|(_, it)| {
                        it.accel
                            .and_then(|pos| it.name.chars().nth(pos))
                            .map(|ch| ch.to_ascii_lowercase() == needle)
                            .unwrap_or(false)
                    })
                {
                    return self.activate(mi, idx);
                }
            }
            _ => {}
        }
        false
    }

    fn activate(&mut self, mi: usize, ii: usize) -> bool {
        self.mode = Mode::Normal;
        match (mi, ii) {
            (0, 0) => self.do_new(),
            (0, 1) => {
                self.mode = Mode::Open(String::new());
            }
            (0, 2) => self.do_save(),
            (0, 3) => {
                let n = self.editor.filename.clone().unwrap_or_default();
                self.mode = Mode::SaveAs(n);
            }
            (0, 5) => {}
            (0, 7) => {
                if self.editor.dirty {
                    self.mode = Mode::ConfirmExit;
                    return false;
                }
                return true;
            }
            (1, 0) => self.editor.cut_line(),
            (1, 1) => self.editor.copy_line(),
            (1, 2) => self.editor.paste(),
            (1, 3) => self.editor.delete(),
            (2, 0) => {
                self.mode = Mode::Find(self.last_find.clone());
            }
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
            (4, 3) => {
                self.mode = Mode::About;
            }
            _ => {}
        }
        false
    }

    fn key_open(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
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
            KeyCode::Backspace => {
                if let Mode::Open(ref mut s) = self.mode {
                    s.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Mode::Open(ref mut s) = self.mode {
                    s.push(c);
                }
            }
            _ => {}
        }
        false
    }

    fn key_save_as(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
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
                            Err(e) => {
                                self.message = Some(format!("Error: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Backspace => {
                if let Mode::SaveAs(ref mut s) = self.mode {
                    s.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Mode::SaveAs(ref mut s) = self.mode {
                    s.push(c);
                }
            }
            _ => {}
        }
        false
    }

    fn key_find(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
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
            KeyCode::Backspace => {
                if let Mode::Find(ref mut s) = self.mode {
                    s.pop();
                }
            }
            KeyCode::Char(c) => {
                if let Mode::Find(ref mut s) = self.mode {
                    s.push(c);
                }
            }
            _ => {}
        }
        false
    }

    fn key_goto(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                if let Mode::Goto(ref s) = self.mode.clone() {
                    let line = s.parse::<usize>().unwrap_or(0);
                    self.editor.goto_line(line);
                    self.mode = Mode::Normal;
                }
            }
            KeyCode::Backspace => {
                if let Mode::Goto(ref mut s) = self.mode {
                    s.pop();
                }
            }
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if let Mode::Goto(ref mut s) = self.mode {
                    s.push(c);
                }
            }
            _ => {}
        }
        false
    }

    fn key_replace(&mut self, key: KeyEvent, find: String, replace: String, focus: usize) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Tab => {
                self.mode = Mode::Replace {
                    find,
                    replace,
                    focus: 1 - focus,
                };
            }
            KeyCode::Enter => {
                self.last_find = find.clone();
                let mut count = 0usize;
                for line in &mut self.editor.lines {
                    while let Some(pos) = line.find(&find) {
                        line.replace_range(pos..pos + find.len(), &replace);
                        count += 1;
                        if find.is_empty() {
                            break;
                        }
                    }
                }
                self.editor.dirty = count > 0;
                self.editor.highlight();
                self.mode = Mode::Normal;
                self.message = Some(format!("{} replacement(s) made", count));
            }
            KeyCode::Backspace => {
                if focus == 0 {
                    let mut f2 = find;
                    f2.pop();
                    self.mode = Mode::Replace {
                        find: f2,
                        replace,
                        focus,
                    };
                } else {
                    let mut r2 = replace;
                    r2.pop();
                    self.mode = Mode::Replace {
                        find,
                        replace: r2,
                        focus,
                    };
                }
            }
            KeyCode::Char(c) => {
                if focus == 0 {
                    let mut f2 = find;
                    f2.push(c);
                    self.mode = Mode::Replace {
                        find: f2,
                        replace,
                        focus,
                    };
                } else {
                    let mut r2 = replace;
                    r2.push(c);
                    self.mode = Mode::Replace {
                        find,
                        replace: r2,
                        focus,
                    };
                }
            }
            _ => {}
        }
        false
    }

    fn key_confirm_new(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                if self.editor.filename.is_some() {
                    self.do_save();
                    if !self.editor.dirty {
                        self.new_confirmed();
                    }
                } else {
                    self.mode = Mode::SaveAs(String::new());
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => self.new_confirmed(),
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') => {
                self.mode = Mode::Normal;
            }
            _ => {}
        }
        false
    }

    fn key_confirm_exit(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(path) = self.editor.filename.clone() {
                    match self.editor.save_file(&path) {
                        Ok(_) => return true,
                        Err(e) => {
                            self.message = Some(format!("Error saving: {}", e));
                            self.mode = Mode::Normal;
                        }
                    }
                } else {
                    self.mode = Mode::SaveAs(String::new());
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => return true,
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') => {
                self.mode = Mode::Normal;
            }
            _ => {}
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
            let ey = my as usize - 2 + self.editor.scroll.1 as usize;
            let ex = mx as usize - 1 + self.editor.scroll.0 as usize;
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
                Err(e) => {
                    self.message = Some(format!("Error saving: {}", e));
                }
            },
            None => {
                self.mode = Mode::SaveAs(String::new());
            }
        }
    }

    fn do_new(&mut self) {
        if self.editor.dirty {
            self.mode = Mode::ConfirmNew;
        } else {
            self.new_confirmed();
        }
    }

    fn new_confirmed(&mut self) {
        self.editor = Editor::new();
        self.mode = Mode::Normal;
        self.message = None;
    }
}
