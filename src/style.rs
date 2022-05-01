use eframe::egui::*;

const RED: Color32 = Color32::from_rgb(0xCC, 0x3E, 0x28);
const BLUE: Color32 = Color32::from_rgb(0x1E, 0x6F, 0xCC);
pub const GREEN: Color32 = Color32::from_rgb(0x21, 0x66, 0x09);
const YELLOW: Color32 = Color32::from_rgb(0xB5, 0x89, 0x00);
// const PURPLE: Color32 = Color32::from_rgb(0x5C, 0x21, 0xA5);
// const CYAN: Color32 = Color32::from_rgb(0x15, 0x8C, 0x86);

pub const TEXT_BG: Color32 = Color32::from_rgb(0xF2, 0xEE, 0xDE);
pub const TEXT_FG: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
pub const TEXT_HIGLIGHT: Color32 = Color32::from_rgb(0xD8, 0xD5, 0xC7);
pub const COMMENT_FG: Color32 = Color32::from_rgb(0xAA, 0xAA, 0xAA);
pub const HEX_DATA_FG: Color32 = TEXT_FG;
pub const HEX_ADDR_FG: Color32 = COMMENT_FG;
pub const CODE_FG: Color32 = TEXT_FG;
pub const CODE_ERR_BG: Color32 = RED;
pub const CODE_DBG_BG: Color32 = YELLOW;

pub fn hex_addr_rich(text: String) -> RichText {
    RichText::new(text).monospace().color(HEX_ADDR_FG)
}

pub fn hex_data_rich(text: String, consumed: bool) -> RichText {
    let r = RichText::new(text).monospace().color(HEX_DATA_FG);
    if consumed {
        r.background_color(TEXT_HIGLIGHT)
    } else {
        r
    }
}

pub fn tune(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = Visuals::light();
    style.override_text_style = Some(TextStyle::Monospace);
    style.visuals.override_text_color = Some(TEXT_FG);
    style.visuals.widgets.noninteractive.bg_fill = TEXT_BG;
    //style.visuals.widgets.noninteractive.bg_stroke.width = 1.0;
    // style.visuals.widgets.active.fg_stroke.color = Color32::RED;
    // style.visuals.widgets.inactive.fg_stroke.color = Color32::RED;
    // style.visuals.widgets.hovered.fg_stroke.color = Color32::RED;
    style.visuals.text_cursor_width = 1.0;
    style.visuals.selection.bg_fill = BLUE;
    style.visuals.button_frame = false;
    style.visuals.extreme_bg_color = Color32::from_rgba_unmultiplied(0x77, 0x77, 0x77, 30);
    style.visuals.widgets.inactive.rounding = Rounding::same(0.0);
    style.visuals.widgets.noninteractive.rounding = Rounding::same(0.0);
    style.visuals.widgets.noninteractive.bg_stroke.width = 0.0;
    style.visuals.widgets.active.rounding = Rounding::same(0.0);
    style.visuals.widgets.hovered.rounding = Rounding::same(0.0);
    style.visuals.widgets.open.rounding = Rounding::same(0.0);
    ctx.set_style(style);
}
