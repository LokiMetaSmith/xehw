use eframe::{egui, egui::*};

pub fn interactive_canvas_pressed(ctx: &egui::Context) -> bool {
    ctx.input().modifiers.ctrl
        && ctx.input().modifiers.shift
        && ctx.input().key_pressed(Key::M)
}
