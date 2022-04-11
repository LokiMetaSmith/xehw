use eframe::{egui, egui::*};

pub fn interactive_canvas_pressed(ctx: &egui::Context) -> bool {
    ctx.input().modifiers.ctrl
        && ctx.input().modifiers.shift
        && ctx.input().key_pressed(Key::M)
}

pub fn scroll_view(ctx: &egui::Context, page_size: isize) -> isize {
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

// "^ + ⇧ + S"

pub fn snapshot_hotkey_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.ctrl &&
    ui.input().modifiers.shift &&
    ui.input().key_pressed(egui::Key::S)
}

pub fn rollback_hotkey_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.ctrl &&
    ui.input().modifiers.shift &&
    ui.input().key_pressed(egui::Key::R)
}

pub fn help_hotkey_pressed(ui: &Ui) -> bool {
    ui.input().modifiers.ctrl &&
    ui.input().key_pressed(egui::Key::G)
}
