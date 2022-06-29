use eframe::egui::*;
use eframe::{egui, epi};

use crate::hotkeys;
use crate::style::*;
use crate::{canvas::*, layouter};
use std::fmt::Write;
use xeh::prelude::*;

#[cfg(target_arch = "wasm32")]
type Instant = instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
type Instant = std::time::Instant;

type BoxFuture = Box<dyn Future<Output = Option<Vec<u8>>>>;

#[derive(PartialEq)]
enum HelpMode {
    Hotkeys,
    Index,
    QuickRef,
}

struct Help {
    is_open: bool,
    mode: HelpMode,
    words: Vec<(Xstr, Cell)>,
    filter: String,
    live_cursor: Option<String>,
    follow_cursor: bool,
}

pub struct TemplateApp {
    pub xs: Xstate,
    view_pos: usize,
    num_rows: usize,
    num_cols: usize,
    live_code: String,
    trial_code: Option<Xstr>,
    frozen_code: Vec<FrozenStr>,
    last_dt: Option<f64>,
    canvas: Canvas,
    canvas_open: bool,
    debug_token: Option<TokenLocation>,
    rdebug_enabled: bool,
    calc_limit: usize,
    snapshot: Option<(Xstate, Vec<FrozenStr>)>,
    bin_future: Option<Pin<BoxFuture>>,
    input_binary: Option<Xbitstr>,
    focus_on_code: bool,
    bytecode_open: bool,
    vars_open: bool,
    vars_boot_len: usize,
    goto_open: bool,
    goto_text: String,
    goto_old_pos: Option<usize>,
    help: Help,
    theme: Theme,
    theme_editor: bool,
    example_request: Option<(&'static str, &'static [u8])>,
}

#[derive(Clone)]
enum FrozenStr {
    Code(Xsubstr),
    Log(String),
    TrialLog(String),
}

const SECTION_TAG: Cell = Cell::Str(arcstr::literal!("section"));
const STACK_TAG: Cell = Cell::Str(arcstr::literal!("stack-comment"));
const EXAMPLE_TAG: Cell = Cell::Str(arcstr::literal!("example"));

impl Default for TemplateApp {
    fn default() -> Self {
        let xs = Self::xs_respawn();
        let vars_boot_len = xs.var_list().len();
        Self {
            xs,
            view_pos: 0,
            num_rows: 10,
            num_cols: 8,
            live_code: String::new(),
            frozen_code: Vec::new(),
            trial_code: Some(Xstr::new()),
            last_dt: None,
            debug_token: None,
            canvas: Canvas::new(),
            canvas_open: false,
            calc_limit: 10_000_000,
            snapshot: None,
            bin_future: None,
            input_binary: None,
            focus_on_code: true,
            rdebug_enabled: false,
            bytecode_open: false,
            vars_open: false,
            vars_boot_len,
            goto_open: false,
            goto_text: String::new(),
            goto_old_pos: None,
            help: Help {
                is_open: false,
                mode: HelpMode::Hotkeys,
                words: Vec::new(),
                filter: String::new(),
                live_cursor: None,
                follow_cursor: false,
            },
            theme: Theme::default(),
            theme_editor: false,
            example_request: None,
        }
    }
}

impl TemplateApp {
    fn xs_respawn() -> Xstate {
        let mut xs = Xstate::boot().unwrap();
        xs.intercept_stdout(true);
        xeh::d2_plugin::load(&mut xs).unwrap();
        xs
    }

    fn load_help(&mut self) {
        self.xs
            .eval(include_str!("../../xeh/docs/help.xeh"))
            .unwrap();
        let words = self
            .xs
            .word_list()
            .into_iter()
            .filter_map(|name| {
                let s = self.xs.help_str(&name).unwrap_or(&NIL);
                if s != &NIL {
                    Some((name, s.clone()))
                } else {
                    None
                }
            })
            .collect();
        self.help.words = words;
    }

    fn move_view(&mut self, nrows: isize) {
        let n = (self.view_pos as isize + (nrows * self.num_cols as isize * 8)).max(0) as usize;
        self.view_pos = n.min(self.current_bstr().end());
    }

    fn current_bstr(&self) -> &Xbitstr {
        self.xs.get_var_value("input").unwrap().bitstr().unwrap()
    }

    fn current_offset(&self) -> usize {
        self.xs.get_var_value("offset").unwrap().to_usize().unwrap()
    }

    fn binary_dropped(&mut self, s: Xbitstr) {
        self.input_binary = Some(s.clone());
        self.reload_state();
    }

