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

mod term;
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
    /// --faithful: disable all redit-specific enhancements (syntax highlight, etc.)
    /// so behavior matches the original MS-DOS EDIT as closely as possible.
    pub faithful: bool,
    // F12 toggles keyboard debug mode; stores the last key description
    pub(super) kbd_debug: Option<String>,
    // Embedded terminal pane (Ctrl+T, disabled in --faithful mode)
    pub(super) term_pane: Option<term::TermPane>,
}

impl App {
    pub fn new() -> Self {
        let mut editor = Editor::new();
        let args: Vec<String> = std::env::args().collect();
        let mut settings = UserSettings::load();
        let mut filename = None;
        let mut faithful = false;
        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--v1" => settings.set_style(Version::V1),
                "--v2" => settings.set_style(Version::V2),
                "--faithful" => faithful = true,
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
            faithful,
            kbd_debug: None,
            term_pane: None,
        }
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        loop {
            // Drain PTY output; close pane if the shell exited
            if let Some(tp) = &mut self.term_pane {
                tp.drain();
            }
            if self.term_pane.as_ref().map(|t| t.closed).unwrap_or(false) {
                self.term_pane = None;
            }

            terminal.draw(|f| self.render(f))?;

            if !event::poll(std::time::Duration::from_millis(50))? {
                continue;
            }

            match event::read()? {
                Event::Key(k) => {
                    // When terminal pane is focused, forward all input to the PTY
                    // except editor-level shortcuts which must remain available.
                    if self.term_pane.as_ref().map(|t| t.focused).unwrap_or(false) {
                        let ctrl = k.modifiers == KeyModifiers::CONTROL;
                        let alt_menu = matches!(
                            k.code,
                            KeyCode::Modifier(ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt)
                                | KeyCode::Char('f' | 'F' | 'e' | 'E' | 's' | 'S' | 'o' | 'O' | 'h' | 'H')
                                if k.modifiers.contains(KeyModifiers::ALT)
                                    || matches!(k.code, KeyCode::Modifier(_))
                        );
                        if alt_menu {
                            if let Some(tp) = &mut self.term_pane {
                                tp.focused = false;
                            }
                            if self.handle_key(k) {
                                break;
                            }
                        } else if k.code == KeyCode::Char('t') && ctrl {
                            if let Some(tp) = &mut self.term_pane {
                                tp.focused = false;
                            }
                        } else if k.code == KeyCode::Up && ctrl {
                            if let Some(tp) = &mut self.term_pane {
                                let new_h = (tp.height + 1).min(20);
                                tp.resize(tp.width, new_h);
                            }
                        } else if k.code == KeyCode::Down && ctrl {
                            if let Some(tp) = &mut self.term_pane {
                                let new_h = tp.height.saturating_sub(1).max(3);
                                tp.resize(tp.width, new_h);
                            }
                        } else {
                            self.forward_key_to_term(k);
                        }
                        continue;
                    }

                    if self.handle_key(k) {
                        break;
                    }
                }
                Event::Paste(text) => {
                    if self.term_pane.as_ref().map(|t| t.focused).unwrap_or(false) {
                        if let Some(tp) = &mut self.term_pane {
                            tp.write_input(text.as_bytes());
                        }
                    } else {
                        self.handle_paste(&text);
                    }
                }
                Event::Mouse(m) => self.handle_mouse(m),
                Event::Resize(w, h) => {
                    self.page_height = h.saturating_sub(4) as usize;
                    // Resize terminal pane to new width
                    if let Some(tp) = &mut self.term_pane {
                        tp.resize(w, tp.height);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_paste(&mut self, text: &str) {
        match &mut self.mode {
            Mode::Normal => {
                self.editor.delete_selection();
                self.editor.paste_text(text);
            }
            Mode::Open(s) | Mode::SaveAs(s) | Mode::Find(s) | Mode::Goto(s) => {
                s.push_str(&text.replace(['\r', '\n'], ""));
            }
            Mode::Replace { find, replace, focus } => {
                let text = text.replace(['\r', '\n'], "");
                if *focus == 0 {
                    find.push_str(&text);
                } else {
                    replace.push_str(&text);
                }
            }
            _ => {}
        }
    }

    fn forward_key_to_term(&mut self, key: KeyEvent) {
        let Some(tp) = &mut self.term_pane else { return };
        let bytes: Vec<u8> = match key.code {
            KeyCode::Char(c) if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let byte = (c as u8).to_ascii_lowercase();
                if byte >= b'a' && byte <= b'z' {
                    vec![byte - b'a' + 1]
                } else {
                    c.to_string().into_bytes()
                }
            }
            KeyCode::Char(c) => c.to_string().into_bytes(),
            KeyCode::Enter => vec![b'\r'],
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Tab => vec![b'\t'],
            KeyCode::Esc => vec![0x1b],
            KeyCode::Up => b"\x1b[A".to_vec(),
            KeyCode::Down => b"\x1b[B".to_vec(),
            KeyCode::Right => b"\x1b[C".to_vec(),
            KeyCode::Left => b"\x1b[D".to_vec(),
            KeyCode::Home => b"\x1b[H".to_vec(),
            KeyCode::End => b"\x1b[F".to_vec(),
            KeyCode::Delete => b"\x1b[3~".to_vec(),
            KeyCode::F(n) => format!("\x1b[{}~", 10 + n).into_bytes(),
            _ => return,
        };
        tp.write_input(&bytes);
    }

    // ── Render ────────────────────────────────────────────────────────────────

    fn render(&mut self, f: &mut Frame) {
        let size = f.area();
        let menu_area = Rect::new(0, 0, size.width, 1);
        let stat_area = Rect::new(0, size.height.saturating_sub(1), size.width, 1);

        // Reserve rows at the bottom for the terminal pane (separator + content)
        let term_h = self.term_pane.as_ref().map(|tp| tp.height + 1).unwrap_or(0);
        let edit_h = size.height.saturating_sub(2).saturating_sub(term_h);
        let edit_area = Rect::new(0, 1, size.width, edit_h);

        self.render_editor(f, edit_area);
        self.render_menu_bar(f, menu_area);
        self.render_status_bar(f, stat_area);

        // Render terminal pane if open
        if self.term_pane.is_some() {
            let term_area = Rect::new(0, 1 + edit_h, size.width, term_h);
            let border_style = Style::default().bg(self.theme.frame_bg).fg(self.theme.frame_fg);
            let title_style  = Style::default().bg(self.theme.title_bg).fg(self.theme.title_fg);
            let inner_bg     = Style::default().bg(self.theme.edit_bg).fg(self.theme.edit_fg);
            let tp = self.term_pane.as_mut().unwrap();
            tp.render(f, term_area, border_style, title_style, inner_bg);

            // Place the real blinking cursor at the terminal's cursor position
            if tp.focused {
                let (cur_row, cur_col) = tp.parser.screen().cursor_position();
                // +1 for left │ border, +1 for separator row
                f.set_cursor_position((
                    term_area.x + 1 + cur_col,
                    term_area.y + 1 + cur_row,
                ));
            }
        }

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

        Self::render_shadow(f, area);
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

    pub(super) fn render_text_row(
        f: &mut Frame,
        area: Rect,
        chars: &[char],
        sx: usize,
        cx: usize,
        cursor_here: bool,
        base: Style,
        cur_style: Style,
        sel: Option<(usize, usize)>, // (sel_start, sel_end) in char-index doc coords
    ) {
        let vw = area.width as usize;
        let tab_stop = 8;

        // Compute raw_col for correct tab-stop positions starting at sx
        let mut raw_col = 0usize;
        for &c in chars.iter().take(sx) {
            raw_col += Self::char_display_width(c, raw_col, tab_stop);
        }

        // Build spans char by char, merging consecutive same-style runs
        let mut out: Vec<(String, Style)> = Vec::new();
        let mut screen_col = 0usize;
        let mut cursor_rendered = false;

        let style_at = |i: usize| -> Style {
            if cursor_here && i == cx {
                cur_style
            } else if sel.map(|(s, e)| i >= s && i < e).unwrap_or(false) {
                cur_style
            } else {
                base
            }
        };

        for (i, &c) in chars.iter().enumerate().skip(sx) {
            let w = Self::char_display_width(c, raw_col, tab_stop);
            if screen_col + w > vw {
                break;
            }
            if cursor_here && i == cx {
                cursor_rendered = true;
            }
            let text = if c == '\t' { " ".repeat(w) } else { c.to_string() };
            let sty = style_at(i);
            match out.last_mut() {
                Some(last) if last.1 == sty => last.0.push_str(&text),
                _ => out.push((text, sty)),
            }
            screen_col += w;
            raw_col += w;
        }

        // Cursor past end of content
        if cursor_here && !cursor_rendered && screen_col < vw {
            out.push((" ".to_string(), cur_style));
            screen_col += 1;
        }

        // Pad remainder with base style
        if screen_col < vw {
            let pad = " ".repeat(vw - screen_col);
            match out.last_mut() {
                Some(last) if last.1 == base => last.0.push_str(&pad),
                _ => out.push((pad, base)),
            }
        }

        if out.is_empty() {
            f.render_widget(Paragraph::new(Span::styled(" ".repeat(vw), base)), area);
        } else {
            let spans: Vec<Span> = out.into_iter().map(|(s, st)| Span::styled(s, st)).collect();
            f.render_widget(Paragraph::new(Line::from(spans)), area);
        }
    }

    /// Like `render_text_row` but uses per-character colors from syntect highlighting.
    /// Keeps the editor's `edit_bg` as background; only overrides foreground.
    pub(super) fn render_highlighted_row(
        f: &mut Frame,
        area: Rect,
        hl_line: &ratatui::text::Line<'static>,
        sx: usize,
        cx: usize,
        cursor_here: bool,
        edit_bg: Color,
        cur_style: Style,
        tab_stop: usize,
        sel: Option<(usize, usize)>, // (sel_start, sel_end) in char-index doc coords
    ) {
        let vw = area.width as usize;

        // Flatten highlighted spans into (char, Style) pairs, forcing edit_bg
        let mut char_styles: Vec<(char, Style)> = Vec::new();
        for span in &hl_line.spans {
            let s = span.style.bg(edit_bg);
            for c in span.content.chars() {
                char_styles.push((c, s));
            }
        }

        // Compute display column at sx (needed for correct tab expansion)
        let mut raw_col = 0usize;
        for &(c, _) in char_styles.iter().take(sx) {
            raw_col += Self::char_display_width(c, raw_col, tab_stop);
        }

        let mut out: Vec<Span<'static>> = Vec::new();
        let mut screen_col = 0usize;

        for (i, &(c, style)) in char_styles.iter().enumerate().skip(sx) {
            let w = Self::char_display_width(c, raw_col, tab_stop);
            if screen_col + w > vw {
                break;
            }
            let is_sel = sel.map(|(s, e)| i >= s && i < e).unwrap_or(false);
            let eff = if cursor_here && i == cx || is_sel { cur_style } else { style };
            if c == '\t' {
                out.push(Span::styled(" ".repeat(w), eff));
            } else {
                out.push(Span::styled(c.to_string(), eff));
            }
            screen_col += w;
            raw_col += w;
        }

        // Cursor past end of content
        if cursor_here && cx >= char_styles.len() && screen_col < vw {
            out.push(Span::styled(" ".to_string(), cur_style));
            screen_col += 1;
        }

        // Pad remainder with plain editor background
        if screen_col < vw {
            out.push(Span::styled(
                " ".repeat(vw - screen_col),
                Style::default().bg(edit_bg),
            ));
        }

        f.render_widget(Paragraph::new(Line::from(out)), area);
    }

    // False when the terminal pane is focused — suppresses the editor software cursor.
    pub(super) fn editor_cursor_active(&self) -> bool {
        !self.term_pane.as_ref().map(|t| t.focused).unwrap_or(false)
    }

    // Returns the selected char-index range for a given line, if any.
    pub(super) fn line_selection(&self, line_idx: usize) -> Option<(usize, usize)> {
        if !self.editor.has_selection() {
            return None;
        }
        let ((sx, sy), (ex, ey)) = self.editor.selection_range()?;
        if line_idx < sy || line_idx > ey {
            return None;
        }
        let start = if line_idx == sy { sx } else { 0 };
        let end = if line_idx == ey {
            ex
        } else {
            self.editor.lines[line_idx].chars().count()
        };
        if start >= end { None } else { Some((start, end)) }
    }

    // ── Status bar ────────────────────────────────────────────────────────────

    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let (cx, cy) = self.editor.cursor;
        let ovr = if self.editor.overtype { "OVR" } else { "   " };
        let fname = self.editor.filename.as_deref().unwrap_or("Untitled");
        let w = area.width as usize;

        if self.theme.version == Version::V1 {
            let left = if let Some(dbg) = &self.kbd_debug {
                format!(" {}", dbg)
            } else if self.mode == Mode::Welcome {
                " F1=Help   Enter=Execute   Esc=Cancel   Tab=Next Field   Arrow=Next Item"
                    .to_string()
            } else if matches!(self.mode, Mode::ConfirmNew | Mode::ConfirmExit) {
                " F1=Help   Enter=Execute   Esc=Cancel   Tab=Next Field   Arrow=Next Item"
                    .to_string()
            } else if matches!(self.mode, Mode::Open(_) | Mode::SaveAs(_)) {
                " F1=Help   Enter=Execute   Esc=Cancel   Tab=Next Field   Arrow=Next Item"
                    .to_string()
            } else if let Mode::Menu { menu, item } = self.mode {
                let help = MENU_ITEMS[menu][item].help;
                format!(" F1=Help │ {}", help)
            } else if self.term_pane.as_ref().map(|t| t.focused).unwrap_or(false) {
                " Ctrl+T=Unfocus   Ctrl+Up/Dn=Resize".to_string()
            } else if self.term_pane.is_some() {
                " Ctrl+T=Focus Terminal".to_string()
            } else if let Some(m) = &self.message {
                format!(" {}", m)
            } else {
                " MS-DOS Editor  <F1=Help> Press ALT to activate menus".to_string()
            };
            if matches!(
                self.mode,
                Mode::Welcome | Mode::ConfirmNew | Mode::ConfirmExit | Mode::Open(_) | Mode::SaveAs(_)
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

        let left = if let Some(dbg) = &self.kbd_debug {
            format!(" {}", dbg)
        } else if self.mode == Mode::Welcome {
            format!(" F1=Help   Enter=Execute   Esc=Cancel")
        } else if matches!(self.mode, Mode::Open(_) | Mode::SaveAs(_)) {
            format!(" F1=Help   Enter=Execute   Esc=Cancel   Tab=Next Field   Arrow=Next Item")
        } else if self.term_pane.as_ref().map(|t| t.focused).unwrap_or(false) {
            format!(" Ctrl+T=Unfocus   Ctrl+Up/Dn=Resize")
        } else if self.term_pane.is_some() {
            format!(" Ctrl+T=Focus Terminal")
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
            Mode::Open(s) => self.open_dialog(f, s),
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

    /// Darken cells in the DOS-style drop-shadow area of `area` (2 cols right + 1 row below).
    /// Uses buffer_mut so the underlying characters are preserved with a dark palette.
    pub(super) fn render_shadow(f: &mut Frame, area: Rect) {
        let size = f.area();
        let buf = f.buffer_mut();
        // Right strip: 2 cols wide, starting 1 row down (skip top row of dialog)
        for dy in 1..area.height {
            for dx in 0u16..2 {
                let x = area.x.saturating_add(area.width).saturating_add(dx);
                let y = area.y.saturating_add(dy);
                if x < size.width && y < size.height {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_fg(Color::DarkGray).set_bg(Color::Black);
                    }
                }
            }
        }
        // Bottom strip: dialog.width cols from x+2, 1 row below dialog
        let y = area.y.saturating_add(area.height);
        if y < size.height {
            for dx in 2u16..area.width.saturating_add(2) {
                let x = area.x.saturating_add(dx);
                if x < size.width {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_fg(Color::DarkGray).set_bg(Color::Black);
                    }
                }
            }
        }
    }

    fn input_dialog(&self, f: &mut Frame, title: &str, label: &str, input: &str) {
        let t = &self.theme;
        let area = Self::center_rect(f, 52, 7);
        Self::render_shadow(f, area);
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

    fn open_dialog(&self, f: &mut Frame, input: &str) {
        if self.theme.version != Version::V1 {
            self.input_dialog(f, "Open", "File Name:", input);
            return;
        }

        let area = Self::center_rect(f, 69, 20);
        Self::render_shadow(f, area);
        f.render_widget(Clear, area);

        let bs = Style::default().bg(Color::Gray).fg(Color::Black);
        let ts = Style::default().bg(Color::White).fg(Color::Black);
        let acs = Style::default().bg(Color::Gray).fg(Color::White);
        let input_s = Style::default().bg(Color::White).fg(Color::Black);
        let scroll_s = Style::default().bg(Color::Gray).fg(Color::Black);

        let x = area.x;
        let y = area.y;
        let iw = area.width.saturating_sub(2) as usize;

        let title = " Open ";
        let dashes = iw.saturating_sub(title.len());
        let ld = dashes / 2;
        let rd = dashes - ld;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("┌", bs),
                Span::styled("─".repeat(ld), bs),
                Span::styled(title, ts),
                Span::styled("─".repeat(rd), bs),
                Span::styled("┐", bs),
            ])),
            Rect::new(x, y, area.width, 1),
        );

        for row in 1u16..area.height.saturating_sub(1) {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("│", bs),
                    Span::styled(" ".repeat(iw), bs),
                    Span::styled("│", bs),
                ])),
                Rect::new(x, y + row, area.width, 1),
            );
        }

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("            ┌{}┐ ", "─".repeat(50)),
                bs,
            )),
            Rect::new(x + 1, y + 1, iw as u16, 1),
        );

        let field = Self::fit_text(input, 50);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" File ", bs),
                Span::styled("N", acs),
                Span::styled("ame: │", bs),
                Span::styled(format!("{:<50}", field), input_s),
                Span::styled("│ ", bs),
            ])),
            Rect::new(x + 1, y + 2, iw as u16, 1),
        );

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("            └{}┘ ", "─".repeat(50)),
                bs,
            )),
            Rect::new(x + 1, y + 3, iw as u16, 1),
        );

        let cwd = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        f.render_widget(
            Paragraph::new(Span::styled(format!(" {:<66}", Self::fit_text(&cwd, 66)), bs)),
            Rect::new(x + 1, y + 4, iw as u16, 1),
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{:^45}", "Files"), bs),
                Span::styled(" ", bs),
                Span::styled(format!("{:^14}", "Dirs/Drives"), bs),
            ])),
            Rect::new(x + 1, y + 5, iw as u16, 1),
        );

        let files_x = x + 2;
        let dirs_x = x + 49;
        f.render_widget(
            Paragraph::new(Span::styled(format!("┌{}┐", "─".repeat(43)), bs)),
            Rect::new(files_x, y + 6, 45, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled(format!("┌{}┐", "─".repeat(14)), bs)),
            Rect::new(dirs_x, y + 6, 16, 1),
        );

        let (files, dirs) = Self::dir_listing();
        for i in 0u16..8 {
            let file = files.get(i as usize).map(String::as_str).unwrap_or("");
            f.render_widget(
                Paragraph::new(Span::styled(format!("│ {:<42}│", Self::fit_text(file, 42)), bs)),
                Rect::new(files_x, y + 7 + i, 45, 1),
            );

            let dir = dirs.get(i as usize).map(String::as_str).unwrap_or("");
            let sc = if i == 0 {
                "↑"
            } else if i == 7 {
                "↓"
            } else {
                "░"
            };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("│ ", bs),
                    Span::styled(format!("{:<11}", Self::fit_text(dir, 11)), bs),
                    Span::styled(sc, scroll_s),
                    Span::styled(" │", bs),
                ])),
                Rect::new(dirs_x, y + 7 + i, 16, 1),
            );
        }

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("└", bs),
                Span::styled("←", bs),
                Span::styled("░".repeat(41), scroll_s),
                Span::styled("→", bs),
                Span::styled("┘", bs),
            ])),
            Rect::new(files_x, y + 15, 45, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(14)), bs)),
            Rect::new(dirs_x, y + 15, 16, 1),
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("├{}┤", "─".repeat(iw)), bs)),
            Rect::new(x, y + 17, area.width, 1),
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("           ", bs),
                Span::styled("<", acs),
                Span::styled(" OK ", bs),
                Span::styled(">", acs),
                Span::styled("          ", bs),
                Span::styled("<", acs),
                Span::styled(" Cancel ", bs),
                Span::styled(">", acs),
                Span::styled("          ", bs),
                Span::styled("<", acs),
                Span::styled(" Help ", bs),
                Span::styled(">", acs),
                Span::styled("          ", bs),
            ])),
            Rect::new(x + 1, y + 18, iw as u16, 1),
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(iw)), bs)),
            Rect::new(x, y + 19, area.width, 1),
        );
    }

    fn save_as_dialog(&self, f: &mut Frame, input: &str) {
        // Compact centered modal matching original MS-DOS EDIT Save As dialog:
        //   42 wide × 18 tall: file-name field box + Dirs/Drives list + OK/Cancel/Help
        let area = Self::center_rect(f, 42, 18);
        let size = f.area();
        Self::render_shadow(f, area);
        f.render_widget(Clear, area);

        let bs = Style::default().bg(Color::Gray).fg(Color::Black);
        let ts = Style::default().bg(Color::White).fg(Color::Black);
        let acs = Style::default().bg(Color::Gray).fg(Color::White);

        let x = area.x;
        let y = area.y;

        // ── Title row ─────────────────────────────────────────────────────
        let title = " Save As ";
        let dashes = 40usize.saturating_sub(title.len());
        let ld = dashes / 2;
        let rd = dashes - ld;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("┌", bs),
                Span::styled("─".repeat(ld), bs),
                Span::styled(title, ts),
                Span::styled("─".repeat(rd), bs),
                Span::styled("┐", bs),
            ])),
            Rect::new(x, y, 42, 1),
        );

        // ── Interior rows (fill with box_style blank) ──────────────────────
        for row in 1u16..17 {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("│", bs),
                    Span::styled(" ".repeat(40), bs),
                    Span::styled("│", bs),
                ])),
                Rect::new(x, y + row, 42, 1),
            );
        }

        // ── File Name field (box-within-box, inner width = 25) ─────────────
        // Row 1: "            ┌─────────────────────────┐ "
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("            ┌{}┐ ", "─".repeat(25)),
                bs,
            )),
            Rect::new(x + 1, y + 1, 40, 1),
        );
        // Row 2: " File Name: │<input>│ "
        let field_content = format!("{:<25}", &input[..input.len().min(25)]);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" File ", bs),
                Span::styled("N", acs),
                Span::styled("ame: │", bs),
                Span::styled(field_content, bs),
                Span::styled("│ ", bs),
            ])),
            Rect::new(x + 1, y + 2, 40, 1),
        );
        // Row 3: "            └─────────────────────────┘ "
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("            └{}┘ ", "─".repeat(25)),
                bs,
            )),
            Rect::new(x + 1, y + 3, 40, 1),
        );

        // ── Current directory ──────────────────────────────────────────────
        let cwd = env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| ".".to_string());
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(" {:<39}", Self::fit_text(&cwd, 39)),
                bs,
            )),
            Rect::new(x + 1, y + 4, 40, 1),
        );

        // ── "Dirs/Drives" centered header ──────────────────────────────────
        f.render_widget(
            Paragraph::new(Span::styled(format!("{:^40}", "Dirs/Drives"), bs)),
            Rect::new(x + 1, y + 5, 40, 1),
        );

        // ── Dirs/Drives list box (16 wide, centered at offset +12) ─────────
        let lx = x + 13; // 1 (border) + 12 (pad)
        f.render_widget(
            Paragraph::new(Span::styled(format!("┌{}┐", "─".repeat(14)), bs)),
            Rect::new(lx, y + 6, 16, 1),
        );
        let (_, dirs) = Self::dir_listing();
        for i in 0u16..7 {
            let entry = dirs.get(i as usize).map(String::as_str).unwrap_or("");
            f.render_widget(
                Paragraph::new(Span::styled(
                    format!("│ {:<13}│", Self::fit_text(entry, 13)),
                    bs,
                )),
                Rect::new(lx, y + 7 + i, 16, 1),
            );
        }
        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(14)), bs)),
            Rect::new(lx, y + 14, 16, 1),
        );

        // ── Separator ─────────────────────────────────────────────────────
        f.render_widget(
            Paragraph::new(Span::styled(format!("├{}┤", "─".repeat(40)), bs)),
            Rect::new(x, y + 15, 42, 1),
        );

        // ── Buttons row ────────────────────────────────────────────────────
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("      ", bs),
                Span::styled("< OK >", ts),
                Span::styled("    ", bs),
                Span::styled("< Cancel >", bs),
                Span::styled("    ", bs),
                Span::styled("< Help >", bs),
                Span::styled("  ", bs),
            ])),
            Rect::new(x + 1, y + 16, 40, 1),
        );

        // ── Bottom border ──────────────────────────────────────────────────
        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(40)), bs)),
            Rect::new(x, y + 17, 42, 1),
        );

        // ── Status bar ────────────────────────────────────────────────────
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "{:<w$}",
                    "F1=Help  Enter=Execute  Esc=Cancel  Tab=Next Field",
                    w = size.width as usize
                ),
                Style::default().bg(self.theme.stat_bg).fg(self.theme.stat_fg),
            )),
            Rect::new(0, size.height.saturating_sub(1), size.width, 1),
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
        Self::render_shadow(f, area);
        f.render_widget(Clear, area);

        let box_style = Style::default()
            .bg(ratatui::style::Color::Gray)
            .fg(ratatui::style::Color::Black);
        let hilite_style = Style::default()
            .bg(ratatui::style::Color::Gray)
            .fg(ratatui::style::Color::White);
        let iw = area.width as usize - 2;

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
        Self::render_shadow(f, area);
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
        Self::render_shadow(f, area);
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
        Self::render_shadow(f, area);
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
        Self::render_shadow(f, area);
        f.render_widget(Clear, area);

        let box_s = Style::default().bg(Color::Gray).fg(Color::Black);
        let title_s = Style::default().bg(Color::White).fg(Color::Black);
        let sel_s = Style::default().bg(Color::Black).fg(Color::White);
        let inner_w = area.width.saturating_sub(2) as usize;

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
            "Using the MS-DOS Editor:",
            "",
            "  \u{25a0} To activate the MS-DOS Editor menu bar, press Alt.",
            "  \u{25a0} To activate menus and commands, press the highlighted letter.",
            "  \u{25a0} To move between menus and commands, use the direction keys.",
            "  \u{25a0} To get help on a selected menu, command, or dialog box, press F1.",
            "  \u{25a0} To exit Help, press Esc.",
            "",
            "Browsing the MS-DOS Editor Help system:",
            "",
            "  \u{25a0} To select one of the following topics, press the Tab key or the first",
            "    letter of the topic. Then press the Enter key to see information on:",
            "",
            "    \u{25c4}Getting Started\u{25ba}  Loading and using the MS-DOS Editor and the",
            "                       MS-DOS Editor Help system",
            "    \u{25c4}Keyboard\u{25ba}         Editing and navigating text and MS-DOS Editor Help",
            "",
            "Tip: These topics are also available from the Help menu.",
        ];
        self.render_help_window(f, "HELP: Survival Guide", lines, scroll);
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
        // Help window: full-width, from row 2 to near-bottom, matching original MS-DOS EDIT.
        // Leaves 2 rows below for the editor frame title + one content row.
        let help_h = area.height.saturating_sub(3); // rows 1..(h-2)
        let help_area = Rect::new(0, 1, area.width, help_h);
        f.render_widget(Clear, help_area);

        // Color palette matching original:
        //   [37m][40m] = Gray fg on Black bg  (body text, borders)
        //   [30m][47m] = Black fg on Gray bg  (title, scroll indicators)
        //   [97m][40m] = White fg on Black bg (section headings — bright white)
        //   [92m][40m] = LightGreen fg on Black bg (hyperlink brackets ◄ ►)
        let body_s   = Style::default().bg(Color::Black).fg(Color::Gray);
        let title_s  = Style::default().bg(Color::Gray).fg(Color::Black);
        let scroll_s = Style::default().bg(Color::Gray).fg(Color::Black);
        let link_s   = Style::default().bg(Color::Black).fg(Color::LightGreen);

        let iw = help_area.width.saturating_sub(2) as usize;

        // ── Top border with centered title ─────────────────────────────────
        let title_str = format!(" {} ", title);
        let dashes = iw.saturating_sub(title_str.len());
        let ld = dashes / 2;
        let rd = dashes - ld;
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("┌", body_s),
                Span::styled("─".repeat(ld), body_s),
                Span::styled(&title_str, title_s),
                Span::styled("─".repeat(rd), body_s),
                Span::styled("┐", body_s),
            ])),
            Rect::new(help_area.x, help_area.y, help_area.width, 1),
        );

        // ── Content rows ────────────────────────────────────────────────────
        let content_rows = help_h.saturating_sub(2) as usize; // exclude top+bottom borders
        let total = lines.len();
        for row in 0..content_rows {
            let y = help_area.y + 1 + row as u16;
            let line_idx = scroll + row;
            let text = if line_idx < total { lines[line_idx] } else { "" };
            let content_w = iw; // full inner width; scroll char overlays last col

            // Left border
            f.render_widget(
                Paragraph::new(Span::styled("│", body_s)),
                Rect::new(help_area.x, y, 1, 1),
            );

            // Body line — render with heading/link highlighting
            let padded = format!("{:<w$}", text, w = content_w);
            let spans = Self::help_line_spans(&padded, body_s, link_s);
            f.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(help_area.x + 1, y, content_w as u16, 1),
            );

            // Scrollbar
            let sc = if row == 0 {
                '↑'
            } else if row == content_rows - 1 {
                '↓'
            } else if total <= content_rows {
                ' '
            } else {
                let thumb = (scroll * (content_rows - 2)) / total.max(1);
                if row.saturating_sub(1) == thumb { ' ' } else { '░' }
            };
            f.render_widget(
                Paragraph::new(Span::styled(sc.to_string(), scroll_s)),
                Rect::new(help_area.x + help_area.width - 1, y, 1, 1),
            );
        }

        // ── Bottom border (shares row with editor title using ├ ┤) ──────────
        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(iw)), body_s)),
            Rect::new(help_area.x, help_area.y + help_h - 1, help_area.width, 1),
        );

        // ── Status bar ────────────────────────────────────────────────────
        let stat_s = Style::default().bg(Color::Cyan).fg(Color::Black);
        let stat_text = " <F1=Help> <F6=Window> <Esc=Cancel> <Ctrl+F1=Next> <Alt+F1=Back>";
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{:<w$}", stat_text, w = area.width as usize),
                stat_s,
            )),
            Rect::new(0, area.height.saturating_sub(1), area.width, 1),
        );
    }

    /// Parse a help line and return spans with ◄ ► in link_s and rest in body_s.
    fn help_line_spans<'a>(
        text: &'a str,
        body_s: Style,
        link_s: Style,
    ) -> Vec<Span<'a>> {
        // Check if text has heading marker (starts with non-space after leading spaces,
        // and uses the Bright-White heading style). Headings in the original are lines
        // like "Using the MS-DOS Editor:" — we detect them as lines not starting with
        // spaces that aren't borders.
        let heading_s = Style::default().bg(Color::Black).fg(Color::White);
        let is_heading = !text.starts_with(' ')
            && !text.starts_with('│')
            && !text.starts_with('─')
            && !text.starts_with(' ')
            && !text.trim().is_empty()
            && !text.contains('■')
            && !text.contains('◄');

        if !text.contains('◄') && !text.contains('►') {
            let s = if is_heading { heading_s } else { body_s };
            return vec![Span::styled(text.to_string(), s)];
        }
        // Split on ◄ and ► to colorize link brackets
        let mut spans = Vec::new();
        let mut remaining = text;
        loop {
            if let Some(pos) = remaining.find(['◄', '►']) {
                if pos > 0 {
                    spans.push(Span::styled(remaining[..pos].to_string(), body_s));
                }
                let ch = &remaining[pos..pos + '◄'.len_utf8()];
                spans.push(Span::styled(ch.to_string(), link_s));
                remaining = &remaining[pos + ch.len()..];
            } else {
                if !remaining.is_empty() {
                    spans.push(Span::styled(remaining.to_string(), body_s));
                }
                break;
            }
        }
        spans
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

    fn key_desc(key: KeyEvent) -> String {
        let mut parts = Vec::new();
        let m = key.modifiers;
        if m.contains(KeyModifiers::CONTROL) { parts.push("Ctrl"); }
        if m.contains(KeyModifiers::ALT)     { parts.push("Alt"); }
        if m.contains(KeyModifiers::SHIFT)   { parts.push("Shift"); }
        let code = match key.code {
            KeyCode::Char(' ')        => "Space".to_string(),
            KeyCode::Char(c)          => format!("{}", c),
            KeyCode::F(n)             => format!("F{}", n),
            KeyCode::Enter            => "Enter".to_string(),
            KeyCode::Esc              => "Esc".to_string(),
            KeyCode::Backspace        => "Backspace".to_string(),
            KeyCode::Delete           => "Delete".to_string(),
            KeyCode::Tab              => "Tab".to_string(),
            KeyCode::BackTab          => "BackTab".to_string(),
            KeyCode::Left             => "Left".to_string(),
            KeyCode::Right            => "Right".to_string(),
            KeyCode::Up               => "Up".to_string(),
            KeyCode::Down             => "Down".to_string(),
            KeyCode::Home             => "Home".to_string(),
            KeyCode::End              => "End".to_string(),
            KeyCode::PageUp           => "PgUp".to_string(),
            KeyCode::PageDown         => "PgDn".to_string(),
            KeyCode::Insert           => "Ins".to_string(),
            KeyCode::Null             => "Null".to_string(),
            KeyCode::Modifier(mc)     => format!("{:?}", mc),
            _                         => format!("{:?}", key.code),
        };
        parts.push(&code);
        parts.join("+")
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        // F12 toggles keyboard debug mode (works in any mode, no ambiguous byte)
        if !self.faithful && key.code == KeyCode::F(12) && key.modifiers == KeyModifiers::NONE {
            self.kbd_debug = match self.kbd_debug {
                None    => Some("KBD DEBUG ON — press keys (F12=off)".to_string()),
                Some(_) => None,
            };
            return false;
        }
        // In debug mode record every key description in the status bar slot
        if !self.faithful && self.kbd_debug.is_some() {
            self.kbd_debug = Some(format!("KBD: {}  [raw code={:?} mod={:?}]",
                Self::key_desc(key), key.code, key.modifiers));
        }
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
            KeyCode::F(10) => {
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

            KeyCode::F(1) => {
                self.mode = Mode::HelpGettingStarted { scroll: 0 };
            }

            // Ctrl+T — toggle embedded terminal (disabled in --faithful mode)
            KeyCode::Char('t') if m == KeyModifiers::CONTROL && !self.faithful => {
                if self.term_pane.is_some() {
                    // If already open: toggle focus (open→focused→closed cycle)
                    let focused = self.term_pane.as_ref().map(|t| t.focused).unwrap_or(false);
                    if focused {
                        self.term_pane = None; // close
                    } else {
                        self.term_pane.as_mut().unwrap().focused = true;
                    }
                } else {
                    // Open a new terminal pane
                    let w = 80u16; // will be corrected on next Resize
                    let h = 10u16;
                    self.term_pane = term::TermPane::spawn(w, h);
                }
            }

            KeyCode::Char('s') if m == KeyModifiers::CONTROL => self.do_save(),
            KeyCode::F(2) => self.do_save(),

            KeyCode::Char('x') if m == KeyModifiers::CONTROL => {
                self.editor.clear_selection();
                if self.editor.dirty {
                    self.mode = Mode::ConfirmExit;
                } else {
                    return true;
                }
            }

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

            // Ctrl+K — nano-style kill line / cut selection (disabled in --faithful mode)
            KeyCode::Char('k') if m == KeyModifiers::CONTROL && !self.faithful => {
                self.editor.cut_line();
            }
            // Ctrl+C / Ctrl+V — modern copy/paste aliases (disabled in --faithful mode)
            KeyCode::Char('c') if m == KeyModifiers::CONTROL && !self.faithful => {
                self.editor.copy_line();
            }
            KeyCode::Char('v') if m == KeyModifiers::CONTROL && !self.faithful => {
                self.editor.delete_selection();
                self.editor.paste();
            }
            // DOS keyboard shortcuts (always active): Shift+Del / Ctrl+Ins / Shift+Ins
            KeyCode::Delete if m == KeyModifiers::SHIFT => {
                self.editor.cut_line();
            }
            KeyCode::Insert if m == KeyModifiers::CONTROL => {
                self.editor.copy_line();
            }
            KeyCode::Insert if m == KeyModifiers::SHIFT => {
                self.editor.delete_selection();
                self.editor.paste();
            }

            // ── Shift+movement → extend selection ──────────────────────────────
            KeyCode::Left if m == KeyModifiers::SHIFT => {
                if self.editor.selection_anchor.is_none() {
                    self.editor.selection_anchor = Some(self.editor.cursor);
                }
                self.editor.cursor_left();
            }
            KeyCode::Right if m == KeyModifiers::SHIFT => {
                if self.editor.selection_anchor.is_none() {
                    self.editor.selection_anchor = Some(self.editor.cursor);
                }
                self.editor.cursor_right();
            }
            KeyCode::Up if m == KeyModifiers::SHIFT => {
                if self.editor.selection_anchor.is_none() {
                    self.editor.selection_anchor = Some(self.editor.cursor);
                }
                self.editor.cursor_up();
            }
            KeyCode::Down if m == KeyModifiers::SHIFT => {
                if self.editor.selection_anchor.is_none() {
                    self.editor.selection_anchor = Some(self.editor.cursor);
                }
                self.editor.cursor_down();
            }
            KeyCode::Home if m == KeyModifiers::SHIFT => {
                if self.editor.selection_anchor.is_none() {
                    self.editor.selection_anchor = Some(self.editor.cursor);
                }
                self.editor.home();
            }
            KeyCode::End if m == KeyModifiers::SHIFT => {
                if self.editor.selection_anchor.is_none() {
                    self.editor.selection_anchor = Some(self.editor.cursor);
                }
                self.editor.end();
            }

            // ── Bare movement → clear selection ────────────────────────────────
            KeyCode::Left => { self.editor.clear_selection(); self.editor.cursor_left(); }
            KeyCode::Right => { self.editor.clear_selection(); self.editor.cursor_right(); }
            KeyCode::Up => { self.editor.clear_selection(); self.editor.cursor_up(); }
            KeyCode::Down => { self.editor.clear_selection(); self.editor.cursor_down(); }
            KeyCode::Home if m == KeyModifiers::CONTROL => {
                self.editor.clear_selection();
                self.editor.cursor = (0, 0);
            }
            KeyCode::End if m == KeyModifiers::CONTROL => {
                self.editor.clear_selection();
                let last = self.editor.lines.len().saturating_sub(1);
                let col = self.editor.lines[last].chars().count();
                self.editor.cursor = (col, last);
            }
            KeyCode::Home => { self.editor.clear_selection(); self.editor.home(); }
            KeyCode::End => { self.editor.clear_selection(); self.editor.end(); }
            KeyCode::PageUp => { self.editor.clear_selection(); self.editor.page_up(self.page_height); }
            KeyCode::PageDown => { self.editor.clear_selection(); self.editor.page_down(self.page_height); }

            KeyCode::Insert => {
                self.editor.overtype = !self.editor.overtype;
            }
            KeyCode::Delete => {
                if !self.editor.delete_selection() {
                    self.editor.delete();
                }
            }
            KeyCode::Backspace => {
                if !self.editor.delete_selection() {
                    self.editor.backspace();
                }
            }
            KeyCode::Enter => {
                self.editor.delete_selection();
                self.editor.insert_newline();
            }
            KeyCode::Tab => {
                self.editor.delete_selection();
                for _ in 0..4 {
                    self.editor.insert_char(' ');
                }
            }
            KeyCode::Char(c) => {
                self.message = None;
                self.editor.delete_selection();
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
                self.mode = Mode::Open("*.TXT".to_string());
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
                return false;
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
