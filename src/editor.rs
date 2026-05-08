use crate::syntax::{to_ratatui_style, SyntaxHighlighter};
use ratatui::text::{Line, Span};
use std::fs;
use std::io;
use std::path::Path;
use syntect::easy::HighlightLines;

pub struct Editor {
    pub lines: Vec<String>,
    pub cursor: (usize, usize), // x (column), y (row)
    pub filename: Option<String>,
    pub scroll: (u16, u16),
    pub overtype: bool,
    pub dirty: bool,
    pub clip: String,
    pub selection_anchor: Option<(usize, usize)>, // (x, y) where selection started
    highlighter: SyntaxHighlighter,
    pub highlighted: Vec<Line<'static>>,
}

impl Editor {
    pub fn new() -> Self {
        let mut ed = Self {
            lines: vec![String::new()],
            cursor: (0, 0),
            filename: None,
            scroll: (0, 0),
            overtype: false,
            dirty: false,
            clip: String::new(),
            selection_anchor: None,
            highlighter: SyntaxHighlighter::new(),
            highlighted: Vec::new(),
        };
        ed.highlight();
        ed
    }

    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()> {
        let text = fs::read_to_string(path.as_ref())?;
        self.lines = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(|s| s.to_string()).collect()
        };
        self.filename = Some(path.as_ref().to_string_lossy().into_owned());
        self.cursor = (0, 0);
        self.scroll = (0, 0);
        self.dirty = false;
        self.highlight();
        Ok(())
    }

    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        fs::write(path.as_ref(), self.lines.join("\n"))?;
        Ok(())
    }

    pub fn cursor_left(&mut self) {
        if self.cursor.0 > 0 {
            self.cursor.0 -= 1;
        } else if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
            self.cursor.0 = self.lines[self.cursor.1].chars().count();
        }
    }

    pub fn cursor_right(&mut self) {
        let line_len = self.lines[self.cursor.1].chars().count();
        if self.cursor.0 < line_len {
            self.cursor.0 += 1;
        } else if self.cursor.1 + 1 < self.lines.len() {
            self.cursor.1 += 1;
            self.cursor.0 = 0;
        }
    }

    pub fn cursor_up(&mut self) {
        if self.cursor.1 > 0 {
            self.cursor.1 -= 1;
        }
    }

    pub fn cursor_down(&mut self) {
        if self.cursor.1 + 1 < self.lines.len() {
            self.cursor.1 += 1;
        }
    }

    pub fn home(&mut self) {
        self.cursor.0 = 0;
    }

    pub fn end(&mut self) {
        self.cursor.0 = self.lines[self.cursor.1].chars().count();
    }

    pub fn page_up(&mut self, page_height: usize) {
        for _ in 0..page_height.saturating_sub(1) {
            self.cursor_up();
        }
    }

    pub fn page_down(&mut self, page_height: usize) {
        for _ in 0..page_height.saturating_sub(1) {
            self.cursor_down();
        }
    }

    pub fn goto_line(&mut self, line: usize) {
        let y = line.saturating_sub(1).min(self.lines.len().saturating_sub(1));
        self.cursor.1 = y;
        self.cursor.0 = 0;
    }

    pub fn find_next(&mut self, query: &str) -> bool {
        if query.is_empty() {
            return false;
        }
        let (start_x, start_y) = self.cursor;
        for y in start_y..self.lines.len() {
            let line = &self.lines[y];
            let offset = if y == start_y {
                start_x.saturating_add(1)
            } else {
                0
            };
            if offset >= line.chars().count() {
                continue;
            }
            let byte_offset = line
                .chars()
                .take(offset)
                .map(|c| c.len_utf8())
                .sum::<usize>();
            if let Some(byte_pos) = line[byte_offset..].find(query) {
                let char_pos = line[..byte_offset + byte_pos].chars().count();
                self.cursor = (char_pos, y);
                return true;
            }
        }
        for y in 0..=start_y {
            let line = &self.lines[y];
            if let Some(byte_pos) = line.find(query) {
                let char_pos = line[..byte_pos].chars().count();
                self.cursor = (char_pos, y);
                return true;
            }
        }
        false
    }

    pub fn insert_char(&mut self, c: char) {
        let (x, y) = self.cursor;
        if y >= self.lines.len() {
            self.lines.push(String::new());
        }
        let line = &mut self.lines[y];
        // Pad with spaces if cursor is past end of line (virtual space mode)
        let line_len = line.chars().count();
        if x > line_len {
            line.extend(std::iter::repeat(' ').take(x - line_len));
        }
        let byte_idx = line
            .char_indices()
            .nth(x)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        if self.overtype && byte_idx < line.len() {
            if let Some(ch) = line[byte_idx..].chars().next() {
                line.replace_range(byte_idx..byte_idx + ch.len_utf8(), &c.to_string());
            } else {
                line.insert(byte_idx, c);
            }
        } else {
            line.insert(byte_idx, c);
        }
        self.cursor.0 += 1;
        self.dirty = true;
        self.highlight();
    }

    pub fn insert_newline(&mut self) {
        let (x, y) = self.cursor;
        if y < self.lines.len() {
            let line = &mut self.lines[y];
            let byte_idx = line
                .char_indices()
                .nth(x)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            let rest = line.split_off(byte_idx);
            self.lines.insert(y + 1, rest);
            self.cursor.0 = 0;
            self.cursor.1 += 1;
            self.dirty = true;
            self.highlight();
        }
    }

    pub fn backspace(&mut self) {
        let (x, y) = self.cursor;
        if x > 0 {
            let line = &mut self.lines[y];
            let byte_idx = line
                .char_indices()
                .nth(x - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let end_idx = line
                .char_indices()
                .nth(x)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            line.drain(byte_idx..end_idx);
            self.cursor.0 -= 1;
            self.dirty = true;
            self.highlight();
        } else if y > 0 {
            let line = self.lines.remove(y);
            let prev_len = self.lines[y - 1].chars().count();
            self.lines[y - 1].push_str(&line);
            self.cursor.1 -= 1;
            self.cursor.0 = prev_len;
            self.dirty = true;
            self.highlight();
        }
    }

    pub fn delete(&mut self) {
        let (x, y) = self.cursor;
        if y >= self.lines.len() {
            return;
        }
        let line = &mut self.lines[y];
        let byte_idx = line
            .char_indices()
            .nth(x)
            .map(|(i, _)| i)
            .unwrap_or(line.len());
        if byte_idx < line.len() {
            let ch = line[byte_idx..].chars().next().unwrap();
            line.drain(byte_idx..byte_idx + ch.len_utf8());
            self.dirty = true;
            self.highlight();
        } else if y + 1 < self.lines.len() {
            let next = self.lines.remove(y + 1);
            self.lines[y].push_str(&next);
            self.dirty = true;
            self.highlight();
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection_anchor.map(|a| a != self.cursor).unwrap_or(false)
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    // Returns ((sx, sy), (ex, ey)) in document order (start ≤ end).
    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let anchor = self.selection_anchor?;
        let cursor = self.cursor;
        if (anchor.1, anchor.0) <= (cursor.1, cursor.0) {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
        }
    }

    pub fn get_selected_text(&self) -> String {
        if !self.has_selection() {
            return String::new();
        }
        let ((sx, sy), (ex, ey)) = match self.selection_range() {
            Some(r) => r,
            None => return String::new(),
        };
        if sy == ey {
            let chars: Vec<char> = self.lines[sy].chars().collect();
            chars[sx..ex.min(chars.len())].iter().collect()
        } else {
            let mut result = String::new();
            for y in sy..=ey {
                let chars: Vec<char> = self.lines[y].chars().collect();
                if y == sy {
                    result.extend(chars[sx..].iter());
                    result.push('\n');
                } else if y == ey {
                    result.extend(chars[..ex.min(chars.len())].iter());
                } else {
                    result.push_str(&self.lines[y]);
                    result.push('\n');
                }
            }
            result
        }
    }

    // Deletes the current selection and moves cursor to its start.
    // Returns true if anything was deleted.
    pub fn delete_selection(&mut self) -> bool {
        if !self.has_selection() {
            return false;
        }
        let ((sx, sy), (ex, ey)) = match self.selection_range() {
            Some(r) => r,
            None => return false,
        };
        if sy == ey {
            let chars: Vec<char> = self.lines[sy].chars().collect();
            let new_line: String = chars[..sx]
                .iter()
                .chain(chars[ex.min(chars.len())..].iter())
                .collect();
            self.lines[sy] = new_line;
        } else {
            let end_chars: Vec<char> = self.lines[ey].chars().collect();
            let end_tail: String = end_chars[ex.min(end_chars.len())..].iter().collect();
            let start_chars: Vec<char> = self.lines[sy].chars().collect();
            self.lines[sy] = start_chars[..sx].iter().collect::<String>() + &end_tail;
            self.lines.drain(sy + 1..=ey);
        }
        self.cursor = (sx, sy);
        self.selection_anchor = None;
        self.dirty = true;
        self.highlight();
        true
    }

    pub fn copy_line(&mut self) {
        if self.has_selection() {
            self.clip = self.get_selected_text();
            self.selection_anchor = None;
        } else if self.cursor.1 < self.lines.len() {
            self.clip = self.lines[self.cursor.1].clone();
        }
    }

    pub fn cut_line(&mut self) {
        if self.has_selection() {
            self.clip = self.get_selected_text();
            self.delete_selection();
            return;
        }
        self.copy_line();
        if self.lines.len() > 1 {
            self.lines.remove(self.cursor.1);
            if self.cursor.1 >= self.lines.len() {
                self.cursor.1 = self.lines.len() - 1;
            }
            self.cursor.0 = self.cursor.0.min(self.lines[self.cursor.1].chars().count());
        } else {
            self.lines[0].clear();
            self.cursor.0 = 0;
        }
        self.dirty = true;
        self.highlight();
    }

    pub fn paste(&mut self) {
        let (x, y) = self.cursor;
        if y < self.lines.len() && !self.clip.is_empty() {
            let line = &mut self.lines[y];
            let byte_idx = line
                .char_indices()
                .nth(x)
                .map(|(i, _)| i)
                .unwrap_or(line.len());
            line.insert_str(byte_idx, &self.clip);
            self.cursor.0 += self.clip.chars().count();
            self.dirty = true;
            self.highlight();
        }
    }

    pub fn highlight(&mut self) {
        let ext = self
            .filename
            .as_ref()
            .and_then(|f| Path::new(f).extension())
            .and_then(|e| e.to_str());
        let ss = &self.highlighter.syntax_set;
        let syntax = ext
            .and_then(|e| ss.find_syntax_by_extension(e))
            .unwrap_or_else(|| ss.find_syntax_plain_text());
        let theme = &self.highlighter.theme;
        let mut h = HighlightLines::new(syntax, theme);

        self.highlighted = self
            .lines
            .iter()
            .map(|line| {
                let regions = h.highlight_line(line, ss).unwrap_or_default();
                let spans: Vec<Span<'static>> = regions
                    .into_iter()
                    .map(|(style, text)| Span::styled(text.to_string(), to_ratatui_style(style)))
                    .collect();
                Line::from(spans)
            })
            .collect();
    }
}
