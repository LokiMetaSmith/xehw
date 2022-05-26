use eframe::{egui, egui::*};

pub fn interactive_canvas_pressed(i: &InputState) -> bool {
    i.key_pressed(Key::M)
}

pub fn focus_on_code_pressed(i: &InputState) -> bool {
    i.key_pressed(egui::Key::E)
}

pub fn file_open_pressed(i: &InputState) -> bool {
    i.key_pressed(egui::Key::O)
}

pub fn help_pressed(i: &InputState) -> bool {
    i.key_pressed(egui::Key::G)
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

pub fn recording_pressed(ui: &Ui) -> bool {
    ui.input().key_pressed(egui::Key::Y)
}

pub fn snapshot_pressed(ui: &Ui) -> bool {
    ui.input().key_pressed(egui::Key::S)
}

pub fn rollback_pressed(ui: &Ui) -> bool {
    ui.input().key_pressed(egui::Key::L)
}

pub fn next_pressed(ui: &Ui) -> bool {
    ui.input().key_pressed(egui::Key::N)
}   

pub fn rnext_pressed(ui: &Ui) -> bool {
    ui.input().key_pressed(egui::Key::B)
}

pub fn run_pressed(ui: &Ui) -> bool {
    ui.input().key_pressed(egui::Key::R)
}
