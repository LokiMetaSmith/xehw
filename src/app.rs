use eframe::{egui, epi};
use egui::*;
use xeh::prelude::*;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct TemplateApp {
    repl: String,
    log: String,
    xs: Xstate,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            repl: String::new(),
            log: String::new(),
            xs: Xstate::new().unwrap(),
        }
    }
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "test"
    }

    /// Called by the framework to load old app state (if any).
    #[cfg(feature = "persistence")]
    fn load(&mut self, storage: &dyn epi::Storage) {
        *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
    }

    /// Called by the frame work to save state before shutdown.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let TemplateApp { repl, log, xs } = self;

        egui::TopPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                egui::menu::menu(ui, "File", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("REPL:");
            ui.separator();
            ui.add(
                TextEdit::multiline(log)
                    .text_style(TextStyle::Monospace)
                    .desired_rows(25)
            );
            ui.add(TextEdit::singleline(repl).text_style(TextStyle::Monospace));
            let res = ui.button("Run");
            if res.clicked() {
                log.push_str(repl);
                log.push_str("\n");
                let _ = xs.interpret(repl);
                log.push_str(xs.display_str());
                xs.display_clear();
                repl.clear();
            }
        });
    }
}
