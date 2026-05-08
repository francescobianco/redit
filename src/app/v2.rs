use ratatui::{
    layout::Rect,
    style::Style,
    text::Span,
    widgets::Paragraph,
    Frame,
};

use super::App;

impl App {
    // ── V2 editor area ────────────────────────────────────────────────────────

    /// Simplified look: full box border (top + bottom), no title, no scrollbar.
    pub(super) fn render_editor_v2(&mut self, f: &mut Frame, area: Rect) {
        let t            = &self.theme;
        let border_style = Style::default().bg(t.frame_bg).fg(t.frame_fg);
        let base         = Style::default().bg(t.edit_bg).fg(t.edit_fg);
        let cur_style    = Style::default().bg(t.edit_fg).fg(t.edit_bg);
        let w            = area.width as usize;

        // Top border
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("┌{}┐", "─".repeat(w.saturating_sub(2))),
                border_style,
            )),
            Rect::new(area.x, area.y, area.width, 1),
        );

        // Bottom border
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("└{}┘", "─".repeat(w.saturating_sub(2))),
                border_style,
            )),
            Rect::new(area.x, area.y + area.height - 1, area.width, 1),
        );

        let inner = Rect::new(area.x + 1, area.y + 1, (w - 2) as u16, area.height - 2);
        let vh    = inner.height as usize;
        let vw    = inner.width as usize;
        let (cx, cy) = self.editor.cursor;

        // Adjust scroll
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
                f.render_widget(
                    Paragraph::new(Span::styled(" ".repeat(vw), base)),
                    text_area,
                );
                continue;
            }

            let on_this_line = line_idx == cy;
            if line_idx < self.editor.highlighted.len() {
                let hl = self.editor.highlighted[line_idx].clone();
                Self::render_highlighted_row(
                    f, text_area, &hl, sx, cx, on_this_line,
                    t.edit_bg, cur_style, 8,
                );
            } else {
                let chars: Vec<char> = self.editor.lines[line_idx].chars().collect();
                Self::render_text_row(f, text_area, &chars, sx, cx, on_this_line, base, cur_style);
            }
        }
    }
}