    fn collect_frozen_code(&self) -> String {
        self.frozen_code.iter().fold(String::new(), |mut buf, x| {
            match x {
                FrozenStr::Code(s) => {
                    buf.push_str(s);
                    buf.push_str("\n");
                }
                _ => (),
            }
            buf
        })
    }

    fn reload_state(&mut self) {
        let buf = self.collect_frozen_code();
        self.xs = Self::xs_respawn();
        self.canvas = Canvas::new();
        self.live_code = buf;
        self.frozen_code.clear();
        if let Some(bin) = &self.input_binary {
            let _ = self.xs.set_binary_input(bin.clone());
        }
        self.vars_boot_len = self.xs.var_list().len();
        if self.is_trial() {
            self.trial_code = Some(Xstr::new());
            self.snapshot();
        } else {
            self.snapshot = None;
        }
    }

    fn snapshot(&mut self) {
        self.snapshot = Some((self.xs.clone(), self.frozen_code.to_owned()));
    }

    fn rollback(&mut self) {
        let old_state = if self.is_trial() {
            self.snapshot.clone()
        } else {
            self.snapshot.take()
        };
        if let Some((xs, frozen)) = old_state {
            self.xs = xs;
            self.frozen_code = frozen;
        }
    }

    fn hex_offset_str(&self, offset: usize, _end: usize) -> String {
        let a = offset / 8;
        let b = offset % 8;
        if b == 0 {
            format!("{:06x}", a)
        } else {
            format!("{:06x}.{}", a, b)
        }
    }

    fn menu_text(&self, name: &str) -> RichText {
        RichText::new(name).monospace().color(self.theme.text)
    }

