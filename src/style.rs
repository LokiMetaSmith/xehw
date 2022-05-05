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

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
#[derive(Clone)]
pub struct Theme {
    pub text: Color32,
    pub code: Color32,
    pub error: Color32,
    pub debug_marker: Color32,
    pub comment: Color32,
    pub highlight: Color32,
    pub background: Color32,
    pub selection: Color32,
    current_item: usize,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            text: TEXT,
            background: BACKGROUND,
            code: TEXT,
            debug_marker: CYAN,
            error: RED,
            highlight: TEXT_HIGLIGHT,
            comment: COMMENT_FG,
            selection: BLUE,
            current_item: 0,
        }
    }
}

impl Theme {
    pub fn ui(&mut self, ui: &mut Ui) {
        let tab = [
            ("background", &mut self.background),
            ("text", &mut self.text),
            ("selection", &mut self.selection),
            ("code", &mut self.code),
            ("comment", &mut self.comment),
            ("error", &mut self.error),
            ("debug", &mut self.debug_marker),
            ("highlight", &mut self.highlight),
        ];
        let mut reset = false;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                for i in 0..tab.len() {
                    let text = RichText::new(tab[i].0);
                    ui.radio_value(&mut self.current_item, i, text);
                }
                reset = ui
                    .button(RichText::new("Reset Theme"))
                    .clicked();
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
