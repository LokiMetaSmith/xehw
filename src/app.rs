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

        egui::SidePanel::left("stack", 200.0)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading("Stack");
                ui.separator();
                ui.set_min_width(200.0);
                for i in 0.. {
                    if let Some(val) = xs.get_data(i) {
                        ui.label(format!("{:1?}", val));
                    } else {
                        break;
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::multiline(log)
                        .text_style(TextStyle::Monospace)
                        .desired_rows(25)
                );
                ui.add(
                    Label::new("aa\nbb\n")
                        .wrap(true)
                        .monospace()
                )
            });
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