    fn editor(&mut self, ctx: &egui::Context) {
        let mut snapshot_clicked = false;
        let mut rollback_clicked = false;
        let mut run_clicked = false;
        let mut next_clicked = false;
        let mut rnext_clicked = false;
        let mut repl_clicked = false;
        let mut trial_clicked = false;
        let mut help_clicked = false;
        let mut canvas_clicked = false;
        let mut open_clicked = false;
        let mut goto_clicked = false;
        let mut vars_clicked = false;
        let mut unfreeze_clicked = false;
        let win_rect = ctx.available_rect();

        self.theme.theme_ui(ctx, &mut self.theme_editor);

        egui::Window::new("Variables")
            .open(&mut self.vars_open)
            .default_pos(pos2(win_rect.right() - 200.0, 200.0))
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                let lst = self.xs.var_list();
                let n = lst.len().checked_sub(self.vars_boot_len).unwrap_or(0);
                for (name, val) in lst.iter().rev().take(n) {
                    ui.horizontal(|ui| {
                        ui.colored_label(self.theme.text, name.to_string());
                        let s = self.xs.format_cell(val).unwrap_or_else(|_| "Can't display value".to_string());
                        ui.colored_label(self.theme.code_frozen, s);
                    });
                }
            });

        egui::Window::new("Bytecode")
            .open(&mut self.bytecode_open)
            .default_pos(pos2(200.0, 400.0))
            .vscroll(true)
            .show(ctx, |ui| {
                //ctx.style_ui(ui);
                ui.label(format!("ip={}", self.xs.ip()));
                ui.vertical(|ui| {
                    for (ip, op) in self.xs.bytecode().iter().enumerate() {
                        let optext = self.xs.fmt_opcode(ip, op);
                        let mut rich = RichText::new(format!("{:05x}: {}", ip, optext))
                            .monospace()
                            .color(self.theme.text);
                        if ip == self.xs.ip() {
                            rich = rich.background_color(self.theme.border);
                        }
                        ui.label(rich);
                    }
                });
            });

        Window::new("Canvas")
            .open(&mut self.canvas_open)
            .default_size(self.canvas.size())
            .resizable(true)
            .show(ctx, |ui| {
                self.canvas.ui(ui, &self.theme);
            });

        let mut is_goto_open = self.goto_open;
        Window::new("Go To...")
            .open(&mut is_goto_open)
            .show(ctx, |ui| {
                ui.style_mut().visuals.extreme_bg_color = self.theme.code_background;
                let mut cancel_clicked = ui.input().key_pressed(Key::Escape);
                let mut ok_clicked = ui.input().key_pressed(Key::Enter);
                ui.text_edit_singleline(&mut self.goto_text).request_focus();
                ui.style_mut().visuals.extreme_bg_color = self.theme.border;
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        ok_clicked = true;
                    }
                    if ui.button("Close").clicked() {
                        cancel_clicked = true;
                    }
                });
                if cancel_clicked {
                    self.goto_open = false;    
                    return;
                }
                let evalgoto = |s: &str| {
                    if s.is_empty() {
                        return Err(Xerr::ExpectingLiteral);
                    }
                    let mut xs = Xstate::core()?;
                    xs.eval(s.into())?;
                    xs.pop_data()?.to_xint()
                };
                match evalgoto(&self.goto_text) {
                    Ok(n) => {
                        let bs = self.current_bstr().clone();
                        if n < 0 {
                            self.view_pos = bs.end().wrapping_sub(n.abs() as usize);
                        } else {
                            self.view_pos = bs.end().min(n as usize);
                        }
                        if ok_clicked {
                            self.goto_text.clear();
                            self.goto_open = false;
                            self.goto_old_pos.take();
                        }
                        ui.colored_label(self.theme.text, format!("{}", n));
                    }
                    Err(e) => {
                        ui.colored_label(self.theme.error, format!("{}", e));
                    }
                }
            });
        if !is_goto_open || !self.goto_open {
            self.goto_open = false;
            self.goto_text.clear();
            if let Some(pos) = self.goto_old_pos.take() {
                self.view_pos = pos;
            }
        }

        let help_pos = pos2(win_rect.width() * 0.25, win_rect.height() * 0.25);
        egui::Window::new("Help")
            .open(&mut self.help.is_open)
            .default_pos(help_pos)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.help.mode,
                        HelpMode::Hotkeys,
                        RichText::new("Hotkeys").heading(),
                    );
                    ui.selectable_value(
                        &mut self.help.mode,
                        HelpMode::Index,
                        RichText::new("Index").heading(),
                    );
                    ui.selectable_value(
                        &mut self.help.mode,
                        HelpMode::QuickRef,
                        RichText::new("Quick Reference").heading(),
                    );
                });
                match self.help.mode {
                    HelpMode::Hotkeys => {
                        let add = |ui: &mut Ui, text, combo| {
                            ui.horizontal(|ui| {
                                ui.colored_label(self.theme.text, text);
                                ui.colored_label(self.theme.selection, combo);
                            });
                        };
                        ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                ui.heading("Hotkeys");
                                add(ui, "Open binary file...", "(Esc, O)");
                                add(ui, "Program - Run", "(Esc, R)");
                                add(ui, "Program - Snapshot", "(Esc, S)");
                                add(ui, "Program - Rollback", "(Esc, L)");
                                add(ui, "Debugger - Next", "(Esc, B)");
                                add(ui, "Debugger - Reverse Next", "(Esc, N)");
                                add(ui, "Debugger - Enable Reverse Next", "(Esc, Y)");
                                add(ui, "Hex - Scroll Up", "(Esc, Arrow Up)");
                                add(ui, "Hex - Scroll Down", "(Esc, Arrow Down)");
                                add(ui, "Hex - Go To...", "(Esc, G)");
                                add(ui, "Focus on Code", "(Esc, E)");
                                add(ui, "Canvas - Show", "(Esc, M)");
                                add(ui, "Help - Show", "(Esc, H)");
                                ui.heading("Mouse");
                                ui.colored_label(
                                    self.theme.text,
                                    "Open binary file with Drag and Drop",
                                );
                            });
                    }
                    HelpMode::Index => {
                        ui.horizontal(|ui| {
                            ui.style_mut().visuals.extreme_bg_color = self.theme.code_background;
                            let edit = TextEdit::singleline(&mut self.help.filter);
                            ui.add_enabled(!self.help.follow_cursor, edit);
                            ui.style_mut().visuals.extreme_bg_color = self.theme.border;
                            ui.checkbox(&mut self.help.follow_cursor, "Follow Editor Cursor");
                        });
                        let filter = if self.help.follow_cursor {
                            self.help
                                .live_cursor
                                .as_ref()
                                .map(|s| s.as_str())
                                .unwrap_or("")
                        } else {
                            self.help.filter.as_str()
                        }
                        .trim();
                        self.help.words.sort_by(|a, b| a.0.cmp(&b.0));
                        let mut new_filter = None;
                        ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                for (word, help) in &self.help.words {
                                    let section = help
                                        .get_tagged(&SECTION_TAG)
                                        .and_then(|s| s.str().ok())
                                        .unwrap_or("");
                                    if filter.is_empty()
                                        || word.as_str().starts_with(filter)
                                        || section.starts_with(filter)
                                    {
                                        ui.horizontal(|ui| {
                                            let heading = RichText::new(word.as_str())
                                                .color(self.theme.selection);
                                            ui.monospace(heading);
                                            if let Some(t) = help.get_tagged(&STACK_TAG) {
                                                ui.colored_label(
                                                    self.theme.comment,
                                                    format!(" # ( {:?} ) ", t),
                                                );
                                            }
                                            if !section.is_empty() {
                                                let name = RichText::new(section)
                                                    .color(self.theme.selection)
                                                    .underline();
                                                if ui.button(name).clicked() {
                                                    new_filter = Some(section.to_string());
                                                }
                                            }
                                        });
                                        if let Ok(s) = help.str() {
                                            ui.label(s);
                                        }
                                        if let Some(v) = help.tag().and_then(|t| t.vec().ok()) {
                                            for i in v.iter() {
                                                if i.tag() == Some(&EXAMPLE_TAG) {
                                                    let s = i.str().ok().unwrap_or("");
                                                    let example = RichText::new(s)
                                                        .color(self.theme.code_frozen)
                                                        .background_color(
                                                            self.theme.code_background,
                                                        );
                                                    ui.separator();
                                                    ui.horizontal(|ui| {
                                                        ui.separator();
                                                        ui.monospace(example);
                                                    });
                                                }
                                            }
                                        }
                                        ui.separator();
                                    }
                                }
                            });
                        if let Some(s) = new_filter {
                            self.help.filter = s;
                        }
                    }
                    HelpMode::QuickRef => {
                        ui.hyperlink_to(
                            "Github README.md",
                            "https://anykey111.github.io/README.md",
                        );
                        ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                ui.style_mut().visuals.extreme_bg_color =
                                    self.theme.code_background;
                                ui.monospace(include_str!("../../xeh/README.md"));
                            });
                    }
                }
            }); //help

        let mut rnext_enabled = false;
        let rollback_enabled = self.snapshot.is_some() && !self.is_trial();
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button(self.menu_text("Open...")).clicked() {
                        open_clicked = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    if ui.button(self.menu_text("Go To...")).clicked() {
                        goto_clicked = true;
                        ui.close_menu();
                    }
                    if ui.button(self.menu_text("Variables")).clicked() {
                        vars_clicked = true;
                        ui.close_menu();
                    }
                    if ui.button(self.menu_text("Canvas")).clicked() {
                        canvas_clicked = true;
                        ui.close_menu();
                    }
                    if ui.button(self.menu_text("Theme")).clicked() {
                        self.theme_editor = !self.theme_editor;
                        ui.close_menu();
                    }

                });
            run_clicked = ui.button(self.menu_text("🚀Run")).clicked();
                snapshot_clicked = ui
                    .add_enabled(!self.is_trial(), Button::new(self.menu_text("💾Snapshot"))
                    .wrap(false))
                    .clicked();
                rollback_clicked = ui
                    .add_enabled(rollback_enabled, Button::new(self.menu_text("🔨Rollback"))
                    .wrap(false))
                    .clicked();
                let unfreeze_enabled = self.frozen_code.iter().any(|c| match c { FrozenStr::Code(_) => true, _ => false });
                unfreeze_clicked = ui.add_enabled(unfreeze_enabled, Button::new(self.menu_text("🔥Unfreeze"))
                .wrap(false))
                .clicked();
                let mut trial_mode = self.trial_code.is_some();
                if ui.checkbox(&mut trial_mode, "TRIAL").changed() {
                    repl_clicked = !trial_mode;
                    trial_clicked = trial_mode;
                }
                ui.checkbox(&mut self.rdebug_enabled, "RDebug");
                self.xs.set_recording_enabled(self.rdebug_enabled);
                if self.xs.is_recording() {
                    rnext_enabled = self.rlog_size().map(|n| n > 0).unwrap_or(false);
                    rnext_clicked = ui
                        .add_enabled(rnext_enabled, Button::new(self.menu_text("↩RNext")))
                        .clicked();
                    next_clicked = ui
                        .add_enabled(self.xs.is_running(), Button::new(self.menu_text("↪Next")))
                        .clicked();
                }
                ui.menu_button("Help", |ui| {
                    if ui.button(self.menu_text("Hotkeys")).clicked() {
                        help_clicked = true;
                        self.help.mode = HelpMode::Hotkeys;
                        ui.close_menu();
                    }
                    if ui.button(self.menu_text("Word Index")).clicked() {
                        help_clicked = true;
                        self.help.mode = HelpMode::Index;
                        ui.close_menu();
                    }
                    if ui.button(self.menu_text("Quick Ref")).clicked() {
                        help_clicked = true;
                        self.help.mode = HelpMode::QuickRef;
                        ui.close_menu();
                    }
                    ui.hyperlink_to("Youtube", "https://www.youtube.com/channel/UCYTeJIi6aLE9rS7s_QOto3w");
                    ui.add_enabled(false, Label::new("Examples:"));
                    self.menu_examples(ui);

                });
            });
        }); // top panel

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            let ncols = self.num_cols * 4 + 10;
            let total_rows = self.num_rows;
            let text_style = TextStyle::Monospace;
            let font = text_style.resolve(ui.style());
            let glyph_width = ui.fonts().glyph_width(&font, '0');
            let row_height = ui.fonts().row_height(&font);
            let size1 = Vec2::new(ncols as f32 * glyph_width, total_rows as f32 * row_height);
            ui.set_min_width(size1.x);

            let xgrid = ui.vertical(|ui| {
                let offset = self.current_offset();
                let mut from = self.view_pos;
                let bs = self.current_bstr().seek(from).unwrap_or_default();
                let mut it = bs.iter8();
                let visible_bits = self.num_rows * self.num_cols * 8;
                let to = bs.end().min(from + visible_bits);
                ui.spacing_mut().item_spacing = vec2(0.0, 2.0);
                ui.spacing_mut().interact_size = vec2(0.0, 0.0);

                ui.horizontal(|ui| {
                    let hdr_text = self.hex_offset_str(offset, bs.end());
                    let hdr = Label::new(
                        RichText::new(hdr_text)
                            .color(self.theme.comment)
                            .underline(),
                    )
                    .sense(Sense::click());
                    if ui.add(hdr).clicked() {
                        self.view_pos = offset;
                    }
                    let end = self.hex_offset_str(bs.end(), bs.end());
                    ui.colored_label(self.theme.comment, format!(" of {}",end));
                });

                for _ in 0..self.num_rows {
                    let mut addr_text = self.hex_offset_str(from, bs.end());
                    if from >= to {
                        ui.colored_label(self.theme.comment, addr_text);
                        continue;
                    }
                    ui.horizontal(|ui| {
                        addr_text.push_str(" ");
                        ui.colored_label(self.theme.comment, addr_text);
                        let mut ascii = String::new();
                        ascii.push_str("  ");
                        for i in 0..self.num_cols {
                            if let Some((val, n)) = it.next() {
                                let hex_data = RichText::new(format!(" {:02x}", val)).color(
                                    if from < offset {
                                        self.theme.code_frozen
                                    } else {
                                        self.theme.code
                                    },
                                );
                                ui.label(hex_data);
                                let c = xeh::bitstr_ext::byte_to_dump_char(val);
                                ascii.push(c);
                                from += n as usize;
                            } else {
                                let n = (self.num_cols - i) as usize;
                                let mut pad = String::with_capacity(n * 3);
                                for _ in 0..n {
                                    pad.push_str("   ");
                                    ascii.push(' ');
                                }
                                ui.colored_label(self.theme.comment, pad);
                                break;
                            }
                        }
                        ui.colored_label(self.theme.comment, ascii);
                    });
                }
            });

            let resp = xgrid.response.interact(egui::Sense::drag());
            let v = resp.drag_delta();
            self.move_view(v.y as isize);

            ui.colored_label(self.theme.comment, "Stack:");

            egui::containers::ScrollArea::vertical().show(ui, |ui| {
                ui.set_min_width(size1.x);
                ui.set_max_width(size1.x);
                for i in 0.. {
                    if let Some(x) = self.xs.get_data(i) {
                        let mut s = self.xs.format_cell(x).unwrap();
                        if s.chars().count() > ncols as usize {
                            s.truncate(ncols as usize - 3);
                            s.push_str("...");
                        }
                        let color = if i < self.xs.data_depth() {
                            self.theme.code
                        } else {
                            self.theme.code_frozen
                        };
                        ui.horizontal(|ui| {
                            ui.colored_label(self.theme.comment, format!("{:6}:", i));
                            ui.colored_label(color, s);
                        });
                    } else {
                        break;
                    }
                }
            });
        });

        let esc_pressed = ctx.input().key_down(Key::Escape);
        let mut live_has_focus = false;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::containers::ScrollArea::vertical()
                .stick_to_bottom()
                .show(ui, |ui| {
                    let old_spacing = ui.spacing_mut().item_spacing;
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    for x in self.frozen_code.iter() {
                        match x {
                            FrozenStr::Log(s) | FrozenStr::TrialLog(s) => {
                                ui.colored_label(self.theme.comment, s.trim_end().to_string());
                            }
                            FrozenStr::Code(s) => {
                                if let Some(loc) = self.xs.last_err_location() {
                                    if Xsubstr::shallow_eq(&loc.whole_line, s) {
                                        let err = self.xs.last_error().unwrap();
                                        self.ui_error_highlight(ui, loc, err);
                                        continue;
                                    }
                                }
                                if let Some(loc) = self.debug_token.as_ref() {
                                    if Xsubstr::shallow_eq(&loc.whole_line, s) {
                                        self.ui_debugger_highlight(ui, loc);
                                        continue;
                                    }
                                }
                                ui.colored_label(self.theme.code_frozen, s.as_str());
                            }
                        }
                    }
                    ui.spacing_mut().item_spacing = old_spacing;
                    let show_trial_error = self.is_trial()
                        && self.xs.last_error().is_some()
                        && self.live_code.trim().len() > 0;
                    {
                        // mini status
                        if let Some(err) = self.xs.last_error() {
                            if show_trial_error {
                                let s = format!("ERROR: {}", err);
                                ui.colored_label(self.theme.error, s);
                            }
                        } else {
                            let mut s = String::new();
                            write!(s, "OK ").unwrap();
                            if let Some(secs) = self.last_dt {
                                write!(s, "{:.3}s", secs).unwrap();
                            }
                            if let Some(n) = self.rlog_size() {
                                write!(s, " RLOG {}", n).unwrap();
                            }
                            ui.colored_label(self.theme.comment, s);
                        }
                    }
                    let mut errtok = None;
                    let mut dbgtok = None;
                    if show_trial_error {
                        match (self.xs.last_err_location(), &self.trial_code) {
                            (Some(loc), Some(code)) if loc.token.parent() == code => {
                                errtok = Some(loc.token.clone())
                            }
                            _ => (),
                        }
                    }
                    if self.is_trial() {
                        match (&self.debug_token, &self.trial_code) {
                            (Some(loc), Some(code)) if loc.token.parent() == code => {
                                dbgtok = Some(loc.token.clone());
                            }
                            _ => (),
                        }
                    }
                    let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                        let font_id = TextStyle::Monospace.resolve(ui.style());
                        let j = layouter::code_layouter(
                            text,
                            errtok.as_ref(),
                            dbgtok.as_ref(),
                            &font_id,
                            wrap_width,
                            &self.theme.clone(),
                        );
                        ui.fonts().layout_job(j)
                    };
                    let code_id = Id::new("live");
                    ui.style_mut().visuals.extreme_bg_color = self.theme.code_background;
                    let code = egui::TextEdit::multiline(&mut self.live_code)
                        .desired_rows(1)
                        .desired_width(f32::INFINITY)
                        .code_editor()
                        .margin(vec2(4.0, 2.0))
                        .id(code_id)
                        .layouter(&mut layouter)
                        .show(ui);
                    ui.style_mut().visuals.extreme_bg_color = self.theme.border;
                    let word = layouter::word_under_cursor(
                        &self.live_code,
                        code.cursor_range.map(|c| c.primary.ccursor.index),
                    );
                    self.help.live_cursor = word;
                    if esc_pressed {
                        code.response.surrender_focus();
                    } else if hotkeys::focus_on_code_pressed(&ctx.input()) || self.focus_on_code {
                        code.response.request_focus();
                        self.focus_on_code = false;
                    }
                    live_has_focus = code.response.has_focus();
                });

            let has_some_code = !self.live_code.trim().is_empty();
            if !live_has_focus && !self.help.is_open && !self.goto_open {
                let n = hotkeys::scroll_view_pressed(ctx, self.num_cols as isize);
                if n != 0 {
                    self.move_view(n);
                }
                if hotkeys::recording_pressed(ui) {
                    self.rdebug_enabled = !self.rdebug_enabled;
                }
                if hotkeys::run_pressed(ui) {
                    run_clicked = true;
                }
                if hotkeys::next_pressed(ui) {
                    next_clicked = true;
                }
                if hotkeys::rnext_pressed(ui) {
                    rnext_clicked = true;
                }
                if hotkeys::rollback_pressed(ui) {
                    rollback_clicked = true;
                }
                if hotkeys::snapshot_pressed(ui) {
                    snapshot_clicked = true;
                }
                if hotkeys::interactive_canvas_pressed(&ctx.input()) {
                    canvas_clicked = true;
                }
                if hotkeys::help_pressed(&ui.input()) {
                    help_clicked = true;
                }
                if hotkeys::focus_on_code_pressed(&ui.input()) {
                    self.focus_on_code = true;
                }
                if hotkeys::file_open_pressed(&ui.input()) {
                    open_clicked = true;
                }
                if hotkeys::goto_pressed(&ui.input()) {
                    goto_clicked = true;
                }
            }
            if vars_clicked {
                self.vars_open = true;
            }
            if goto_clicked {
                self.goto_open = true;
                self.goto_old_pos = Some(self.view_pos);
            }
            if open_clicked {
                self.open_file_dialog();
            }
            if self.process_async_file_open() || self.process_file_drop(ctx) {
                ctx.request_repaint();
            }
            if canvas_clicked {
                self.canvas_open = !self.canvas_open;
            }
            if help_clicked {
                self.help.is_open = !self.help.is_open;
            }
            if rollback_clicked && rollback_enabled {
                let t = Instant::now();
                self.rollback();
                self.last_dt = Some(t.elapsed().as_secs_f64());
            }
            if snapshot_clicked && !self.is_trial() {
                let t = Instant::now();
                self.snapshot();
                self.last_dt = Some(t.elapsed().as_secs_f64());
            }

            if self.is_trial() && repl_clicked {
                self.rollback();
                self.trial_code = None;
                self.focus_on_code = true;
            }
            if (!self.is_trial() && trial_clicked) || (self.is_trial() && self.snapshot.is_none()) {
                let t = Instant::now();
                self.trial_code = Some(Xstr::new());
                self.snapshot();
                self.last_dt = Some(t.elapsed().as_secs_f64());
                self.focus_on_code = true;
                self.frozen_code.push(FrozenStr::Log(
                    "Trial and error mode, everyting evaluating on-fly!\n\
                Press Run to freeze changes."
                        .into(),
                ));
            }

            if self.is_trial() {
                if self.trial_code.as_ref().map(|s| s.as_str()) != Some(&self.live_code) {
                    self.rollback();
                    self.xs.set_recording_enabled(self.rdebug_enabled);
                    let t = Instant::now();
                    let xsrc = Xstr::from(&self.live_code);
                    self.trial_code = Some(xsrc.clone());
                    if has_some_code {
                        self.xs.set_calc_limit(self.calc_limit).unwrap();
                        let _res = self.xs.evalxstr(xsrc);
                    }
                    self.debug_token = self.xs.location_from_current_ip();
                    self.last_dt = Some(t.elapsed().as_secs_f64());
                }
                if self.xs.last_error().is_some() || self.xs.is_running() {
                    // prevent from saving errorneous code
                    run_clicked = false;
                }
            }
            if let Some(s) = self.xs.stdout() {
                if !s.is_empty() {
                    let s = s.take();
                    self.frozen_code.push(if self.is_trial() {
                        FrozenStr::TrialLog(s)
                    } else {
                        FrozenStr::Log(s)
                    });
                }
            }
            if next_clicked || rnext_clicked {
                let t = Instant::now();
                let _res = if next_clicked {
                    self.xs.next()
                } else {
                    self.xs.rnext()
                };
                self.debug_token = self.xs.location_from_current_ip();
                self.last_dt = Some(t.elapsed().as_secs_f64());
            } else if run_clicked && has_some_code {
                let t = Instant::now();
                let xsrc = if self.is_trial() {
                    self.trial_code.clone().unwrap()
                } else {
                    Xstr::from(&self.live_code)
                };
                let buble_log = match self.frozen_code.last() {
                    Some(FrozenStr::TrialLog(_)) => self.frozen_code.pop(),
                    _ => None,
                };
                for s in xeh::lex::XstrLines::new(xsrc.clone()) {
                    self.frozen_code.push(FrozenStr::Code(s))
                }
                if let Some(FrozenStr::TrialLog(log)) = buble_log {
                    self.frozen_code.push(FrozenStr::Log(log));
                }
                if self.is_trial() {
                    self.snapshot();
                } else {
                    self.xs.set_calc_limit(self.calc_limit).unwrap();
                    let _ = self.xs.evalxstr(xsrc);
                    self.debug_token = self.xs.location_from_current_ip();
                }
                self.live_code.clear();
                self.last_dt = Some(t.elapsed().as_secs_f64());
            }
            if next_clicked
                || rnext_clicked
                || run_clicked
                || rollback_clicked
                || (self.is_trial() && has_some_code)
            {
                self.debug_token = self.xs.location_from_current_ip();
                if let Ok((w, h, buf)) = crate::canvas::copy_rgba(&mut self.xs) {
                    if self.canvas.is_empty() {
                        self.canvas_open = true;
                    }
                    self.canvas.update(ctx, w, h, buf);
                }
            }
            if unfreeze_clicked {
                self.reload_state();
            }
        });

        // CentralPanel end
    }

    fn is_trial(&self) -> bool {
        self.trial_code.is_some()
    }

    fn rlog_size(&self) -> Option<usize> {
        self.xs.reverse_log.as_ref().map(|rlog| rlog.len())
    }

    fn ui_error_highlight(&self, ui: &mut Ui, loc: &TokenLocation, err: &Xerr) {
        let (a, b, c) = split_highlight(loc);
        ui.horizontal(|ui| {
            ui.label(RichText::new(a).monospace().color(self.theme.code));
            ui.label(RichText::new(b).monospace().color(self.theme.error));
            ui.label(RichText::new(c).monospace().color(self.theme.code));
        });
        let n: usize = loc
            .whole_line
            .chars()
            .take(loc.col)
            .map(|c| if c == '\t' { egui::text::TAB_SIZE } else { 1 })
            .sum();
        let pos = format!("{:->1$}", '^', n + 1);
        ui.colored_label(self.theme.error, pos);
        ui.colored_label(self.theme.error, format!("{}", err));
    }

    fn ui_debugger_highlight(&self, ui: &mut Ui, loc: &TokenLocation) {
        let (a, b, c) = split_highlight(loc);
        ui.horizontal_top(|ui| {
            ui.label(RichText::new(a).monospace().color(self.theme.code));
            ui.label(RichText::new(b).monospace().color(self.theme.debug_marker));
            ui.label(RichText::new(c).monospace().color(self.theme.code));
        });
    }

    fn menu_examples(&mut self, ui: &mut Ui) {
        if ui.button("C String").clicked() {
            self.example_request = Some((
                include_str!("../docs/examples/cstring.xeh"),
                include_bytes!("../docs/examples/cstring.bin"),
            ));
            ui.close_menu();
        }
        if ui.button("Gameboy Tile 2BPP").clicked() {
            self.example_request = Some((
                include_str!("../docs/examples/gb-tile-2bpp.xeh"),
                include_bytes!("../docs/examples/gb-tile-2bpp.bin"),
            ));
            ui.close_menu();
        }
        if ui.button("iNES ROM").clicked() {
            self.example_request = Some((
                include_str!("../docs/examples/ines.xeh"),
                &[],
                //include_bytes!("../docs/examples/smb.nes"),
            ));
            ui.close_menu();
        }
        // if ui.button("6502 instructions").clicked() {
        //     self.example_request = Some(include_str!("docs/ex/6502.xeh"),
        //                                    include_bytes!("docs/ex/6502.bin"));
        //     ui.close_menu();
        // }
    }

    fn open_file_dialog(&mut self) {
        self.bin_future = Some(Box::pin(async {
            let res = rfd::AsyncFileDialog::new().pick_file().await;
            if let Some(file) = res {
                Some(file.read().await)
            } else {
                None
            }
        }));
    }

    fn process_async_file_open(&mut self) -> bool {
        if let Some(future) = self.bin_future.as_mut() {
            let waker = Arc::new(MyWaker()).into();
            let context = &mut Context::from_waker(&waker);
            match Pin::new(future).poll(context) {
                Poll::Pending => (),
                Poll::Ready(None) => {
                    self.bin_future.take();
                }
                Poll::Ready(Some(data)) => {
                    let s = Xbitstr::from(data);
                    self.bin_future.take();
                    self.binary_dropped(s);
                }
            }
        }
        return self.bin_future.is_some();
    }

    fn process_file_drop(&mut self, ctx: &egui::Context) -> bool {
        if let Some(d) = ctx.input().raw.dropped_files.first() {
            if let Some(data) = &d.bytes {
                let s = Xbitstr::from(data.as_ref().to_owned());
                self.binary_dropped(s);
                return true;
            }
        }
        return false;
    }
}

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake};

