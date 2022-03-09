use eframe::{egui, epi};
use eframe::egui::*;
//use eframe::epi::*;

use xeh::prelude::*;

type BoxFuture = Box<dyn Future<Output = Vec<u8>>>;
/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    xs: Xstate,
    view_start: isize,
    num_rows: isize,
    num_cols: isize,
    live_code: String,
    win_size: Vec2,
    frozen_code: Vec<FrozenStr>,
    backup: Option<(Xstate, Vec<FrozenStr>)>,
    bin_future: Option<Pin<BoxFuture>>,
}

#[derive(Clone)]
struct FrozenStr {
    text: String,
    log: bool,
}

impl Default for TemplateApp {
    fn default() -> Self {
        let mut xs = Xstate::boot().unwrap();
        xs.capture_stdout();
        Self {
            xs,
            view_start: 0,
            num_rows: 10,
            num_cols: 16,
            win_size: Vec2::new(640.0, 480.0),
            live_code: String::new(),
            frozen_code: Vec::new(),
            backup: None,
            bin_future: None,
        }
    }
}

impl TemplateApp {
    fn move_view(&mut self, nrows: isize) {
        let limit = (self.current_bstr().end() / 8) as isize;
        let n = self.view_start + self.num_rows * nrows;
        self.view_start = n.max(0).min((limit - 1).max(0));
    }

    fn current_bstr(&self) -> Xbitstr {
        self.xs.get_var_value("bitstr/input").unwrap().clone().to_bitstring().unwrap()
    }
}

use std::future::{Future};
use std::task::{Poll, Context, Wake};
use std::pin::Pin;
use std::sync::Arc;

struct MyWaker();

impl Wake for MyWaker {
    fn wake(self: Arc<Self>) {
    }
}


#[cfg(target_arch = "wasm32")]
fn instant_now() -> instant::Instant {
    instant::Instant::now()
}

