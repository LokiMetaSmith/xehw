use eframe::egui::*;

const RED: Color32 = Color32::from_rgb(0xCC, 0x3E, 0x28);
const BLUE: Color32 = Color32::from_rgb(0x1E, 0x6F, 0xCC);
//const GREEN: Color32 = Color32::from_rgb(0x21, 0x66, 0x09);
//const YELLOW: Color32 = Color32::from_rgb(0xB5, 0x89, 0x00);
// const PURPLE: Color32 = Color32::from_rgb(0x5C, 0x21, 0xA5);
const CYAN: Color32 = Color32::from_rgb(0x15, 0x8C, 0x86);
const BACKGROUND: Color32 = Color32::from_rgb(0xF2, 0xEE, 0xDE);
const TEXT: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
const TEXT_HIGLIGHT: Color32 = Color32::from_rgb(0xD8, 0xD5, 0xC7);
const COMMENT_FG: Color32 = Color32::from_rgb(0xAA, 0xAA, 0xAA);

#[derive(Clone)]
pub struct Theme {
    pub text: Color32,
    pub code_fg: Color32,
    pub error_bg: Color32,
    pub debug_bg: Color32,
    pub comment_fg: Color32,
    pub code_highlight: Color32,
    pub background: Color32,
    pub selection: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            text: TEXT,
            background: BACKGROUND,
            code_fg: TEXT,
            debug_bg: CYAN,
            error_bg: RED,
            code_highlight: TEXT_HIGLIGHT,
            comment_fg: COMMENT_FG,
            selection: BLUE,
        }
    }
}

pub fn tune(ctx: &Context, theme: &Theme) {
    if ctx.style().visuals.widgets.noninteractive.bg_fill == theme.background
        && ctx.style().visuals.selection.bg_fill == theme.selection
        && ctx.style().visuals.override_text_color == Some(theme.text)
    {
        return;
    }
    let mut style = (*ctx.style()).clone();
    style.visuals = Visuals::light();
    style.override_text_style = Some(TextStyle::Monospace);
    style.visuals.override_text_color = Some(theme.text);
    style.visuals.text_cursor_width = 1.0;
    style.visuals.selection.bg_fill = theme.selection;
    style.visuals.button_frame = false;
    style.visuals.extreme_bg_color = Color32::from_rgba_unmultiplied(0x77, 0x77, 0x77, 30);
    style.visuals.widgets.inactive.rounding = Rounding::same(0.0);
    style.visuals.widgets.noninteractive.bg_fill = theme.background;
    style.visuals.widgets.noninteractive.rounding = Rounding::same(0.0);
    style.visuals.widgets.noninteractive.bg_stroke.width = 0.0;
    style.visuals.widgets.active.rounding = Rounding::same(0.0);
    style.visuals.widgets.hovered.rounding = Rounding::same(0.0);
    style.visuals.widgets.open.rounding = Rounding::same(0.0);
    ctx.set_style(style);
}
