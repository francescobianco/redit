use ratatui::style::Color;

/// Which visual variant to use when rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Version {
    /// Faithful MS-DOS EDIT reproduction (colours from dosemu ANSI capture)
    V1,
    /// Simplified modern look (current implementation)
    V2,
}

/// All colours used by the UI — swap the Theme to change the whole look.
#[derive(Debug, Clone)]
pub struct Theme {
    pub version: Version,

    // Menu bar
    pub menu_bg: Color,
    pub menu_fg: Color,

    // Dropdown items
    pub drop_bg: Color,
    pub drop_fg: Color,
    pub drop_sel_bg: Color,
    pub drop_sel_fg: Color,

    // Editor text area
    pub edit_bg: Color,
    pub edit_fg: Color,

    // Box-drawing frame  (┌─┐│└─┘)
    pub frame_bg: Color,
    pub frame_fg: Color,

    // Filename shown centred in the top border (V1 only)
    pub title_bg: Color,
    pub title_fg: Color,

    // Scrollbar column on the right (V1 only)
    pub scroll_bg: Color,
    pub scroll_fg: Color,

    // Status bar
    pub stat_bg: Color,
    pub stat_fg: Color,

    // Dialog windows
    pub dlg_bg: Color,
    pub dlg_fg: Color,
    pub dlg_btn_bg: Color,
    pub dlg_btn_fg: Color,
    pub dlg_inp_bg: Color,
    pub dlg_inp_fg: Color,
}

/// V1 — exact MS-DOS EDIT 2.0 palette extracted from dosemu ANSI output.
///
/// ESC[30m = Black fg   ESC[47m = Light-gray bg   (menu / title highlight)
/// ESC[37m = White fg   ESC[40m = Black bg         (frame box-drawing chars)
/// ESC[44m = Blue bg                                (editor area)
/// ESC[90m = Dark-gray fg                           (scrollbar)
pub fn v1() -> Theme {
    Theme {
        version: Version::V1,

        menu_bg:      Color::Gray,       // ESC[47m ≈ light gray
        menu_fg:      Color::Black,      // ESC[30m

        drop_bg:      Color::White,
        drop_fg:      Color::Black,
        drop_sel_bg:  Color::Black,
        drop_sel_fg:  Color::White,

        edit_bg:      Color::Blue,       // ESC[44m
        edit_fg:      Color::White,      // ESC[37m

        frame_bg:     Color::Black,      // ESC[40m — box chars on black
        frame_fg:     Color::White,      // ESC[37m

        title_bg:     Color::Gray,       // ESC[47m — filename inverted in border
        title_fg:     Color::Black,      // ESC[30m

        scroll_bg:    Color::Black,      // ESC[40m
        scroll_fg:    Color::DarkGray,   // ESC[90m

        stat_bg:      Color::Gray,       // ESC[47m
        stat_fg:      Color::Black,      // ESC[30m

        dlg_bg:       Color::Cyan,
        dlg_fg:       Color::Black,
        dlg_btn_bg:   Color::Black,
        dlg_btn_fg:   Color::White,
        dlg_inp_bg:   Color::White,
        dlg_inp_fg:   Color::Black,
    }
}

/// V2 — simplified modern look (the previous default).
pub fn v2() -> Theme {
    Theme {
        version: Version::V2,

        menu_bg:      Color::Gray,
        menu_fg:      Color::Black,

        drop_bg:      Color::White,
        drop_fg:      Color::Black,
        drop_sel_bg:  Color::Black,
        drop_sel_fg:  Color::White,

        edit_bg:      Color::Blue,
        edit_fg:      Color::White,

        // In V2 the frame blends into the editor background
        frame_bg:     Color::Blue,
        frame_fg:     Color::White,

        title_bg:     Color::Blue,
        title_fg:     Color::White,

        scroll_bg:    Color::Blue,
        scroll_fg:    Color::White,

        stat_bg:      Color::Gray,
        stat_fg:      Color::Black,

        dlg_bg:       Color::Cyan,
        dlg_fg:       Color::Black,
        dlg_btn_bg:   Color::Black,
        dlg_btn_fg:   Color::White,
        dlg_inp_bg:   Color::White,
        dlg_inp_fg:   Color::Black,
    }
}