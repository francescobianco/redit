use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers, ModifierKeyCode, MouseButton, MouseEventKind,
};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::{env, fs, io};
use unicode_width::UnicodeWidthChar;

use crate::editor::Editor;
use crate::settings::UserSettings;
use crate::theme::{Theme, Version};

mod v1;
mod v2;

// ── V1 color palette ─────────────────────────────────────────────────────────
const V1_FG_COLORS: &[(&str, Color)] = &[
    ("Black", Color::Black),
    ("Blue", Color::Blue),
    ("Green", Color::Green),
    ("Cyan", Color::Cyan),
    ("Red", Color::Red),
    ("Magenta", Color::Magenta),
    ("Brown", Color::Yellow),
    ("White", Color::Gray),
    ("Gray", Color::DarkGray),
    ("BrBlue", Color::LightBlue),
    ("BrGreen", Color::LightGreen),
    ("BrCyan", Color::LightCyan),
    ("BrRed", Color::LightRed),
    ("Pink", Color::LightMagenta),
    ("Yellow", Color::LightYellow),
    ("BrWhite", Color::White),
];

const V1_BG_COLORS: &[(&str, Color)] = &[
    ("Black", Color::Black),
    ("Blue", Color::Blue),
    ("Green", Color::Green),
    ("Cyan", Color::Cyan),
    ("Red", Color::Red),
    ("Magenta", Color::Magenta),
    ("Brown", Color::Yellow),
    ("White", Color::Gray),
];

fn v1_fg_index(color: Color) -> usize {
    V1_FG_COLORS
        .iter()
        .position(|(_, c)| *c == color)
        .unwrap_or(15)
}

fn v1_bg_index(color: Color) -> usize {
    V1_BG_COLORS
        .iter()
        .position(|(_, c)| *c == color)
        .unwrap_or(1)
}

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
    &[
        item("Display...", "", Some(0), "Changes display colors and style"),
        item(
            "Help Path...",
            "",
            Some(0),
            "Sets the path to the EDIT.HLP help file",
        ),
    ],
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
    DisplaySettings {
        // foreground color index into V1_FG_COLORS (0-15)
        fg_idx: usize,
        // background color index into V1_BG_COLORS (0-7)
        bg_idx: usize,
        // scroll offset for the fg list (0 = top, max = 8)
        fg_scroll: usize,
        // Display Options section
        scroll_bars: bool,
        tab_stops: u8,
        // focused panel: 0=fg_list 1=bg_list 2=scroll_bars 3=tab_stops 4=ok 5=cancel 6=help
        focus: usize,
    },
    HelpGettingStarted { scroll: usize },
    HelpKeyboard { scroll: usize },
}

pub struct App {
    pub(super) editor: Editor,
    pub(super) mode: Mode,
    pub(super) last_find: String,
    pub(super) message: Option<String>,
    pub(super) page_height: usize,
    pub(super) settings: UserSettings,
    pub theme: Theme,
}

