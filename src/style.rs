use eframe::{egui::*, egui::style::*};

pub const WINDOW_BG_FILL: Color32 = Color32::from_rgb(0x10, 0x10, 0x10);
pub const TEXT_FG: Color32 = Color32::from_rgb(0xE6, 0x9F, 0x00);
pub const HEX_FG1: Color32 = Color32::from_rgb(0x15, 0xac, 0x0);
pub const HEX_FG2: Color32 = Color32::from_rgb(0x15, 0x5c, 0x0);
pub const LOG_FG: Color32 = Color32::from_rgb(0x5a, 0x5a, 0x5a);

pub fn code(text: String, is_error: bool) -> RichText {
    let r = RichText::new(text).monospace();
    if is_error {
        r.background_color(Color32::RED)
    } else {
        r
    }
}

pub fn log(text: String) -> RichText {
    RichText::new(text).monospace().color(LOG_FG)
}

pub fn hex_addr_rich(text: String) -> RichText {
    RichText::new(text).monospace().color(HEX_FG2)
}

pub fn hex_data_rich(text: String, consumed: bool) -> RichText {
    let r = RichText::new(text).monospace().color(HEX_FG1);
    if consumed {
        r.background_color(Color32::DARK_GRAY)
    } else {
        r
    }
}

pub fn tune(ctx: &Context, font: &FontId) {
    let mut style = (*ctx.style()).clone();
    style.override_font_id = Some(font.clone());
    style.visuals = Visuals::light();
    //style.override_text_style = Some(TextStyle::Monospace);
    style.visuals.override_text_color = Some(TEXT_FG);
    style.visuals.widgets.noninteractive.bg_fill = WINDOW_BG_FILL;
    //style.visuals.widgets.noninteractive.bg_stroke.width = 1.0;
    // style.visuals.widgets.active.fg_stroke.color = Color32::RED;
    // style.visuals.widgets.inactive.fg_stroke.color = Color32::RED;
    // style.visuals.widgets.hovered.fg_stroke.color = Color32::RED;

    style.visuals.button_frame = false;
    
    style.visuals.extreme_bg_color = Color32::from_rgba_unmultiplied(0x77, 0x77, 0x77, 30);
    style.visuals.widgets.inactive.rounding = Rounding::same(0.0);
    style.visuals.widgets.noninteractive.rounding = Rounding::same(0.0);
    ctx.set_style(style);
}
