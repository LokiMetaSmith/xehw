use eframe::egui::*;

fn set_no_rounding(r: &mut Rounding)
{
    r.ne = 0.0;
    r.nw = 0.0;
    r.se = 0.0;
    r.sw = 0.0;
}

pub fn tune(ctx: &Context, font: &FontId) {
    // update style
    let mut style = (*ctx.style()).clone();
    style.override_font_id = Some(font.clone());
    style.visuals.override_text_color = Some(Color32::from_rgb(0xE6, 0x9F, 0x00));
    style.visuals.widgets.noninteractive.bg_fill = Color32::from_rgba_premultiplied(0, 0, 0, 90);
    //style.visuals.code_bg_color = Color32::TRANSPARENT;
    //style.visuals.widgets.active.bg_fill = Color32::TRANSPARENT;
    style.visuals.extreme_bg_color = Color32::from_rgba_premultiplied(0, 0, 0, 50);
    set_no_rounding(&mut style.visuals.widgets.inactive.rounding);
    set_no_rounding(&mut style.visuals.widgets.noninteractive.rounding);
    //style.visuals.widgets.noninteractive.fg_stroke.color = Color32::from_rgb(0xE6, 0x9F, 0x00);
    ctx.set_style(style);
}