impl App {
    pub fn new() -> Self {
        let mut editor = Editor::new();
        let args: Vec<String> = std::env::args().collect();
        let mut settings = UserSettings::load();
        let mut filename = None;
        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--v1" => settings.set_style(Version::V1),
                "--v2" => settings.set_style(Version::V2),
                _ if filename.is_none() => filename = Some(arg.clone()),
                _ => {}
            }
        }
        let theme = settings.theme();
        if let Some(path) = filename.as_deref() {
            let _ = editor.load_file(path);
        }
        let initial_mode = if filename.is_none() {
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
            settings,
            theme,
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

        let show_accels = matches!(self.mode, Mode::Menu { .. });
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
                let st = if show_accels && j == acc_pos { ac } else { fg };
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
        let x = if self.theme.version == Version::V1 && menu_idx == MENUS.len() - 1 {
            f.area().width.saturating_sub(width)
        } else {
            Self::menu_x(menu_idx)
        };
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
    /// Returns the display width of a character, expanding tabs to the next tab stop.
    /// `col` is the current display column (0-based) before this character.
    fn char_display_width(c: char, col: usize, tab_stop: usize) -> usize {
        if c == '\t' {
            tab_stop - (col % tab_stop)
        } else {
            UnicodeWidthChar::width(c).unwrap_or(0)
        }
    }

    /// Builds a screen-ready string of exactly `vw` display columns from `chars`
    /// starting at char index `sx`, with tab expansion.  Returns (string, cursor_col)
    /// where cursor_col is the display column of `cx` within the returned string
    /// (or None when cx is not in the visible range).
    fn chars_to_display(
        chars: &[char],
        sx: usize,
        cx: usize,
        vw: usize,
        tab_stop: usize,
        cursor_here: bool,
    ) -> (String, Option<usize>, usize) {
        // First: compute column of sx in the raw line so tab positions are correct.
        let mut raw_col = 0usize;
        for &c in chars.iter().take(sx) {
            raw_col += Self::char_display_width(c, raw_col, tab_stop);
        }

        let mut out = String::new();
        let mut used_cols = 0usize;
        let mut cursor_col: Option<usize> = None;

        for (i, &c) in chars.iter().enumerate().skip(sx) {
            let w = Self::char_display_width(c, raw_col, tab_stop);
            if used_cols + w > vw {
                break;
            }
            if cursor_here && i == cx {
                cursor_col = Some(used_cols);
            }
            if c == '\t' {
                out.push_str(&" ".repeat(w));
            } else {
                out.push(c);
            }
            used_cols += w;
            raw_col += w;
        }
        // If cursor is past the end of content (empty line or cx == len)
        if cursor_here && cursor_col.is_none() && cx >= chars.len() && used_cols < vw {
            cursor_col = Some(used_cols);
            out.push(' ');
            used_cols += 1;
        }
        // Pad remainder
        if used_cols < vw {
            out.push_str(&" ".repeat(vw - used_cols));
        }
        (out, cursor_col, used_cols)
    }

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
        let tab_stop = 8; // TODO: wire up settings.tab_stops

        let (display, cursor_col, _) =
            Self::chars_to_display(chars, sx, cx, vw, tab_stop, cursor_here);

        let Some(cc) = cursor_col else {
            // No cursor on this row — render as a single styled span
            f.render_widget(
                Paragraph::new(Span::styled(display, base)),
                area,
            );
            return;
        };

        // Split display string at the cursor column
        // display is already exactly vw display cols wide
        let bytes_before = display
            .char_indices()
            .scan(0usize, |col, (byte_i, c)| {
                if *col < cc {
                    *col += UnicodeWidthChar::width(c).unwrap_or(1);
                    Some(byte_i)
                } else {
                    None
                }
            })
            .last()
            .map(|bi| {
                // advance past last char
                let c = display[bi..].chars().next().unwrap();
                bi + c.len_utf8()
            })
            .unwrap_or(0);

        let before = &display[..bytes_before];
        // cursor char: may be a multi-byte char; take chars until width >= 1
        let mut cur_end = bytes_before;
        let mut cur_w = 0usize;
        for c in display[bytes_before..].chars() {
            cur_end += c.len_utf8();
            cur_w += UnicodeWidthChar::width(c).unwrap_or(1);
            if cur_w >= 1 {
                break;
            }
        }
        let cursor_ch = &display[bytes_before..cur_end];
        let after = &display[cur_end..];

        let mut spans = Vec::with_capacity(3);
        if !before.is_empty() {
            spans.push(Span::styled(before.to_string(), base));
        }
        spans.push(Span::styled(
            if cursor_ch.is_empty() { " ".to_string() } else { cursor_ch.to_string() },
            cur_style,
        ));
        if !after.is_empty() {
            spans.push(Span::styled(after.to_string(), base));
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
            if matches!(
                self.mode,
                Mode::Welcome | Mode::ConfirmNew | Mode::ConfirmExit
            ) {
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
            Mode::SaveAs(s) => self.save_as_dialog(f, s),
            Mode::Find(s) => self.input_dialog(f, "Find", "Find What:", s),
            Mode::Goto(s) => self.input_dialog(f, "Go To Line", "Line Number:", s),
            Mode::ConfirmNew | Mode::ConfirmExit => self.confirm_save_dialog(f),
            Mode::About => self.about_dialog(f),
            Mode::Replace {
                find,
                replace,
                focus,
            } => self.replace_dialog(f, find, replace, *focus),
            Mode::DisplaySettings {
                fg_idx,
                bg_idx,
                fg_scroll,
                scroll_bars,
                tab_stops,
                focus,
            } => self.display_settings_dialog(
                f,
                *fg_idx,
                *bg_idx,
                *fg_scroll,
                *scroll_bars,
                *tab_stops,
                *focus,
            ),
            Mode::HelpGettingStarted { scroll } => self.help_getting_started(f, *scroll),
            Mode::HelpKeyboard { scroll } => self.help_keyboard(f, *scroll),
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

    fn save_as_dialog(&self, f: &mut Frame, input: &str) {
        let size = f.area();
        let area = Rect::new(
            0,
            1.min(size.height),
            size.width,
            size.height.saturating_sub(2),
        );
        if area.width < 12 || area.height < 8 {
            return;
        }
        f.render_widget(Clear, area);

        let box_style = Style::default().bg(Color::Gray).fg(Color::Black);
        let title_style = Style::default().bg(Color::White).fg(Color::Black);
        let accel_style = Style::default().bg(Color::Gray).fg(Color::White);
        let shadow_style = Style::default().bg(Color::Black).fg(Color::DarkGray);
        let inner_w = area.width.saturating_sub(2) as usize;

        if area.x + area.width + 2 <= size.width {
            for y in area.y + 1..area.y + area.height {
                f.render_widget(
                    Paragraph::new(Span::styled("  ", shadow_style)),
                    Rect::new(area.x + area.width, y, 2, 1),
                );
            }
        }
        if area.y + area.height < size.height.saturating_sub(1) {
            f.render_widget(
                Paragraph::new(Span::styled(" ".repeat(area.width as usize), shadow_style)),
                Rect::new(
                    area.x + 2,
                    area.y + area.height,
                    area.width.saturating_sub(2),
                    1,
                ),
            );
        }

        let title = " Save As ";
        let side = inner_w.saturating_sub(title.len());
        let left = side / 2;
        let right = side - left;
        let mut top = Vec::new();
        top.push(Span::styled("┌", box_style));
        top.push(Span::styled("─".repeat(left), box_style));
        top.push(Span::styled(
            format!("{:^w$}", title, w = title.len()),
            title_style,
        ));
        top.push(Span::styled("─".repeat(right), box_style));
        top.push(Span::styled("┐", box_style));
        f.render_widget(
            Paragraph::new(Line::from(top)),
            Rect::new(area.x, area.y, area.width, 1),
        );

        for row in 1..area.height.saturating_sub(1) {
            let y = area.y + row;
            f.render_widget(
                Paragraph::new(Span::styled("│", box_style)),
                Rect::new(area.x, y, 1, 1),
            );
            f.render_widget(
                Paragraph::new(Span::styled(" ".repeat(inner_w), box_style)),
                Rect::new(area.x + 1, y, inner_w as u16, 1),
            );
            f.render_widget(
                Paragraph::new(Span::styled("│", box_style)),
                Rect::new(area.x + area.width - 1, y, 1, 1),
            );
        }
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("└{}┘", "─".repeat(inner_w)),
                box_style,
            )),
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        );

        let label_x = area.x + ((area.width.saturating_sub(50)) / 2).max(4);
        let field_w = inner_w.saturating_sub(18).min(32);
        let field = format!("[{:<w$}]", input, w = field_w);
        let mut file_line = Vec::new();
        file_line.push(Span::styled("File ", box_style));
        file_line.push(Span::styled("N", accel_style));
        file_line.push(Span::styled("ame: ", box_style));
        file_line.push(Span::styled(field, box_style));
        f.render_widget(
            Paragraph::new(Line::from(file_line)),
            Rect::new(label_x, area.y + 2, inner_w as u16 - 4, 1),
        );

        let cwd = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let cwd_text = Self::fit_text(&cwd, inner_w.saturating_sub(6));
        f.render_widget(
            Paragraph::new(Span::styled(cwd_text, box_style)),
            Rect::new(label_x, area.y + 4, inner_w as u16 - 4, 1),
        );

        let mut labels = Vec::new();
        labels.push(Span::styled(" Existing ", box_style));
        labels.push(Span::styled("F", accel_style));
        labels.push(Span::styled("iles:         ", box_style));
        labels.push(Span::styled("D", accel_style));
        labels.push(Span::styled("irectories:", box_style));
        f.render_widget(
            Paragraph::new(Line::from(labels)),
            Rect::new(label_x, area.y + 6, inner_w as u16 - 4, 1),
        );

        let (files, dirs) = Self::dir_listing();
        self.render_save_as_list(f, Rect::new(label_x, area.y + 7, 23, 11), &files);
        self.render_save_as_list(f, Rect::new(label_x + 25, area.y + 7, 21, 11), &dirs);

        self.render_dos_button(
            f,
            label_x + 2,
            area.y + area.height - 3,
            "►  OK  ◄",
            None,
            true,
        );
        self.render_dos_button(
            f,
            label_x + 18,
            area.y + area.height - 3,
            "  Cancel  ",
            None,
            false,
        );
        self.render_dos_button(
            f,
            label_x + 35,
            area.y + area.height - 3,
            "  Help  ",
            Some(2),
            false,
        );

        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "{:<w$}",
                    "F1=Help  Enter=Execute  Esc=Cancel  Tab=Next Field",
                    w = size.width as usize
                ),
                Style::default()
                    .bg(self.theme.stat_bg)
                    .fg(self.theme.stat_fg),
            )),
            Rect::new(0, size.height.saturating_sub(1), size.width, 1),
        );
    }

    fn render_save_as_list(&self, f: &mut Frame, area: Rect, entries: &[String]) {
        let style = Style::default().bg(Color::Gray).fg(Color::Black);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("┌{}┐", "─".repeat(area.width as usize - 2)),
                style,
            )),
            Rect::new(area.x, area.y, area.width, 1),
        );
        let rows = area.height.saturating_sub(2) as usize;
        for i in 0..rows {
            let text = entries.get(i).map(String::as_str).unwrap_or("");
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!(
                        "│ {:<w$}│",
                        Self::fit_text(text, area.width as usize - 4),
                        w = area.width as usize - 3
                    ),
                    style,
                )),
                Rect::new(area.x, area.y + 1 + i as u16, area.width, 1),
            );
        }
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("└{}┘", "─".repeat(area.width as usize - 2)),
                style,
            )),
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        );
    }

    fn render_dos_button(
        &self,
        f: &mut Frame,
        x: u16,
        y: u16,
        label: &str,
        accel: Option<usize>,
        selected: bool,
    ) {
        let style = Style::default().bg(Color::White).fg(Color::Black);
        let accel_style = Style::default().bg(Color::White).fg(Color::Red);
        let shadow_style = Style::default().bg(Color::Gray).fg(Color::Black);
        let mut spans = Vec::new();
        for (i, ch) in label.chars().enumerate() {
            spans.push(Span::styled(
                ch.to_string(),
                if accel == Some(i) { accel_style } else { style },
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(x, y, label.chars().count() as u16, 1),
        );
        let shadow = if selected {
            "▀▀▀▀▀▀▀▀"
        } else {
            "▀▀▀▀▀▀▀▀▀▀"
        };
        f.render_widget(
            Paragraph::new(Span::styled(shadow, shadow_style)),
            Rect::new(x + 1, y + 1, shadow.len() as u16, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled("▄", shadow_style)),
            Rect::new(x + label.chars().count() as u16, y, 1, 1),
        );
    }

    fn dir_listing() -> (Vec<String>, Vec<String>) {
        let mut files = Vec::new();
        let mut dirs = Vec::new();
        if let Ok(read_dir) = fs::read_dir(".") {
            for entry in read_dir.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    dirs.push(format!("[{}]", name));
                } else {
                    files.push(name);
                }
            }
        }
        files.sort_by_key(|s| s.to_ascii_lowercase());
        dirs.sort_by_key(|s| s.to_ascii_lowercase());
        files.truncate(9);
        dirs.truncate(9);
        (files, dirs)
    }

    fn fit_text(text: &str, width: usize) -> String {
        let mut out: String = text.chars().take(width).collect();
        if text.chars().count() > width && width > 0 {
            out.pop();
            out.push('…');
        }
        out
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
            Paragraph::new(Span::styled(" ".repeat(area.width as usize), shadow_style)),
            Rect::new(area.x + 2, area.y + area.height, area.width, 1),
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("┌{}┐", "─".repeat(iw)), box_style)),
            Rect::new(area.x, area.y, area.width, 1),
        );

        let rows = ["", "   Loaded file is not saved. Save it now?   ", ""];
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

    fn display_settings_dialog(
        &self,
        f: &mut Frame,
        fg_idx: usize,
        bg_idx: usize,
        fg_scroll: usize,
        scroll_bars: bool,
        tab_stops: u8,
        focus: usize,
    ) {
        // V1-faithful Display dialog:
        //   Colors section  (Foreground + Background list boxes)
        //   Display Options section  ([X] Scroll Bars   Tab Stops: N)
        //   < OK >  < Cancel >  < Help >
        if f.area().width < 68 || f.area().height < 22 {
            self.confirm_dialog(f, "Display", "Terminal is too small.");
            return;
        }

        // Outer dialog box: 66 wide, 22 tall (faithful to original V1)
        let area = Self::center_rect(f, 66, 22);
        f.render_widget(Clear, area);

        let box_s = Style::default().bg(Color::Gray).fg(Color::Black);
        let title_s = Style::default().bg(Color::White).fg(Color::Black);
        let shadow_s = Style::default().bg(Color::Black).fg(Color::DarkGray);
        let sel_s = Style::default().bg(Color::Black).fg(Color::White);
        let inner_w = area.width.saturating_sub(2) as usize;

        // Drop shadow
        if area.x + area.width + 1 < f.area().width {
            for row in 1..area.height {
                f.render_widget(
                    Paragraph::new(Span::styled("  ", shadow_s)),
                    Rect::new(area.x + area.width, area.y + row, 2, 1),
                );
            }
        }
        if area.y + area.height < f.area().height {
            f.render_widget(
                Paragraph::new(Span::styled(
                    " ".repeat(area.width.saturating_sub(2) as usize),
                    shadow_s,
                )),
                Rect::new(area.x + 2, area.y + area.height, area.width.saturating_sub(2), 1),
            );
        }

        // Outer box borders
        self.draw_box(f, area, box_s, " Display ", title_s);

        // ── Colors sub-box (inner 58 wide, starts at row 1 of inner) ──────────
        // Colors box: x=area.x+1, y=area.y+1, width=64, height=13
        let cb_x = area.x + 1;
        let cb_y = area.y + 1;
        let cb_w: u16 = area.width.saturating_sub(2);
        let cb_h: u16 = 14;
        self.draw_box(
            f,
            Rect::new(cb_x, cb_y, cb_w, cb_h),
            box_s,
            " Colors ",
            title_s,
        );

        // Column headers inside Colors box (inner area starts at cb_x+1, cb_y+1)
        let ci_x = cb_x + 1; // inner x of Colors box
        let ci_y = cb_y + 1; // inner y of Colors box
        let ci_w = cb_w.saturating_sub(2) as usize; // inner width

        // Header: "                             Foreground   Background"
        // Label column: 28 chars, list1 at 28, list2 at 42
        let label_w: usize = 26;
        let list_inner: usize = 10; // content inside each list box
        let list_w: usize = list_inner + 2; // 12 including borders
        let gap: usize = 2;

        let hdr_spaces = label_w + 1; // 27 spaces before "Foreground"
        let header = format!(
            "{:hdr$}Foreground   Background",
            "",
            hdr = hdr_spaces + 1
        );
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{:<w$}", header, w = ci_w),
                box_s,
            )),
            Rect::new(ci_x, ci_y, ci_w as u16, 1),
        );

        // List box tops
        let lb1_x = ci_x + label_w as u16;
        let lb2_x = lb1_x + list_w as u16 + gap as u16;
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("┌{}┐", "─".repeat(list_inner)),
                box_s,
            )),
            Rect::new(lb1_x, ci_y + 1, list_w as u16, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("┌{}┐", "─".repeat(list_inner)),
                box_s,
            )),
            Rect::new(lb2_x, ci_y + 1, list_w as u16, 1),
        );

        // List box contents (8 rows)
        for row in 0..8usize {
            let ry = ci_y + 2 + row as u16;

            // Foreground list
            let fg_item_idx = fg_scroll + row;
            let fg_name = if fg_item_idx < V1_FG_COLORS.len() {
                V1_FG_COLORS[fg_item_idx].0
            } else {
                ""
            };
            let fg_sel = fg_item_idx == fg_idx;
            let fg_sb = self.list_scrollbar_char(row, fg_idx, fg_scroll, V1_FG_COLORS.len());
            let fg_content = format!(" {:<8}{}  ", fg_name, fg_sb);
            // Left border
            f.render_widget(
                Paragraph::new(Span::styled("│", box_s)),
                Rect::new(lb1_x, ry, 1, 1),
            );
            // Content
            f.render_widget(
                Paragraph::new(Span::styled(&fg_content, if fg_sel { Style::default().bg(Color::Black).fg(Color::White) } else { box_s })),
                Rect::new(lb1_x + 1, ry, list_inner as u16, 1),
            );
            // Right border
            f.render_widget(
                Paragraph::new(Span::styled("│", box_s)),
                Rect::new(lb1_x + list_w as u16 - 1, ry, 1, 1),
            );

            // Background list (no scrolling, shows all 8)
            let bg_name = if row < V1_BG_COLORS.len() {
                V1_BG_COLORS[row].0
            } else {
                ""
            };
            let bg_sel = row == bg_idx;
            let bg_sb = self.list_scrollbar_char(row, bg_idx, 0, V1_BG_COLORS.len());
            let bg_content = format!(" {:<8}{}  ", bg_name, bg_sb);
            // Left border
            f.render_widget(
                Paragraph::new(Span::styled("│", box_s)),
                Rect::new(lb2_x, ry, 1, 1),
            );
            // Content
            f.render_widget(
                Paragraph::new(Span::styled(&bg_content, if bg_sel { Style::default().bg(Color::Black).fg(Color::White) } else { box_s })),
                Rect::new(lb2_x + 1, ry, list_inner as u16, 1),
            );
            // Right border
            f.render_widget(
                Paragraph::new(Span::styled("│", box_s)),
                Rect::new(lb2_x + list_w as u16 - 1, ry, 1, 1),
            );

            // Label area (left column) - show text for middle rows
            let label_text = match row {
                3 => "     Set colors for the",
                4 => "     text editor window:",
                _ => "",
            };
            if !label_text.is_empty() {
                f.render_widget(
                    Paragraph::new(Span::styled(label_text, box_s)),
                    Rect::new(ci_x, ry, label_w as u16, 1),
                );
            }
        }

        // List box bottoms
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("└{}┘", "─".repeat(list_inner)),
                box_s,
            )),
            Rect::new(lb1_x, ci_y + 10, list_w as u16, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("└{}┘", "─".repeat(list_inner)),
                box_s,
            )),
            Rect::new(lb2_x, ci_y + 10, list_w as u16, 1),
        );

        // ── Display Options sub-box ───────────────────────────────────────────
        let ob_y = area.y + 16;
        let ob_w = area.width.saturating_sub(2);
        self.draw_box(
            f,
            Rect::new(cb_x, ob_y, ob_w, 3),
            box_s,
            " Display Options ",
            title_s,
        );
        // Content row
        let sb_check = if scroll_bars { "[X]" } else { "[ ]" };
        let sb_style = if focus == 2 { sel_s } else { box_s };
        let ts_style = if focus == 3 { sel_s } else { box_s };
        let sb_label = format!("   {} Scroll Bars", sb_check);
        let ts_label = format!("Tab Stops: {}", tab_stops);
        f.render_widget(
            Paragraph::new(Span::styled(&sb_label, sb_style)),
            Rect::new(cb_x + 1, ob_y + 1, (sb_label.len()) as u16, 1),
        );
        // Tab stops - right portion of the row
        let ts_x = cb_x + 1 + 38;
        if ts_x + ts_label.len() as u16 <= area.x + area.width - 1 {
            f.render_widget(
                Paragraph::new(Span::styled(&ts_label, ts_style)),
                Rect::new(ts_x, ob_y + 1, ts_label.len() as u16, 1),
            );
        }

        // ── Separator + Buttons ───────────────────────────────────────────────
        let sep_y = area.y + area.height - 3;
        let btn_y = area.y + area.height - 2;
        // Separator line (├───┤ style)
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("├{}┤", "─".repeat(inner_w)),
                box_s,
            )),
            Rect::new(area.x, sep_y, area.width, 1),
        );

        // Buttons  < OK >   < Cancel >   < Help >
        let ok_s = if focus == 4 { sel_s } else { box_s };
        let ca_s = if focus == 5 { sel_s } else { box_s };
        let he_s = if focus == 6 { sel_s } else { box_s };
        let ok_x = area.x + 9;
        let ca_x = ok_x + 14;
        let he_x = ca_x + 14;
        f.render_widget(
            Paragraph::new(Span::styled("< OK >", ok_s)),
            Rect::new(ok_x, btn_y, 6, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled("< Cancel >", ca_s)),
            Rect::new(ca_x, btn_y, 10, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled("< Help >", he_s)),
            Rect::new(he_x, btn_y, 8, 1),
        );

        // Focus highlights on list box borders
        for (lx, fi) in [(lb1_x, 0usize), (lb2_x, 1)] {
            if focus == fi {
                let box_top_y = ci_y + 1;
                let box_bot_y = ci_y + 10;
                f.render_widget(
                    Paragraph::new(Span::styled(
                        format!("┌{}┐", "─".repeat(list_inner)),
                        sel_s,
                    )),
                    Rect::new(lx, box_top_y, list_w as u16, 1),
                );
                f.render_widget(
                    Paragraph::new(Span::styled(
                        format!("└{}┘", "─".repeat(list_inner)),
                        sel_s,
                    )),
                    Rect::new(lx, box_bot_y, list_w as u16, 1),
                );
                for rr in 0..8u16 {
                    f.render_widget(
                        Paragraph::new(Span::styled("│", sel_s)),
                        Rect::new(lx, box_top_y + 1 + rr, 1, 1),
                    );
                    f.render_widget(
                        Paragraph::new(Span::styled("│", sel_s)),
                        Rect::new(lx + list_w as u16 - 1, box_top_y + 1 + rr, 1, 1),
                    );
                }
            }
        }
    }

    fn list_scrollbar_char(
        &self,
        row: usize,
        sel_idx: usize,
        _scroll: usize,
        total: usize,
    ) -> char {
        let list_rows = 8usize;
        let track_rows = list_rows - 2; // 6 inner rows (rows 1-6)
        if row == 0 {
            return '↑';
        }
        if row == list_rows - 1 {
            return '↓';
        }
        let thumb = if total <= 1 {
            1
        } else {
            1 + (sel_idx * (track_rows - 1)) / (total - 1)
        };
        if row == thumb {
            ' '
        } else {
            '░'
        }
    }

    fn draw_box(
        &self,
        f: &mut Frame,
        area: Rect,
        box_s: Style,
        title: &str,
        title_s: Style,
    ) {
        let iw = area.width.saturating_sub(2) as usize;
        // Top border
        f.render_widget(
            Paragraph::new(Span::styled(format!("┌{}┐", "─".repeat(iw)), box_s)),
            Rect::new(area.x, area.y, area.width, 1),
        );
        // Title
        if !title.is_empty() {
            let tx = area.x + (area.width.saturating_sub(title.len() as u16)) / 2;
            f.render_widget(
                Paragraph::new(Span::styled(title, title_s)),
                Rect::new(tx, area.y, title.len() as u16, 1),
            );
        }
        // Sides + fill
        for row in 1..area.height.saturating_sub(1) {
            let y = area.y + row;
            f.render_widget(
                Paragraph::new(Span::styled("│", box_s)),
                Rect::new(area.x, y, 1, 1),
            );
            f.render_widget(
                Paragraph::new(Span::styled(" ".repeat(iw), box_s)),
                Rect::new(area.x + 1, y, iw as u16, 1),
            );
            f.render_widget(
                Paragraph::new(Span::styled("│", box_s)),
                Rect::new(area.x + area.width - 1, y, 1, 1),
            );
        }
        // Bottom border
        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(iw)), box_s)),
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        );
    }

    // ── Help dialogs ─────────────────────────────────────────────────────────

    fn help_getting_started(&self, f: &mut Frame, scroll: usize) {
        let lines: &[&str] = &[
            " This section helps you start and use the MS-DOS Editor.",
            " Choose one of the following topics to see information on:",
            "",
            "   Using Help",
            "   Using Menus and Commands",
            "   Using a Dialog Box",
            "   MS-DOS Editor Options",
            "   Copyright and Trademarks",
        ];
        self.render_help_window(f, "HELP: Getting Started", lines, scroll);
    }

    fn help_keyboard(&self, f: &mut Frame, scroll: usize) {
        let lines: &[&str] = &[
            " This section is designed to help you navigate and edit your text while",
            " using the MS-DOS Editor.",
            "",
            " Choose a topic below to see information on:",
            "",
            "     Shortcut Keys           Text-Selection Keys",
            "     Help Keys               Insert and Copy Keys",
            "     Cursor-Movement Keys    Delete Keys",
            "     Text-Scrolling Keys     Find and Change Keys",
            "",
            " The MS-DOS Editor recognizes keystroke combinations familiar to users of",
            " other Microsoft programs (such as Microsoft Word) as well as WordStar key",
            " combinations. WordStar keystrokes that can be used in the MS-DOS Editor are",
            " listed in the right column of the three-column keystroke tables.",
            "",
            " Key                   Action",
            " ────────────────────────────────────────────────────────────",
            " Home                  Move to start of current line",
            " End                   Move to end of current line",
            " Ctrl+Home             Move to start of document",
            " Ctrl+End              Move to end of document",
            " PgUp                  Scroll up one screen",
            " PgDn                  Scroll down one screen",
            " Ctrl+Left             Move left one word",
            " Ctrl+Right            Move right one word",
            " Insert                Toggle insert/overwrite mode",
            " Delete                Delete character at cursor",
            " Backspace             Delete character to the left",
            " F3                    Repeat last find",
            " Shift+Del             Cut selected text",
            " Ctrl+Ins              Copy selected text",
            " Shift+Ins             Paste from clipboard",
            " Alt                   Activate menu bar",
        ];
        self.render_help_window(f, "HELP: Keyboard", lines, scroll);
    }

    fn render_help_window(&self, f: &mut Frame, title: &str, lines: &[&str], scroll: usize) {
        let area = f.area();
        // Help occupies top portion (above the editor bottom)
        let help_h = (area.height.saturating_sub(2)).min(14);
        let help_area = Rect::new(0, 1, area.width, help_h);
        f.render_widget(Clear, help_area);

        let box_s = Style::default().bg(Color::Blue).fg(Color::White);
        let title_s = Style::default().bg(Color::Blue).fg(Color::White);
        let scroll_s = Style::default().bg(Color::Blue).fg(Color::Gray);

        // Border
        let iw = help_area.width.saturating_sub(2) as usize;
        f.render_widget(
            Paragraph::new(Span::styled(format!("┌{}┐", "─".repeat(iw)), box_s)),
            Rect::new(help_area.x, help_area.y, help_area.width, 1),
        );
        let title_str = format!(" {} ", title);
        let tx = help_area.x + (help_area.width.saturating_sub(title_str.len() as u16)) / 2;
        f.render_widget(
            Paragraph::new(Span::styled(&title_str, title_s)),
            Rect::new(tx, help_area.y, title_str.len() as u16, 1),
        );

        // Nav tabs row
        let nav_row_y = help_area.y + 1;
        f.render_widget(
            Paragraph::new(Span::styled("│", box_s)),
            Rect::new(help_area.x, nav_row_y, 1, 1),
        );
        let nav = "  ◄Getting Started►  ◄Keyboard►  ◄Back►";
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{:<w$}", nav, w = iw),
                box_s,
            )),
            Rect::new(help_area.x + 1, nav_row_y, iw as u16, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled("│", box_s)),
            Rect::new(help_area.x + help_area.width - 1, nav_row_y, 1, 1),
        );

        // Separator
        let sep_y = help_area.y + 2;
        f.render_widget(
            Paragraph::new(Span::styled("│", box_s)),
            Rect::new(help_area.x, sep_y, 1, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled("─".repeat(iw), box_s)),
            Rect::new(help_area.x + 1, sep_y, iw as u16, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled("│", box_s)),
            Rect::new(help_area.x + help_area.width - 1, sep_y, 1, 1),
        );

        // Content rows
        let content_rows = help_h.saturating_sub(4) as usize; // -top border -nav -sep -bottom
        for row in 0..content_rows {
            let y = help_area.y + 3 + row as u16;
            let line_idx = scroll + row;
            let text = if line_idx < lines.len() { lines[line_idx] } else { "" };
            f.render_widget(
                Paragraph::new(Span::styled("│", box_s)),
                Rect::new(help_area.x, y, 1, 1),
            );
            let content_w = iw.saturating_sub(1);
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!("{:<w$}", text, w = content_w),
                    box_s,
                )),
                Rect::new(help_area.x + 1, y, content_w as u16, 1),
            );
            // Scrollbar
            let total = lines.len();
            let sb = if row == 0 {
                '↑'
            } else if row == content_rows - 1 {
                '↓'
            } else if total <= content_rows {
                ' '
            } else {
                let thumb = (scroll * (content_rows - 2)) / total.max(1);
                if row - 1 == thumb { ' ' } else { '░' }
            };
            f.render_widget(
                Paragraph::new(Span::styled(sb.to_string(), scroll_s)),
                Rect::new(help_area.x + help_area.width - 1, y, 1, 1),
            );
        }

        // Bottom border
        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(iw)), box_s)),
            Rect::new(
                help_area.x,
                help_area.y + help_h - 1,
                help_area.width,
                1,
            ),
        );

        // Status bar for help window
        let stat_y = area.height.saturating_sub(1);
        let stat_s = Style::default().bg(Color::Cyan).fg(Color::White);
        let stat_text = " <F1=Help>  <Esc=Cancel>";
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{:<w$}", stat_text, w = area.width as usize),
                stat_s,
            )),
            Rect::new(0, stat_y, area.width, 1),
        );
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
            Mode::DisplaySettings {
                fg_idx,
                bg_idx,
                fg_scroll,
                scroll_bars,
                tab_stops,
                focus,
            } => self.key_display_settings(
                key, fg_idx, bg_idx, fg_scroll, scroll_bars, tab_stops, focus,
            ),
            Mode::ConfirmNew => self.key_confirm_new(key),
            Mode::ConfirmExit => self.key_confirm_exit(key),
            Mode::About => {
                self.mode = Mode::Normal;
                false
            }
            Mode::HelpGettingStarted { scroll } => self.key_help(key, scroll, true),
            Mode::HelpKeyboard { scroll } => self.key_help(key, scroll, false),
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
                if let Some((idx, _)) = MENU_ITEMS[mi].iter().enumerate().find(|(_, it)| {
                    it.accel
                        .and_then(|pos| it.name.chars().nth(pos))
                        .map(|ch| ch.to_ascii_lowercase() == needle)
                        .unwrap_or(false)
                }) {
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
            (3, 0) => {
                let fg_idx = v1_fg_index(self.settings.colors.editor_fg);
                let bg_idx = v1_bg_index(self.settings.colors.editor_bg);
                let fg_scroll = fg_idx.saturating_sub(7).min(V1_FG_COLORS.len().saturating_sub(8));
                self.mode = Mode::DisplaySettings {
                    fg_idx,
                    bg_idx,
                    fg_scroll,
                    scroll_bars: self.settings.scroll_bars,
                    tab_stops: self.settings.tab_stops,
                    focus: 0,
                };
            }
            (3, 1) => {} // Help Path... not implemented
            (4, 0) => {
                self.mode = Mode::HelpGettingStarted { scroll: 0 };
            }
            (4, 1) => {
                self.mode = Mode::HelpKeyboard { scroll: 0 };
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

    fn key_display_settings(
        &mut self,
        key: KeyEvent,
        mut fg_idx: usize,
        mut bg_idx: usize,
        mut fg_scroll: usize,
        mut scroll_bars: bool,
        mut tab_stops: u8,
        mut focus: usize,
    ) -> bool {
        const FOCUS_COUNT: usize = 7; // fg, bg, scroll_bars, tab_stops, ok, cancel, help
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return false;
            }
            KeyCode::Tab => {
                focus = (focus + 1) % FOCUS_COUNT;
            }
            KeyCode::BackTab => {
                focus = if focus == 0 { FOCUS_COUNT - 1 } else { focus - 1 };
            }
            KeyCode::Up => match focus {
                0 => {
                    if fg_idx > 0 {
                        fg_idx -= 1;
                        if fg_idx < fg_scroll {
                            fg_scroll = fg_idx;
                        }
                    }
                }
                1 => {
                    if bg_idx > 0 {
                        bg_idx -= 1;
                    }
                }
                _ => {
                    focus = if focus == 0 { FOCUS_COUNT - 1 } else { focus - 1 };
                }
            },
            KeyCode::Down => match focus {
                0 => {
                    if fg_idx + 1 < V1_FG_COLORS.len() {
                        fg_idx += 1;
                        let max_scroll = V1_FG_COLORS.len().saturating_sub(8);
                        if fg_idx >= fg_scroll + 8 {
                            fg_scroll = (fg_idx.saturating_sub(7)).min(max_scroll);
                        }
                    }
                }
                1 => {
                    if bg_idx + 1 < V1_BG_COLORS.len() {
                        bg_idx += 1;
                    }
                }
                _ => {
                    focus = (focus + 1) % FOCUS_COUNT;
                }
            },
            KeyCode::Char(' ') if focus == 2 => {
                scroll_bars = !scroll_bars;
            }
            KeyCode::Left if focus == 3 => {
                tab_stops = tab_stops.saturating_sub(1).max(1);
            }
            KeyCode::Right if focus == 3 => {
                tab_stops = tab_stops.saturating_add(1).min(40);
            }
            KeyCode::Char(c) if focus == 3 && c.is_ascii_digit() => {
                let n = (tab_stops / 10) * 10 + (c as u8 - b'0');
                tab_stops = n.max(1).min(40);
            }
            KeyCode::Enter => {
                match focus {
                    5 => {
                        // Cancel
                        self.mode = Mode::Normal;
                        return false;
                    }
                    6 => {
                        // Help
                        self.mode = Mode::HelpGettingStarted { scroll: 0 };
                        return false;
                    }
                    _ => {
                        // OK (or Enter in a field advances focus unless on ok)
                        if focus == 4 || focus == 0 || focus == 1 {
                            self.apply_display_settings(
                                fg_idx,
                                bg_idx,
                                scroll_bars,
                                tab_stops,
                            );
                            self.mode = Mode::Normal;
                            return false;
                        }
                        focus = (focus + 1) % FOCUS_COUNT;
                    }
                }
            }
            _ => {}
        }
        self.mode = Mode::DisplaySettings {
            fg_idx,
            bg_idx,
            fg_scroll,
            scroll_bars,
            tab_stops,
            focus,
        };
        false
    }

    fn apply_display_settings(
        &mut self,
        fg_idx: usize,
        bg_idx: usize,
        scroll_bars: bool,
        tab_stops: u8,
    ) {
        self.settings.colors.editor_fg = V1_FG_COLORS[fg_idx.min(V1_FG_COLORS.len() - 1)].1;
        self.settings.colors.editor_bg = V1_BG_COLORS[bg_idx.min(V1_BG_COLORS.len() - 1)].1;
        self.settings.scroll_bars = scroll_bars;
        self.settings.tab_stops = tab_stops;
        self.theme = self.settings.theme();
        match self.settings.save() {
            Ok(()) => self.message = Some("Settings saved".to_string()),
            Err(e) => self.message = Some(format!("Error saving settings: {}", e)),
        }
    }

    fn key_help(&mut self, key: KeyEvent, mut scroll: usize, is_getting_started: bool) -> bool {
        let max_lines: usize = if is_getting_started { 8 } else { 33 };
        let visible = 9usize;
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
            }
            KeyCode::Up => {
                scroll = scroll.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::PageDown => {
                scroll = (scroll + 1).min(max_lines.saturating_sub(visible));
            }
            KeyCode::PageUp => {
                scroll = scroll.saturating_sub(visible);
            }
            KeyCode::Tab => {
                // Switch between Getting Started and Keyboard
                if is_getting_started {
                    self.mode = Mode::HelpKeyboard { scroll: 0 };
                } else {
                    self.mode = Mode::HelpGettingStarted { scroll: 0 };
                }
                return false;
            }
            _ => {}
        }
        if is_getting_started {
            self.mode = Mode::HelpGettingStarted { scroll };
        } else {
            self.mode = Mode::HelpKeyboard { scroll };
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
