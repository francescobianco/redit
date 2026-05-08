use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::{
    io::Write,
    sync::mpsc::{self, Receiver},
    thread,
};

const HISTORY_CAP: usize = 262_144; // 256 KB — bounds the resize replay cost

pub struct TermPane {
    pub height: u16,
    pub width: u16,
    pub focused: bool,
    pub closed: bool,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    rx: Receiver<Vec<u8>>,
    pub parser: vt100::Parser,
    history: Vec<u8>, // bounded log of raw PTY bytes for resize replay
}

impl TermPane {
    pub fn spawn(width: u16, height: u16) -> Option<Self> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: height,
                cols: width,
                pixel_width: 0,
                pixel_height: 0,
            })
            .ok()?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");

        let _child = pair.slave.spawn_command(cmd).ok()?;

        let master = pair.master;
        let mut reader = master.try_clone_reader().ok()?;
        let writer = master.take_writer().ok()?;

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) | Err(_) => {
                        // Empty vec signals EOF to the main thread
                        let _ = tx.send(vec![]);
                        break;
                    }
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        Some(Self {
            height,
            width,
            focused: true,
            closed: false,
            master,
            writer,
            rx,
            parser: vt100::Parser::new(height, width, 0),
            history: Vec::new(),
        })
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        let _ = self.master.resize(PtySize {
            rows: height,
            cols: width,
            pixel_width: 0,
            pixel_height: 0,
        });
        // Recreate parser with new dimensions and replay history to restore content
        self.parser = vt100::Parser::new(height, width, 0);
        self.parser.process(&self.history);
    }

    pub fn write_input(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    // Drain pending PTY output into the vt100 parser. Call each frame.
    // Sets self.closed = true when the shell exits (EOF from PTY).
    pub fn drain(&mut self) {
        while let Ok(bytes) = self.rx.try_recv() {
            if bytes.is_empty() {
                self.closed = true;
            } else {
                // Append to history, trimming the oldest bytes when over cap
                self.history.extend_from_slice(&bytes);
                if self.history.len() > HISTORY_CAP {
                    let trim = self.history.len() - HISTORY_CAP;
                    self.history.drain(..trim);
                }
                self.parser.process(&bytes);
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        self.drain();
        let screen = self.parser.screen();

        let sep_style = Style::default().bg(Color::DarkGray).fg(Color::White);
        let bdr_style = Style::default().bg(Color::DarkGray).fg(Color::White);
        let inner_bg  = Style::default().bg(Color::Black).fg(Color::Reset);

        // ── Separator with ├ label ┤ connectors ──────────────────────────────
        let label = if self.focused {
            " Terminal (Ctrl+T: unfocus  Ctrl+↑↓: resize) "
        } else {
            " Terminal (Ctrl+T: focus) "
        };
        // inner dashes: area.width - 2 (for ├ and ┤), minus label length
        let inner_w = area.width.saturating_sub(2) as usize;
        let dashes = inner_w.saturating_sub(label.len());
        let left_d  = dashes / 2;
        let right_d = dashes - left_d;
        let sep_line = format!(
            "├{}{}{}┤",
            "─".repeat(left_d),
            label,
            "─".repeat(right_d),
        );
        f.render_widget(
            Paragraph::new(Span::styled(sep_line, sep_style)),
            Rect::new(area.x, area.y, area.width, 1),
        );

        // ── Content rows with │ borders ──────────────────────────────────────
        let content_w = area.width.saturating_sub(2) as usize; // between the two │

        for row in 0..self.height.min(area.height.saturating_sub(1)) {
            let y = area.y + 1 + row;

            // Left border
            f.render_widget(
                Paragraph::new(Span::styled("│", bdr_style)),
                Rect::new(area.x, y, 1, 1),
            );
            // Right border
            f.render_widget(
                Paragraph::new(Span::styled("│", bdr_style)),
                Rect::new(area.x + area.width - 1, y, 1, 1),
            );

            // Terminal content in the inner area
            let content_area = Rect::new(area.x + 1, y, content_w as u16, 1);
            let mut spans: Vec<Span> = Vec::new();
            let mut rendered = 0usize;

            for col in 0..self.width.min(content_w as u16) {
                let Some(cell) = screen.cell(row, col) else { break };
                let fg = vt_color(cell.fgcolor());
                let bg = vt_color(cell.bgcolor());
                let style = Style::default().fg(fg).bg(bg);
                let ch = if cell.has_contents() {
                    cell.contents().to_string()
                } else {
                    " ".to_string()
                };
                match spans.last_mut() {
                    Some(last) if last.style == style => {
                        last.content = (last.content.to_string() + &ch).into();
                    }
                    _ => spans.push(Span::styled(ch, style)),
                }
                rendered += 1;
            }

            // Pad remainder to content_w
            if rendered < content_w {
                let pad = " ".repeat(content_w - rendered);
                match spans.last_mut() {
                    Some(last) if last.style == inner_bg => {
                        last.content = (last.content.to_string() + &pad).into();
                    }
                    _ => spans.push(Span::styled(pad, inner_bg)),
                }
            }

            f.render_widget(
                Paragraph::new(Line::from(spans)),
                content_area,
            );
        }
    }
}

fn vt_color(c: vt100::Color) -> Color {
    match c {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(i) => Color::Indexed(i),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
