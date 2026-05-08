use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
};

use super::App;

impl App {
    // ── V1 editor area ────────────────────────────────────────────────────────

    /// Faithful MS-DOS EDIT look:
    ///   top border with centred filename, vertical scrollbar, horizontal
    ///   scrollbar on the last row, no bottom border.
    pub(super) fn render_editor_v1(&mut self, f: &mut Frame, area: Rect) {
        let t = &self.theme;
        let frame_style = Style::default().bg(t.frame_bg).fg(t.frame_fg);
        let side_style = Style::default().bg(t.edit_bg).fg(t.frame_fg);
        let title_style = Style::default().bg(t.title_bg).fg(t.title_fg);
        let scroll_style = Style::default().bg(t.scroll_bg).fg(t.scroll_fg);
        let base = Style::default().bg(t.edit_bg).fg(t.edit_fg);
        let cur_style = Style::default().bg(t.edit_fg).fg(t.edit_bg);
        let w = area.width as usize;

        // ── Top border with centred filename ──────────────────────────────────
        let fname = self
            .editor
            .filename
            .as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled")
            .to_string();
        let title_inner = format!(" {} ", fname);
        let tl = title_inner.len();
        let dashes = w.saturating_sub(2 + tl);
        let left_d = dashes.saturating_sub(1) / 2;
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

        // ── Content rows (area.height - 2: top border + horizontal scrollbar)
        let content_h = (area.height - 2) as usize;
        let text_w = w.saturating_sub(2); // between │ and vertical scrollbar

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

            // Left border │ — same background as text area so it blends in
            f.render_widget(
                Paragraph::new(Span::styled("│", side_style)),
                Rect::new(area.x, screen_y, 1, 1),
            );

            // Vertical scrollbar on right
            let sc = Self::vscroll_char(row, content_h, total_lines, sy);
            f.render_widget(
                Paragraph::new(Span::styled(sc, scroll_style)),
                Rect::new(area.x + area.width - 1, screen_y, 1, 1),
            );

            let text_area = Rect::new(area.x + 1, screen_y, text_w as u16, 1);

            if line_idx >= total_lines {
                f.render_widget(
                    Paragraph::new(Span::styled(" ".repeat(text_w), base)),
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

        // ── Horizontal scrollbar (last row of edit area) ──────────────────────
        let hscroll_y = area.y + area.height - 1;
        let track_w = w.saturating_sub(5); // space after left arrow, then track
        let max_line_w = self
            .editor
            .lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let h_thumb = Self::hscroll_thumb(sx, text_w, max_line_w, track_w);

        let mut hbar: Vec<Span> = Vec::new();
        hbar.push(Span::styled("│← ", scroll_style));
        for i in 0..track_w {
            let ch = if h_thumb.1 > 0 && i >= h_thumb.0 && i < h_thumb.0 + h_thumb.1 {
                "█"
            } else {
                "░"
            };
            hbar.push(Span::styled(ch, scroll_style));
        }
        hbar.push(Span::styled("→│", scroll_style));
        f.render_widget(
            Paragraph::new(Line::from(hbar)),
            Rect::new(area.x, hscroll_y, area.width, 1),
        );
    }

    /// Character for the vertical scrollbar at `row` (0-indexed in content block).
    pub(super) fn vscroll_char(
        row: usize,
        content_h: usize,
        total_doc: usize,
        scroll_y: usize,
    ) -> &'static str {
        if row == 0 {
            return "↑";
        }
        if row == content_h - 1 {
            return "↓";
        }
        if content_h <= 2 {
            return "░";
        }
        let body = content_h - 2;
        if total_doc <= content_h {
            return "░";
        }
        let thumb_size = (body * content_h / total_doc).max(1).min(body);
        let max_offset = body - thumb_size;
        let max_scroll = total_doc - content_h;
        let thumb_pos = if max_scroll == 0 {
            0
        } else {
            max_offset * scroll_y / max_scroll
        };
        let body_row = row - 1;
        if body_row >= thumb_pos && body_row < thumb_pos + thumb_size {
            "█"
        } else {
            "░"
        }
    }

    /// (thumb_start, thumb_size) for the horizontal scrollbar track.
    /// Returns (0, 0) when all content fits horizontally.
    pub(super) fn hscroll_thumb(
        scroll_x: usize,
        view_w: usize,
        max_line_w: usize,
        track_w: usize,
    ) -> (usize, usize) {
        if max_line_w <= view_w || track_w == 0 {
            return (0, 0);
        }
        let thumb_size = (track_w * view_w / max_line_w).max(1).min(track_w);
        let max_scroll = max_line_w - view_w;
        let thumb_pos = (track_w - thumb_size) * scroll_x / max_scroll;
        (thumb_pos, thumb_size)
    }

    // ── V1 welcome / credits dialog ───────────────────────────────────────────

    pub(super) fn welcome_dialog(&self, f: &mut Frame) {
        let t = &self.theme;
        let size = f.area();
        let area = Rect::new(
            size.width.saturating_sub(58) / 2 + 1,
            (size.height.saturating_sub(11) / 2).saturating_sub(2),
            58.min(size.width),
            11.min(size.height),
        );
        f.render_widget(Clear, area);

        let dlg_style = Style::default().bg(t.dlg_bg).fg(t.dlg_fg);
        let line_style = Style::default().bg(t.frame_bg).fg(t.frame_fg);
        let iw = area.width as usize - 2;

        // Top border
        f.render_widget(
            Paragraph::new(Span::styled(format!("┌{}┐", "─".repeat(iw)), line_style)),
            Rect::new(area.x, area.y, area.width, 1),
        );

        let rows: &[&str] = &[
            "",
            "              Welcome to the MS-DOS Editor",
            "",
            "     Copyright (C) Microsoft Corporation, 1987-1992.",
            "                  All rights reserved.",
            "",
            "       < Press Enter to see the Survival Guide >",
        ];

        for (i, &row) in rows.iter().enumerate() {
            let y = area.y + 1 + i as u16;
            f.render_widget(
                Paragraph::new(Span::styled("│", line_style)),
                Rect::new(area.x, y, 1, 1),
            );
            f.render_widget(
                Paragraph::new(Span::styled(format!("{:<w$}", row, w = iw), dlg_style)),
                Rect::new(area.x + 1, y, iw as u16, 1),
            );
            f.render_widget(
                Paragraph::new(Span::styled("│", line_style)),
                Rect::new(area.x + area.width - 1, y, 1, 1),
            );
        }

        // Separator ├──┤
        let sep_y = area.y + 1 + rows.len() as u16;
        f.render_widget(
            Paragraph::new(Span::styled(format!("├{}┤", "─".repeat(iw)), line_style)),
            Rect::new(area.x, sep_y, area.width, 1),
        );

        // ESC dismiss row
        let esc_y = sep_y + 1;
        let esc_text = format!("{:^w$}", "< Press ESC to clear this dialog box >", w = iw);
        f.render_widget(
            Paragraph::new(Span::styled("│", line_style)),
            Rect::new(area.x, esc_y, 1, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled(esc_text, dlg_style)),
            Rect::new(area.x + 1, esc_y, iw as u16, 1),
        );
        f.render_widget(
            Paragraph::new(Span::styled("│", line_style)),
            Rect::new(area.x + area.width - 1, esc_y, 1, 1),
        );

        // Bottom border
        f.render_widget(
            Paragraph::new(Span::styled(format!("└{}┘", "─".repeat(iw)), line_style)),
            Rect::new(area.x, esc_y + 1, area.width, 1),
        );
    }
}
