use crate::theme::{self, Theme, Version};
use ratatui::style::Color;
use std::{env, fs, io, path::PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct UserSettings {
    pub style: Version,
    pub colors: ThemeColors,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThemeColors {
    pub menu_fg: Color,
    pub menu_bg: Color,
    pub editor_fg: Color,
    pub editor_bg: Color,
    pub status_fg: Color,
    pub status_bg: Color,
    pub dialog_fg: Color,
    pub dialog_bg: Color,
    pub title_fg: Color,
    pub title_bg: Color,
    pub scrollbar_fg: Color,
    pub scrollbar_bg: Color,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            style: Version::V1,
            colors: ThemeColors::from_theme(&theme::v1()),
        }
    }
}

impl UserSettings {
    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::default();
        };
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };

        let entries = parse_ini_entries(&text);
        let mut settings = Self::default();
        for (section, key, value) in &entries {
            if section == "editor" && key == "style" {
                if let Some(version) = parse_version(value) {
                    settings.style = version;
                    settings.colors = ThemeColors::from_theme(&base_theme(version));
                }
            }
        }
        for (section, key, value) in entries {
            if section == "colors" {
                settings.colors.set(&key, parse_color(&value));
            }
        }
        settings
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "HOME is not set; cannot write .reditrc")
        })?;
        fs::write(path, self.to_ini())
    }

    pub fn theme(&self) -> Theme {
        let mut theme = base_theme(self.style);
        self.colors.apply_to(&mut theme);
        theme
    }

    pub fn set_style(&mut self, style: Version) {
        self.style = style;
        self.colors = ThemeColors::from_theme(&base_theme(style));
    }

    pub fn path() -> Option<PathBuf> {
        env::var_os("HOME").map(|home| PathBuf::from(home).join(".reditrc"))
    }

    fn to_ini(&self) -> String {
        let c = &self.colors;
        format!(
            "[editor]\nstyle={}\n\n[colors]\nmenu_fg={}\nmenu_bg={}\neditor_fg={}\neditor_bg={}\nstatus_fg={}\nstatus_bg={}\ndialog_fg={}\ndialog_bg={}\ntitle_fg={}\ntitle_bg={}\nscrollbar_fg={}\nscrollbar_bg={}\n",
            version_name(self.style),
            color_name(c.menu_fg),
            color_name(c.menu_bg),
            color_name(c.editor_fg),
            color_name(c.editor_bg),
            color_name(c.status_fg),
            color_name(c.status_bg),
            color_name(c.dialog_fg),
            color_name(c.dialog_bg),
            color_name(c.title_fg),
            color_name(c.title_bg),
            color_name(c.scrollbar_fg),
            color_name(c.scrollbar_bg),
        )
    }
}

impl ThemeColors {
    pub fn from_theme(theme: &Theme) -> Self {
        Self {
            menu_fg: theme.menu_fg,
            menu_bg: theme.menu_bg,
            editor_fg: theme.edit_fg,
            editor_bg: theme.edit_bg,
            status_fg: theme.stat_fg,
            status_bg: theme.stat_bg,
            dialog_fg: theme.dlg_fg,
            dialog_bg: theme.dlg_bg,
            title_fg: theme.title_fg,
            title_bg: theme.title_bg,
            scrollbar_fg: theme.scroll_fg,
            scrollbar_bg: theme.scroll_bg,
        }
    }

    pub fn apply_to(&self, theme: &mut Theme) {
        theme.menu_fg = self.menu_fg;
        theme.menu_bg = self.menu_bg;
        theme.drop_fg = self.menu_fg;
        theme.drop_bg = self.menu_bg;
        theme.edit_fg = self.editor_fg;
        theme.edit_bg = self.editor_bg;
        theme.frame_fg = self.editor_fg;
        theme.frame_bg = self.editor_bg;
        theme.stat_fg = self.status_fg;
        theme.stat_bg = self.status_bg;
        theme.dlg_fg = self.dialog_fg;
        theme.dlg_bg = self.dialog_bg;
        theme.title_fg = self.title_fg;
        theme.title_bg = self.title_bg;
        theme.scroll_fg = self.scrollbar_fg;
        theme.scroll_bg = self.scrollbar_bg;
    }

    fn set(&mut self, key: &str, color: Option<Color>) {
        let Some(color) = color else {
            return;
        };
        match key {
            "menu_fg" => self.menu_fg = color,
            "menu_bg" => self.menu_bg = color,
            "editor_fg" => self.editor_fg = color,
            "editor_bg" => self.editor_bg = color,
            "status_fg" => self.status_fg = color,
            "status_bg" => self.status_bg = color,
            "dialog_fg" => self.dialog_fg = color,
            "dialog_bg" => self.dialog_bg = color,
            "title_fg" => self.title_fg = color,
            "title_bg" => self.title_bg = color,
            "scrollbar_fg" => self.scrollbar_fg = color,
            "scrollbar_bg" => self.scrollbar_bg = color,
            _ => {}
        }
    }
}

pub fn color_name(color: Color) -> &'static str {
    match color {
        Color::Black => "black",
        Color::Red => "red",
        Color::Green => "green",
        Color::Yellow => "yellow",
        Color::Blue => "blue",
        Color::Magenta => "magenta",
        Color::Cyan => "cyan",
        Color::Gray => "gray",
        Color::DarkGray => "dark_gray",
        Color::LightRed => "light_red",
        Color::LightGreen => "light_green",
        Color::LightYellow => "light_yellow",
        Color::LightBlue => "light_blue",
        Color::LightMagenta => "light_magenta",
        Color::LightCyan => "light_cyan",
        Color::White => "white",
        _ => "gray",
    }
}

pub fn color_names() -> &'static [&'static str] {
    &[
        "black",
        "blue",
        "green",
        "cyan",
        "red",
        "magenta",
        "yellow",
        "gray",
        "dark_gray",
        "light_blue",
        "light_green",
        "light_cyan",
        "light_red",
        "light_magenta",
        "light_yellow",
        "white",
    ]
}

pub fn parse_color(value: &str) -> Option<Color> {
    match value.trim().to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "dark_grey" => Some(Color::DarkGray),
        "light_red" => Some(Color::LightRed),
        "light_green" => Some(Color::LightGreen),
        "light_yellow" => Some(Color::LightYellow),
        "light_blue" => Some(Color::LightBlue),
        "light_magenta" => Some(Color::LightMagenta),
        "light_cyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        _ => None,
    }
}

pub fn version_name(version: Version) -> &'static str {
    match version {
        Version::V1 => "V1",
        Version::V2 => "V2",
    }
}

pub fn parse_version(value: &str) -> Option<Version> {
    match value.trim().to_ascii_lowercase().as_str() {
        "v1" | "1" => Some(Version::V1),
        "v2" | "2" => Some(Version::V2),
        _ => None,
    }
}

fn base_theme(version: Version) -> Theme {
    match version {
        Version::V1 => theme::v1(),
        Version::V2 => theme::v2(),
    }
}

fn parse_ini_entries(text: &str) -> Vec<(String, String, String)> {
    let mut entries = Vec::new();
    let mut section = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = name.trim().to_ascii_lowercase();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            entries.push((
                section.clone(),
                key.trim().to_ascii_lowercase(),
                value.trim().to_string(),
            ));
        }
    }
    entries
}
