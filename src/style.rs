use eframe::egui::*;

//const RED: Color32 = Color32::from_rgb(0xCC, 0x3E, 0x28);
//const BLUE: Color32 = Color32::from_rgb(0x1E, 0x6F, 0xCC);
//const GREEN: Color32 = Color32::from_rgb(0x21, 0x66, 0x09);
//const YELLOW: Color32 = Color32::from_rgb(0xB5, 0x89, 0x00);
//const YELLOW_ACPA: Color32 = Color32::from_rgb(0xea, 0x74, 0x39);
//const PURPLE: Color32 = Color32::from_rgb(0x5C, 0x21, 0xA5);
//const CYAN: Color32 = Color32::from_rgb(0x15, 0x8C, 0x86);
//const BACKGROUND: Color32 = YELLOW_ACPA;//Color32::from_rgb(0xF2, 0xEE, 0xDE);

//const CODE_BACKGROUND: Color32 = Color32::from_rgb(0xFf, 0xf0, 0xc5);
//const TEXT: Color32 = Color32::from_rgb(0x00, 0x00, 0x00);
//const TEXT_HIGLIGHT: Color32 = Color32::from_rgb(0xD8, 0xD5, 0xC7);
//const COMMENT_FG: Color32 = Color32::from_rgb(0xAA, 0xAA, 0xAA);
//const SCROLL_BORDER: Color32 = COMMENT_FG;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
#[derive(Clone)]
pub struct Theme {
    pub text: Color32,
    pub code: Color32,
    pub code_frozen: Color32,
    pub code_background: Color32,
    pub error: Color32,
    pub debug_marker: Color32,
    pub comment: Color32,
    pub border: Color32,
    pub background: Color32,
    pub selection: Color32,
    pub selection_background: Color32,
    pub font_size: f32,
    current_item: usize,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: Color32::from_rgb(0x28, 0x28, 0x28),
            text: Color32::from_rgb(0xf0, 0xf0, 0xf0),
            selection_background: Color32::from_rgba_premultiplied(0x0, 0x80, 0x10, 0x30),
            selection: Color32::from_rgb(0x50, 0xce, 0x52),
            code: Color32::from_rgb(0x35, 0xf0, 0x00),
            code_frozen: Color32::from_rgb(0xd0, 0xc0, 0x00),
            code_background: Color32::from_rgb(0x28, 0x28, 0x28),
            comment: Color32::from_rgb(0x9a, 0x9a, 0x9a),
            error: Color32::from_rgb(0xff, 0x60, 0x60),
            debug_marker: Color32::from_rgb(0x99, 0x21, 0xaf),
            border: Color32::from_rgb(0x7a, 0xa4, 0x80),
            font_size: 14.0,
            current_item: 0,
        }
    }
}

impl Theme {
    pub fn ui(&mut self, ui: &mut Ui) {
        let tab = [
            ("Background", &mut self.background),
            ("Text", &mut self.text),
            ("Selection Background", &mut self.selection_background),
            ("Selection Stroke", &mut self.selection),
            ("Code/Data", &mut self.code),
            ("Frozen Code/Data", &mut self.code_frozen),
            ("Code Background", &mut self.code_background),
            ("Log/Label", &mut self.comment),
            ("Error", &mut self.error),
            ("Marker", &mut self.debug_marker),
            ("Border", &mut self.border),
        ];
        let mut reset = false;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                for i in 0..tab.len() {
                    let text = RichText::new(tab[i].0);
                    ui.radio_value(&mut self.current_item, i, text);
                }
                let slider = widgets::Slider::new(&mut self.font_size, 8.0..=30.0)
                    .step_by(1.0)
                    .text("Font Size");
                ui.add(slider);
                reset = ui.button(RichText::new("Reset Theme")).clicked();
            });
            ui.vertical(|ui| {
                let i = self.current_item;
                let c = *tab[i].1;
                let c32 = (c.r() as u32) << 24
                    | (c.g() as u32) << 16
                    | (c.b() as u32) << 8
                    | (c.a() as u32);
                let mut s = format!("{:x}", c32);
                ui.text_edit_singleline(&mut s);
                if s.is_empty() {
                    *tab[i].1 = Color32::TRANSPARENT;
                } else if let Ok(n) = u32::from_str_radix(&s, 16) {
                    *tab[i].1 = Color32::from_rgba_premultiplied(
                        (n >> 24) as u8,
                        (n >> 16) as u8,
                        (n >> 8) as u8,
                        (n & 0xff) as u8,
                    );
                }
                widgets::color_picker::color_picker_color32(
                    ui,
                    tab[i].1,
                    color_picker::Alpha::OnlyBlend,
                );
            });
        });
        if reset {
            *self = Theme::default();
        }
    }

    pub fn theme_ui(&mut self, ctx: &Context, open_flag: &mut bool) {
        let style = ctx.style().clone();
        let mut style2 = (*style).clone();
        style2.visuals = Visuals::light();
        ctx.set_style(style2);
        Window::new("Theme").open(open_flag).show(ctx, |ui| {
            self.ui(ui);
        });
        ctx.set_style(style);
    }
}

pub fn tune(ctx: &Context, theme: &Theme) {
    let mut style = (*ctx.style()).clone();
    style.visuals = Visuals::light();
    style.visuals.popup_shadow = Shadow::NONE;
    style.visuals.window_shadow = Shadow::NONE;
    style.override_text_style = Some(TextStyle::Monospace);
    style.override_font_id = Some(FontId::monospace(theme.font_size));
    style
        .text_styles
        .insert(TextStyle::Monospace, FontId::monospace(theme.font_size));
    //style.visuals.window_shadow.extrusion = 1.0;
    style.visuals.button_frame = false;
    style.visuals.override_text_color = Some(theme.text);

    style.visuals.selection.bg_fill = theme.selection_background;
    style.visuals.selection.stroke.color = theme.selection;

    style.visuals.extreme_bg_color = theme.border;

    style.visuals.widgets.inactive.bg_stroke.width = 1.0;
    style.visuals.widgets.inactive.bg_stroke.color = theme.border;

    style.visuals.widgets.noninteractive.bg_fill = theme.background;
    style.visuals.widgets.noninteractive.bg_stroke.width = 0.0;

    style.visuals.window_fill = theme.background;
    style.visuals.panel_fill = theme.background;

    style.visuals.widgets.active.bg_fill = theme.code_background;
    style.visuals.widgets.active.bg_stroke.color = theme.selection;

    style.visuals.widgets.hovered.bg_stroke.color = theme.selection;
    style.visuals.widgets.hovered.bg_stroke.width = 1.0;

    style.visuals.window_rounding = Rounding::same(0.0);
    style.visuals.widgets.inactive.rounding = Rounding::same(0.0);
    style.visuals.widgets.noninteractive.rounding = Rounding::same(0.0);
    style.visuals.widgets.active.rounding = Rounding::same(0.0);
    style.visuals.widgets.hovered.rounding = Rounding::same(0.0);
    style.visuals.widgets.open.rounding = Rounding::same(0.0);
    ctx.set_style(style);
}