struct MyWaker();

impl Wake for MyWaker {
    fn wake(self: Arc<Self>) {}
}

fn split_highlight(loc: &TokenLocation) -> (String, String, String) {
    let line = &loc.whole_line;
    let start = loc.col;
    let end = (start + loc.token.len()).min(line.len());
    let a = line.substr(0..start).to_string();
    let b = line.substr(start..end).to_string();
    let c = line.substr(end..).to_string();
    (a, b, c)
}

impl epi::App for TemplateApp {
    fn name(&self) -> &str {
        "XEH"
    }

    fn max_size_points(&self) -> egui::Vec2 {
        egui::Vec2::new(1280.0, 2048.0)
    }

    fn clear_color(&self) -> egui::Rgba {
        egui::Rgba::from_rgba_premultiplied(0.0, 0.0, 0.0, 1.0)
    }

    /// Called once before the first frame.
    fn setup(
        &mut self,
        _ctx: &egui::Context,
        _frame: &epi::Frame,
        storage: Option<&dyn epi::Storage>,
    ) {
        #[cfg(feature = "persistence")]
        if let Some(storage) = storage {
            self.theme = epi::get_value(storage, epi::APP_KEY).unwrap_or_default()
        }
        self.load_help();
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, &self.theme);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &epi::Frame) {
        crate::style::tune(ctx, &self.theme);
        if let Some((code, data)) = self.example_request.take() {
            self.frozen_code.clear();
            self.binary_dropped(Xbitstr::from(data));
            self.live_code = code.to_string();
        }
        self.editor(ctx);
    }
}