#[cfg(not(target_arch = "wasm32"))]
fn instant_now() -> std::time::Instant {
    std::time::Instant::now()
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "eframe template"
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        _storage: Option<&dyn epi::Storage>,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = _storage {
            *self = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        let mut snapshot_clicked = false;
        let mut rollback_clicked = false;
        let mut run_clicked = false;
        self.win_size = ctx.used_size();

        // update style
        let mut vis = Visuals::default();

        vis.override_text_color = Some(Color32::from_rgb(0xE6, 0x9F, 0x00));
        ctx.set_visuals(vis);

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Open Binary...").clicked() {
                    self.bin_future = Some(Box::pin(async {
                        let res = rfd::AsyncFileDialog::new()
                            .pick_file()
                            .await;
                        if let Some(file) = res {
                            file.read().await
                        } else {
                            Vec::new()
                        }
                    }));
                }
                if let Some(future) = self.bin_future.as_mut() {
                    let waker = Arc::new(MyWaker()).into();
                    let context = &mut Context::from_waker(&waker);
                    match Pin::new(future).poll(context) {
                        Poll::Pending => (),
                        Poll::Ready(data) => {
                            let s = Xbitstr::from(data);
                            self.xs.set_binary_input(s).unwrap();
                            self.bin_future.take();
                        }
                    }
                }
                if let Some(d) = ctx.input().raw.dropped_files.first() {
                    if let Some(data) = &d.bytes {
                        let s = Xbitstr::from(data.as_ref().to_owned());
                        self.xs.set_binary_input(s).unwrap();
                    }
                }
                run_clicked = ui.button("Run").clicked();
                snapshot_clicked = snapshot_clicked || ui.button("Snapshot").clicked() ||
                    ui.input().modifiers.ctrl && ui.input().key_released(egui::Key::G);
                if snapshot_clicked {
                    let t = instant_now();
                    self.backup = Some((self.xs.clone(), self.frozen_code.to_owned()));
                    self.xs.print(&format!("Snapshot {:0.3}s", t.elapsed().as_secs_f64()));
                }
                if self.backup.is_some() {
                    rollback_clicked = rollback_clicked || ui.button("Rollback").clicked()
                     ||  ui.input().modifiers.ctrl && ui.input().key_down(egui::Key::K);
                    if rollback_clicked && !self.live_code.trim().is_empty() {
                        if let Some((xs_old, frozen)) = self.backup.clone() {
                            self.xs = xs_old;
                            self.frozen_code = frozen;
                        }
                    }
                }
            });
        });

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            let glyph_width = ui.text_style_height(&egui::TextStyle::Monospace);
            let total_cols = self.num_cols * 3;
            let total_rows = self.num_rows + 1;
            let row_height = glyph_width;
            let size1 = Vec2::new(total_cols as f32 * glyph_width,
                total_rows as f32 * row_height);

            let _xgrid = ui.vertical(|ui|{
                ui.set_min_size(size1);
                let bs = self.current_bstr();
                let mut from = (self.view_start as usize) * 8;
                let mut it = bs.iter8_unleashed(from);
                let visible_bits = self.num_rows * self.num_cols * 8;
                let to = bs.end().min(from + visible_bits as usize);
                let start = bs.start();
                let header = format!("consumed {},{} of {},{} bytes", start / 8, start % 8,
                    bs.end() / 8, bs.end() % 8);
                ui.monospace(header);
                while from < to {
                    ui.horizontal(|ui| {
                        let addr_text = RichText::new(format!("{:06x}", from / 8)).monospace();
                        ui.add(egui::Label::new(addr_text));
                        ui.separator();
                        let xspacing = ui.spacing_mut().item_spacing;
                        ui.spacing_mut().item_spacing *= 0.5;
                        for _ in 0..self.num_cols {
                            if let Some((val, n)) = it.next() {
                                let mut text = RichText::new(&format!("{:02x}", val)).monospace();
                                if from < start {
                                    text = text.color(Color32::DARK_GRAY);
                                }
                                ui.add(Label::new(text));
                                from += n as usize;
                            } else {
                                break;
                            }
                        }
                        ui.spacing_mut().item_spacing = xspacing;
                    });
                }
            });

            //let resp = xgrid.response.interact(egui::Sense::drag());
            //let v = resp.drag_delta();
            //let index = 0;//*index = (*index as f32 + v.y).abs() as usize;

            ui.separator();
            ui.set_max_width(size1.x);

            egui::containers::ScrollArea::vertical().show(ui, |ui| {
                for i in 0.. {
                    if let Some(x) = self.xs.get_data(i) {
                        let mut s = self.xs.format_cell(x).unwrap();
                        if s.len() > total_cols as usize {
                            s.truncate(total_cols as usize - 3);
                            s.push_str("...");
                        }
                        let mut val = egui::RichText::new(s).monospace();
                        if i < self.xs.data_depth() {
                            val = val.background_color(Color32::DARK_BLUE);
                        }
                        ui.horizontal(|ui| {
                            ui.monospace(format!("{:4}:", i));
                            ui.label(val);
                        });
                    } else {
                        break;
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            let mut code_has_focus = false;

            ui.collapsing("Help", |ui| {
                ui.label("Drag and Drop file or click \"Open Binary...\" to start workspace");
                ui.label("Click \"Run\" or Ctrl+Return to run code snippet");
            });

            egui::containers::ScrollArea::vertical()
                     .stick_to_bottom().show(ui, |ui| {
                
                for s in self.frozen_code.iter() {
                    let richtext = RichText::new(s.text.to_owned())
                        .monospace()
                        .color(if s.log {egui::Color32::WHITE} else {egui::Color32::GRAY});
                    let fl = Label::new(richtext);
                    ui.add(fl);
                }
                let code = egui::TextEdit::multiline(&mut self.live_code)
                    .desired_width(f32::INFINITY)
                    .code_editor()
                    .id(Id::new("code"));
                let res = ui.add(code);
                code_has_focus = res.has_focus();
            });
            
            if !code_has_focus {
                if ctx.input().key_pressed(egui::Key::ArrowUp) {
                    self.move_view(-1);
                }
                if ctx.input().key_pressed(egui::Key::PageUp) {
                    self.move_view(-self.num_rows);
                }
                if ctx.input().key_pressed(egui::Key::ArrowDown) {
                    self.move_view(1);
                }
                if ctx.input().key_pressed(egui::Key::PageDown) {
                    self.move_view(self.num_rows);
                }
            }

            run_clicked = run_clicked || ui.input().key_down(egui::Key::Enter) &&
                            ui.input().modifiers.ctrl;
            if run_clicked && !self.live_code.trim().is_empty() {
                let t = instant_now();
                let res = self.xs.eval(&self.live_code);
                self.frozen_code.push(FrozenStr { text: self.live_code.trim_end().to_owned(), log: false });
                    if let Some(log) = self.xs.console() {
                        self.frozen_code.push(FrozenStr { text: log.take(), log: true });
                    }
                if res.is_ok() {
                    let text = format!("OK {:0.3}s", t.elapsed().as_secs_f64());
                    self.frozen_code.push(FrozenStr { text, log: true });
                }
                self.live_code.clear();
            }
        });

    }
}
