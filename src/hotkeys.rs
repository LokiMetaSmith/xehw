use eframe::{egui, egui::*};

pub fn interactive_canvas_pressed(ctx: &egui::Context) -> bool {
    ctx.input().modifiers.ctrl && ctx.input().modifiers.shift && ctx.input().key_pressed(Key::M)
}

pub fn scroll_view_pressed(ctx: &egui::Context, page_size: isize) -> isize {
    if ctx.input().key_pressed(egui::Key::ArrowUp) {
        -1
    } else if ctx.input().key_pressed(egui::Key::PageUp) {
        -page_size
    } else if ctx.input().key_pressed(egui::Key::ArrowDown) {
        1
    } else if ctx.input().key_pressed(egui::Key::PageDown) {
        page_size
    } else {
        0
    }
}

pub fn snapshot_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.ctrl && ui.input().modifiers.shift && ui.input().key_pressed(egui::Key::S)
}

pub fn rollback_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.ctrl && ui.input().modifiers.shift && ui.input().key_pressed(egui::Key::R)
}

pub fn help_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.ctrl && ui.input().key_pressed(egui::Key::G)
}

pub fn next_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.alt && ui.input().key_pressed(egui::Key::ArrowRight)
}

pub fn rnext_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.alt && ui.input().key_pressed(egui::Key::ArrowLeft)
}

pub fn run_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.ctrl && ui.input().key_pressed(egui::Key::R)
}

pub fn switch_to_grid_pressed(i: &InputState) -> bool {
    i.modifiers.ctrl && i.key_down(egui::Key::Num1)
}

pub fn switch_to_code_pressed(i: &InputState) -> bool {
    i.modifiers.ctrl && i.key_down(egui::Key::Num2)
}
