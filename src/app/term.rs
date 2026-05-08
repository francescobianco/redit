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

pub struct TermPane {
    pub height: u16,
    pub width: u16,
    pub focused: bool,
    pub closed: bool,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    rx: Receiver<Vec<u8>>,
    pub parser: vt100::Parser,
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
        self.parser = vt100::Parser::new(height, width, 0);
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
                self.parser.process(&bytes);
            }
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        self.drain();
        let screen = self.parser.screen();

        // Separator line
        let sep_style = Style::default().bg(Color::DarkGray).fg(Color::White);
        let label = if self.focused { "─ Terminal (Ctrl+T to unfocus) " } else { "─ Terminal (Ctrl+T to focus) " };
        let sep_line = format!("{}{}", label, "─".repeat(area.width.saturating_sub(label.len() as u16) as usize));
        f.render_widget(
            Paragraph::new(Span::styled(sep_line, sep_style)),
            Rect::new(area.x, area.y, area.width, 1),
        );

        // Terminal rows
        for row in 0..self.height.min(area.height.saturating_sub(1)) {
            let y = area.y + 1 + row;
            let mut spans: Vec<Span> = Vec::new();

            for col in 0..self.width.min(area.width) {
                let Some(cell) = screen.cell(row, col) else { continue };
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
            }

            // Pad to area width
            let rendered_cols = self.width.min(area.width) as usize;
            if rendered_cols < area.width as usize {
                let pad = " ".repeat(area.width as usize - rendered_cols);
                spans.push(Span::styled(pad, Style::default().bg(Color::Black)));
            }

            f.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(area.x, y, area.width, 1),
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
