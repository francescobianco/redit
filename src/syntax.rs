use ratatui::style::{Color, Style as RatatuiStyle};
use syntect::highlighting::{Color as SyntectColor, Style, ThemeSet};
use syntect::parsing::SyntaxSet;

pub struct SyntaxHighlighter {
    pub syntax_set: SyntaxSet,
    pub theme: syntect::highlighting::Theme,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = theme_set
            .themes
            .get("base16-ocean.dark")
            .cloned()
            .unwrap_or_else(|| {
                theme_set
                    .themes
                    .get("InspiredGitHub")
                    .cloned()
                    .unwrap_or_else(|| ThemeSet::load_defaults().themes["base16-ocean.dark"].clone())
            });
        Self { syntax_set, theme }
    }
}

pub fn to_ratatui_style(style: Style) -> RatatuiStyle {
    RatatuiStyle::default()
        .fg(to_ratatui_color(style.foreground))
        .bg(to_ratatui_color(style.background))
}

fn to_ratatui_color(c: SyntectColor) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}
