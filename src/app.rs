use egui::*;

use crate::agent::{AgentConfig, AgentRole, AgentSystem};
use crate::collab::{CollabSystem, CollabMessage};
use crate::hotkeys;
use crate::palette::{Palette, CommandAction};
use crate::style::Theme;
use crate::workspace::Workspace;
use uuid::Uuid;
use crate::{canvas::*, layouter};
use std::fmt::Write;
use xeh::prelude::*;
use xeh::*;
use std::collections::HashMap;
use similar::{ChangeTag, TextDiff};

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
    words: Vec<(Xstr, Xmap)>,
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
    last_dt: Option<(f64, &'static str)>,
    canvas: Canvas,
    canvas_open: bool,
    debug_token: Option<TokenLocation>,
    rdebug_enabled: bool,
    interval_enabled: bool,
    interval_timer: Instant,
    interval_word: String,
    insn_limit: Option<usize>,
    snapshot: Option<(Xstate, Vec<FrozenStr>)>,
    bin_future: Option<Pin<BoxFuture>>,
    input_binary: Option<Xbitstr>,
    focus_on_code: bool,
    bytecode_open: bool,
    bytecode_follow: bool,
    vars_open: bool,
    vars_boot_len: usize,
    goto_open: bool,
    goto_text: String,
    goto_old_pos: Option<usize>,
    help: Help,
    theme: Theme,
    theme_editor: bool,
    example_request: Option<(&'static str, &'static [u8])>,
    // Agent System
    agent_system: AgentSystem,
    agents_open: bool,
    todo_open: bool,
    agent_paste_buffer: String,
    new_agent_model: String,
    new_agent_url: String,
    new_agent_system: String,
    new_task_buffer: String,
    planning_goal: String,
    // Collaboration
    collab_system: CollabSystem,
    collab_url: String,
    collab_open: bool,
    last_agent_sync: f64,
    my_uuid: Uuid,
    palette: Palette,
    // Workspaces
    workspaces: HashMap<String, Workspace>,
    workspace_open: bool,
    current_workspace: String,
    new_workspace_name: String,
    pending_reviews: std::collections::VecDeque<(Uuid, String)>,
}

#[derive(Clone)]
enum FrozenStr {
    Code(Xsubstr),
    Log(String),
    TrialLog(String),
}

const SECTION_TAG: Cell = xeh_str_lit!("section");
const STACK_TAG: Cell = xeh_str_lit!("stack-comment");
const EXAMPLE_TAG: Cell = xeh_str_lit!("example");
const HELPTEXT_TAG: Cell = xeh_str_lit!("text");

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
            insn_limit: Some(10_000_000),
            snapshot: None,
            bin_future: None,
            input_binary: None,
            focus_on_code: true,
            rdebug_enabled: false,
            interval_enabled: false,
            interval_word: String::new(),
            interval_timer: Instant::now(),
            bytecode_open: false,
            bytecode_follow: true,
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
            agent_system: AgentSystem::default(),
            agents_open: false,
            todo_open: false,
            agent_paste_buffer: String::new(),
            new_agent_model: "llama3".to_string(),
            new_agent_url: "http://localhost:11434".to_string(),
            new_agent_system: "You are a helpful coding assistant.".to_string(),
            new_task_buffer: String::new(),
            planning_goal: String::new(),
            collab_system: CollabSystem::default(),
            collab_url: "ws://localhost:8080".to_string(),
            collab_open: false,
            last_agent_sync: 0.0,
            my_uuid: Uuid::new_v4(),
            palette: Palette::default(),
            workspaces: HashMap::new(),
            workspace_open: false,
            current_workspace: "Default".to_string(),
            new_workspace_name: String::new(),
            pending_reviews: std::collections::VecDeque::new(),
        }
    }
}

impl TemplateApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        // Load previous app state (if any).
        let mut app = TemplateApp::default();
        #[cfg(feature = "persistence")]
        if let Some(storage) = cc.storage {
            app.theme = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
            app.interval_word = eframe::get_value(storage, "interval").unwrap_or_default();
            app.workspaces = eframe::get_value(storage, "workspaces").unwrap_or_default();
            app.current_workspace = eframe::get_value(storage, "current_workspace").unwrap_or("Default".to_string());
            if let Some(ws) = app.workspaces.get(&app.current_workspace) {
                app.live_code = ws.code.clone();
                app.agent_system.tasks = ws.tasks.clone();
            }
        }
        app.load_help();
        // Pre-populate some tasks/agents for demo
        // app.agent_system.add_task("Analyze codebase structure".to_string());
        return app;
    }

    fn xs_respawn() -> Xstate {
        let mut xs = Xstate::boot().unwrap();
        xs.intercept_stdout(true);
        xeh::d2_plugin::load(&mut xs).unwrap();
        xs
    }

    fn load_help(&mut self) {
        let mut tmp_xs = Xstate::core().unwrap();
        tmp_xs.eval(include_str!("../assets/help.xeh")).unwrap();
        let help_data = tmp_xs.pop_data().unwrap();
        let help_index = help_data.xmap().unwrap();
        let words = self
            .xs
            .word_list()
            .into_iter()
            .filter_map(|name| {
                let res = help_index.get(&Cell::from(name.clone()));
                res.map(|val| (name, val.xmap().unwrap().clone()))
            })
            .collect();
        self.help.words = words;
    }

    fn get_relevant_docs(&self, query: &str) -> String {
        let mut context = String::new();
        let tokens: Vec<&str> = query.split_whitespace().collect();
        let mut matches = 0;
        for (word, help) in &self.help.words {
            if tokens.iter().any(|t| word.as_str().eq_ignore_ascii_case(t)) {
                if let Some(Ok(text)) = help.get(&HELPTEXT_TAG).map(|x| x.str()) {
                    context.push_str(&format!("Word: {}\nDescription: {}\n\n", word, text));
                    matches += 1;
                }
                if matches >= 5 {
                    break;
                }
            }
        }
        context
    }

    fn move_view(&mut self, nrows: isize) {
        let n = (self.view_pos as isize + (nrows * self.num_cols as isize * 8)).max(0) as usize;
        self.view_pos = n.min(self.current_bstr().end());
    }

    fn current_input(&self) -> &Xcell {
        self.xs.eval_named_value("input").unwrap()
    }

    fn current_bstr(&self) -> &Xbitstr {
        self.current_input().bitstr().unwrap()
    }

    fn current_offset(&self) -> usize {
        self.current_input()
            .get_tag(&Cell::from("at"))
            .map(|x| x.to_usize().unwrap())
            .unwrap_or_default()
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

        if let Some(action) = self.palette.show(ctx) {
             match action {
                 CommandAction::ToggleTheme => self.theme_editor = !self.theme_editor,
                 CommandAction::OpenBinary => open_clicked = true,
                 CommandAction::ToggleCanvas => canvas_clicked = true,
                 CommandAction::ToggleBytecode => self.bytecode_open = !self.bytecode_open,
                 CommandAction::ToggleVariables => vars_clicked = true,
                 CommandAction::ToggleAgents => self.agents_open = !self.agents_open,
                 CommandAction::ToggleToDo => self.todo_open = !self.todo_open,
                 CommandAction::Run => run_clicked = true,
                 CommandAction::Snapshot => snapshot_clicked = true,
                 CommandAction::Rollback => rollback_clicked = true,
                 CommandAction::HelpHotkeys => {
                     help_clicked = true;
                     self.help.mode = HelpMode::Hotkeys;
                 },
                 CommandAction::HelpIndex => {
                     help_clicked = true;
                     self.help.mode = HelpMode::Index;
                 },
                 CommandAction::HelpQuickRef => {
                     help_clicked = true;
                     self.help.mode = HelpMode::QuickRef;
                 },
                 CommandAction::ConnectNetwork => self.collab_open = !self.collab_open,
                 CommandAction::ToggleWorkspaces => self.workspace_open = !self.workspace_open,
                 CommandAction::SaveWorkspace => {
                    let ws = Workspace {
                        name: self.current_workspace.clone(),
                        code: self.live_code.clone(),
                        tasks: self.agent_system.tasks.clone(),
                    };
                    self.workspaces.insert(self.current_workspace.clone(), ws);
                 },
             }
        }

        // Poll Agent System
        self.agent_system.poll(ctx.input(|i| i.time), &self.live_code);

        // Poll Collab System
        let mut code_changed_remotely = false;
        let msgs = self.collab_system.poll();
        for msg in msgs {
            match msg {
                CollabMessage::Hello { id, name } => {
                    self.collab_system.peers.insert(id, name);
                }
                CollabMessage::Code { text } => {
                    if self.live_code != text {
                        self.live_code = text;
                        code_changed_remotely = true;
                    }
                }
                CollabMessage::AgentUpdate { agents, tasks } => {
                    for agent in agents {
                        if !self.agent_system.local_ids.contains(&agent.id) {
                            self.agent_system.agents.insert(agent.id, agent);
                        }
                    }
                    for task in tasks {
                        if let Some(existing) = self.agent_system.tasks.iter_mut().find(|t| t.id == task.id) {
                            *existing = task;
                        } else {
                            self.agent_system.tasks.push(task);
                        }
                    }
                }
            }
        }

        // Apply pending changes from agents
        while let Some(item) = self.agent_system.pending_changes.pop_front() {
            self.pending_reviews.push_back(item);
        }

        // Code Review UI
        if let Some((aid, code)) = self.pending_reviews.front().cloned() {
            let mut open = true;
            let agent_name = self.agent_system.agents.get(&aid).map(|a| a.config.name.clone()).unwrap_or_else(|| "Unknown Agent".to_string());

            egui::Window::new("Code Review")
                .open(&mut open)
                .default_pos(pos2(win_rect.center().x, win_rect.center().y))
                .show(ctx, |ui| {
                    ui.heading(format!("Review from {}", agent_name));
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                        let diff = TextDiff::from_lines(&self.live_code, &code);
                        for change in diff.iter_all_changes() {
                            let color = match change.tag() {
                                ChangeTag::Delete => egui::Color32::from_rgb(100, 0, 0),
                                ChangeTag::Insert => egui::Color32::from_rgb(0, 100, 0),
                                ChangeTag::Equal => egui::Color32::TRANSPARENT,
                            };
                            ui.horizontal(|ui| {
                                let sign = match change.tag() {
                                    ChangeTag::Delete => "-",
                                    ChangeTag::Insert => "+",
                                    ChangeTag::Equal => " ",
                                };
                                ui.label(RichText::new(format!("{} {}", sign, change.value().trim_end())).background_color(color).monospace());
                            });
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("✅ Approve").clicked() {
                            // 1. Snapshot BEFORE modification for Undo safety
                            if !self.is_trial() {
                                self.snapshot();
                            }

                            self.frozen_code.push(FrozenStr::Log(format!("\n# Agent {} wrote code", agent_name)));
                            self.live_code.push_str("\n");
                            self.live_code.push_str(&code);
                            self.frozen_code.push(FrozenStr::Code(Xsubstr::from(&code)));

                            self.pending_reviews.pop_front();
                        }
                        if ui.button("❌ Reject").clicked() {
                            self.pending_reviews.pop_front();
                        }
                        if ui.button("Skip").clicked() {
                             // Move to back
                             if let Some(item) = self.pending_reviews.pop_front() {
                                 self.pending_reviews.push_back(item);
                             }
                        }
                    });
                });
        }

        self.theme.theme_ui(ctx, &mut self.theme_editor);

        // Collab UI
        egui::Window::new("Network Connection")
            .open(&mut self.collab_open)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("URL:");
                    ui.text_edit_singleline(&mut self.collab_url);
                    if ui.button("Connect").clicked() {
                        self.collab_system.connect(&self.collab_url, ctx);
                        self.collab_system.send(CollabMessage::Hello {
                            id: self.my_uuid,
                            name: "User".to_string(),
                        });
                    }
                });
                ui.label(format!("Status: {}", self.collab_system.status));
                ui.separator();
                ui.heading("Peers");
                for (id, name) in &self.collab_system.peers {
                    ui.label(format!("{} ({})", name, id));
                }
            });

        // Agents UI
        egui::Window::new("🤖 Agent Swarm")
            .open(&mut self.agents_open)
            .default_pos(pos2(win_rect.right() - 400.0, 100.0))
            .vscroll(true)
            .show(ctx, |ui| {
                self.agent_system.ui_agents(ui);
                ui.separator();
                ui.heading("Configuration");
                ui.horizontal(|ui| {
                     ui.label("Model:");
                     ui.text_edit_singleline(&mut self.new_agent_model);
                });
                ui.horizontal(|ui| {
                     ui.label("Base URL:");
                     ui.text_edit_singleline(&mut self.new_agent_url);
                });
                ui.label("System Prompt:");
                ui.add(egui::TextEdit::multiline(&mut self.new_agent_system).desired_rows(2));
                ui.label("Paste config (Name:Key) one per line:");
                ui.add(egui::TextEdit::multiline(&mut self.agent_paste_buffer).desired_rows(3));
                if ui.button("Add Agents").clicked() {
                    for line in self.agent_paste_buffer.lines() {
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() >= 2 {
                            self.agent_system.add_agent(AgentConfig {
                                name: parts[0].trim().to_string(),
                                role: AgentRole::Generalist,
                                api_key: parts[1].trim().to_string(),
                                base_url: self.new_agent_url.clone(),
                                model: self.new_agent_model.clone(),
                                system_prompt: self.new_agent_system.clone(),
                            });
                        }
                    }
                    self.agent_paste_buffer.clear();
                }
                if ui.button("Clear Agents").clicked() {
                    self.agent_system.agents.clear();
                    self.agent_system.local_ids.clear();
                }
                ui.separator();
                ui.heading("Logs");
                egui::ScrollArea::vertical().id_salt("agent_logs").max_height(150.0).show(ui, |ui| {
                    for msg in &self.agent_system.message_log {
                        ui.label(RichText::new(msg).monospace().size(10.0));
                    }
                });
            });

        let mut todo_open = self.todo_open;
        egui::Window::new("📝 ToDo List")
            .open(&mut todo_open)
            .default_pos(pos2(win_rect.right() - 400.0, 400.0))
            .vscroll(true)
            .show(ctx, |ui| {
                self.agent_system.ui_tasks(ui);
                ui.separator();
                ui.heading("Add Task");
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.new_task_buffer);
                    if ui.button("Add").clicked() {
                        if !self.new_task_buffer.is_empty() {
                            let ctx_docs = self.get_relevant_docs(&self.new_task_buffer);
                            self.agent_system
                                .add_task(self.new_task_buffer.clone(), ctx_docs);
                            self.new_task_buffer.clear();
                        }
                    }
                });
                ui.separator();
                ui.heading("AI Planning");
                ui.label("Describe a high-level goal:");
                ui.add(egui::TextEdit::multiline(&mut self.planning_goal).desired_rows(2));
                if ui.button("🔮 Generate Plan").clicked() && !self.planning_goal.is_empty() {
                     // Find a suitable agent
                     let agent_info = self.agent_system.agents.values().find(|a| matches!(a.config.role, AgentRole::Generalist) || true).map(|a| (a.id, a.config.clone()));

                     if let Some((agent_id, config)) = agent_info {
                         let ctx_docs = self.get_relevant_docs(&self.planning_goal);
                         self.agent_system.spawn_planning_request(agent_id, config, &self.planning_goal, &self.live_code, &ctx_docs);
                         self.agent_system.log(format!("Planning started for: {}", self.planning_goal));
                     }
                }
            });
        self.todo_open = todo_open;

        egui::Window::new("Workspaces")
            .open(&mut self.workspace_open)
            .default_pos(pos2(win_rect.center().x, win_rect.center().y))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Current:");
                    ui.strong(&self.current_workspace);
                    if ui.button("📋 Fork").clicked() {
                         let new_name = format!("{}-copy", self.current_workspace);
                         if !self.workspaces.contains_key(&new_name) {
                             // Save current state to the new workspace
                             let ws = Workspace {
                                 name: new_name.clone(),
                                 code: self.live_code.clone(),
                                 tasks: self.agent_system.tasks.clone(),
                             };
                             self.workspaces.insert(new_name.clone(), ws);
                             // Switch to it
                             self.current_workspace = new_name;
                             // No need to reload code/tasks as they are identical
                         }
                    }
                });
                ui.separator();
                ui.heading("Switch To:");
                let mut to_switch = None;
                let mut to_delete = None;
                for name in self.workspaces.keys() {
                    ui.horizontal(|ui| {
                        if ui.button(name).clicked() {
                            to_switch = Some(name.clone());
                        }
                        if name != "Default" {
                             if ui.button("🗑").clicked() {
                                 to_delete = Some(name.clone());
                             }
                        }
                    });
                }
                if let Some(name) = to_switch {
                    // Save current
                    let ws = Workspace {
                        name: self.current_workspace.clone(),
                        code: self.live_code.clone(),
                        tasks: self.agent_system.tasks.clone(),
                    };
                    self.workspaces.insert(self.current_workspace.clone(), ws);

                    // Clear pending reviews and agent state to prevent cross-workspace pollution
                    self.pending_reviews.clear();
                    self.agent_system.pending_changes.clear();
                    for agent in self.agent_system.agents.values_mut() {
                        agent.status = crate::agent::AgentStatus::Idle;
                        agent.current_task_id = None;
                    }

                    // Load new
                    self.current_workspace = name;
                    if let Some(ws) = self.workspaces.get(&self.current_workspace) {
                        self.live_code = ws.code.clone();
                        self.agent_system.tasks = ws.tasks.clone();
                    }
                }
                if let Some(name) = to_delete {
                    self.workspaces.remove(&name);
                }

                ui.separator();
                ui.horizontal(|ui| {
                     ui.text_edit_singleline(&mut self.new_workspace_name);
                     if ui.button("Create New").clicked() && !self.new_workspace_name.is_empty() {
                         let name = self.new_workspace_name.clone();
                         if !self.workspaces.contains_key(&name) {
                             // Save current
                             let ws = Workspace {
                                 name: self.current_workspace.clone(),
                                 code: self.live_code.clone(),
                                 tasks: self.agent_system.tasks.clone(),
                             };
                             self.workspaces.insert(self.current_workspace.clone(), ws);

                             // Create new (empty)
                             self.current_workspace = name.clone();
                             self.live_code.clear();
                             self.agent_system.tasks.clear();
                             self.workspaces.insert(name, Workspace { name: self.current_workspace.clone(), ..Default::default() });
                             self.new_workspace_name.clear();
                         }
                     }
                });
            });

        egui::Window::new("Variables")
            .open(&mut self.vars_open)
            .default_pos(pos2(win_rect.right() - 200.0, 200.0))
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                for i in self.xs.return_stack.iter().take(1000) {
                    ui.horizontal(|ui| {
                        for j in 0..i.locals.len() {
                            ui.colored_label(self.theme.text, format!("local.{}: ", j));
                            ui.colored_label(self.theme.text, i.locals[j].format_safe());
                        }
                    });
                    ui.separator();
                }
                let lst = self.xs.var_list();
                let n = lst.len().checked_sub(self.vars_boot_len).unwrap_or(0);
                for (name, val) in lst.iter().rev().take(n) {
                    ui.horizontal(|ui| {
                        ui.colored_label(self.theme.text, name.to_string());
                        ui.colored_label(self.theme.code_frozen, val.format_safe());
                    });
                }
            });

        egui::Window::new("Bytecode")
            .open(&mut self.bytecode_open)
            .default_pos(pos2(200.0, 400.0))
            .vscroll(true)
            .show(ctx, |ui| {
                ui.checkbox(&mut self.bytecode_follow, "Follow current instruction");
                ui.vertical(|ui| {
                    let code = self.xs.bytecode();
                    let mut lookat = None;
                    for i in 0..code.len() {
                        let optext = self.xs.fmt_opcode(i, &code[i]);
                        let mut rich = RichText::new(format!("{:05x}:   {}", i, optext))
                            .monospace()
                            .color(self.theme.text);
                        if i == self.xs.ip() {
                            rich = rich.background_color(self.theme.border);
                        }
                        let resp = ui.label(rich);
                        if i == self.xs.ip() {
                            lookat = Some(resp.rect);
                        }
                    }
                    if self.xs.ip() == code.len() {
                        let fmt =
                            RichText::new(format!("{:05x}: # end of bytecode #", self.xs.ip()))
                                .monospace()
                                .color(self.theme.text)
                                .background_color(self.theme.border);
                        let rect = ui.label(fmt).rect;
                        lookat = Some(rect);
                    }
                    if self.bytecode_follow {
                        if let Some(rect) = lookat {
                            ui.scroll_to_rect(rect, Some(Align::Center));
                        }
                    }
                });
                ui.checkbox(&mut self.bytecode_follow, "Follow current instruction");
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
                let mut cancel_clicked = ui.input(|i| i.key_pressed(Key::Escape));
                let mut ok_clicked = ui.input(|i| i.key_pressed(Key::Enter));
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
                                add(
                                    ui,
                                    "Program - Run",
                                    "(Esc, R) or (Cmd+Enter) or (Ctrl+Enter)",
                                );
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
                                add(ui, "Command Palette", "(Cmd/Ctrl+Shift+P)");
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
                            ui.checkbox(&mut self.help.follow_cursor, "Follow");
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
                                        .get(&SECTION_TAG)
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
                                            if let Some(t) = help.get(&STACK_TAG) {
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

                                        if let Some(Ok(s)) =
                                            help.get(&HELPTEXT_TAG).map(|x| x.str())
                                        {
                                            ui.label(s);
                                        }
                                        if let Some(v) = help.get(&EXAMPLE_TAG) {
                                            let s = v.str().ok().unwrap_or("");
                                            let example = RichText::new(s)
                                                .color(self.theme.code_frozen)
                                                .background_color(self.theme.code_background);
                                            ui.separator();
                                            ui.horizontal(|ui| {
                                                ui.separator();
                                                ui.monospace(example);
                                            });
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
                        ScrollArea::vertical()
                            .auto_shrink([false; 2])
                            .show(ui, |ui| {
                                ui.style_mut().visuals.extreme_bg_color =
                                    self.theme.code_background;
                                ui.monospace(include_str!("../README.md"));
                            });
                    }
                }
            }); //help

        let mut rnext_enabled = false;
        let rollback_enabled = self.snapshot.is_some() && !self.is_trial();
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button(self.menu_text("Open Binary...")).clicked() {
                        open_clicked = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Network", |ui| {
                    if ui.button(self.menu_text("Connection...")).clicked() {
                        self.collab_open = !self.collab_open;
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
                    if ui.button(self.menu_text("Bytecode")).clicked() {
                        self.bytecode_open = !self.bytecode_open;
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
                ui.menu_button("Agents", |ui| {
                    if ui.button(self.menu_text("Dashboard")).clicked() {
                        self.agents_open = !self.agents_open;
                        ui.close_menu();
                    }
                    if ui.button(self.menu_text("ToDo List")).clicked() {
                        self.todo_open = !self.todo_open;
                        ui.close_menu();
                    }
                });
                run_clicked = ui.button(self.menu_text("🚀Run")).clicked();
                snapshot_clicked = ui
                    .add_enabled(!self.is_trial(), Button::new(self.menu_text("💾Snapshot")))
                    .clicked();
                rollback_clicked = ui
                    .add_enabled(rollback_enabled, Button::new(self.menu_text("🔨Rollback")))
                    .clicked();
                let unfreeze_enabled = self.frozen_code.iter().any(|c| match c {
                    FrozenStr::Code(_) => true,
                    _ => false,
                });
                unfreeze_clicked = ui
                    .add_enabled(unfreeze_enabled, Button::new(self.menu_text("🔥Unfreeze")))
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
                    ui.add_enabled(false, Label::new("Examples:"));
                    self.menu_examples(ui);
                });
                ui.checkbox(&mut self.interval_enabled, "Interval");
                ui.add(TextEdit::singleline(&mut self.interval_word).desired_width(100.0));
                let duration = std::time::Duration::from_millis(33);
                if self.interval_enabled && self.interval_timer < Instant::now() {
                    let old_meter = self.xs.insn_meter;
                    let _ = self.xs.run_word(&self.interval_word);
                    self.xs.insn_meter = old_meter;
                    self.interval_timer = Instant::now().checked_add(duration).unwrap();
                }
                ui.ctx().request_repaint_after(duration);
                ui.label(format!("{:?}", duration));
            });
        }); // top panel

        egui::SidePanel::left("left_panel")
            .resizable(false)
            .show(ctx, |ui| {
                let ncols = self.num_cols * 4 + 10;
                let total_rows = self.num_rows;
                let text_style = TextStyle::Monospace;
                let font = text_style.resolve(ui.style());
                let (glyph_width, row_height) =
                    ui.fonts(|f| (f.glyph_width(&font, '0'), f.row_height(&font)));
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
                        ui.colored_label(self.theme.comment, format!(" of {}", end));
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
                                    let hl = Label::new(hex_data).sense(Sense::hover());
                                    ui.add(hl);
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
                    let mut it = self.xs.data_slice().iter().rev().enumerate();
                    while let Some((i, val)) = it.next() {
                        if i > 200 {
                            let rest_len = it.size_hint().0;
                            ui.colored_label(
                                self.theme.comment,
                                &format!("\n... {} more items on the stack", rest_len),
                            );
                            break;
                        }
                        let mut s = format!("{:?}", val);
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
                    }
                });
            });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            self.ui_mini_status(ui, true);
        });

        let esc_pressed = ctx.input(|i| i.key_down(Key::Escape));
        let mut live_has_focus = false;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::containers::ScrollArea::vertical()
                .stick_to_bottom(true)
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
                    self.ui_mini_status(ui, show_trial_error);
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
                        ui.fonts(|f| f.layout_job(j))
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
                    } else if ctx.input(hotkeys::focus_on_code_pressed) || self.focus_on_code {
                        code.response.request_focus();
                        self.focus_on_code = false;
                    }
                    live_has_focus = code.response.has_focus();

                    if code.response.changed() && !code_changed_remotely {
                        if self.collab_system.is_connected() {
                            self.collab_system.send(CollabMessage::Code { text: self.live_code.clone() });
                        }
                    }
                });

            // Sync Agents
            if self.collab_system.is_connected() {
                let t = ctx.input(|i| i.time);
                if t - self.last_agent_sync > 1.0 {
                    self.last_agent_sync = t;
                    let local_agents: Vec<_> = self.agent_system.agents.values()
                        .filter(|a| self.agent_system.local_ids.contains(&a.id))
                        .cloned()
                        .collect();
                    let tasks = self.agent_system.tasks.clone();
                    self.collab_system.send(CollabMessage::AgentUpdate { agents: local_agents, tasks });
                }
            }

            let has_some_code = !self.live_code.trim().is_empty();
            if live_has_focus || self.focus_on_code {
                if ui.input(|i| {
                    (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(Key::Enter)
                }) {
                    run_clicked = true;
                }
            }
            if ui.input(hotkeys::command_palette_pressed) {
                self.palette.is_open = !self.palette.is_open;
            }

            if !live_has_focus && !self.help.is_open && !self.goto_open && !self.palette.is_open {
                let n = hotkeys::scroll_view_pressed(ctx, self.num_cols as isize);
                if n != 0 {
                    self.move_view(n);
                }
                if ui.input(hotkeys::recording_pressed) {
                    self.rdebug_enabled = !self.rdebug_enabled;
                }
                if ui.input(hotkeys::run_pressed) {
                    run_clicked = true;
                }
                if ui.input(hotkeys::next_pressed) {
                    next_clicked = true;
                }
                if ui.input(hotkeys::rnext_pressed) {
                    rnext_clicked = true;
                }
                if ui.input(hotkeys::rollback_pressed) {
                    rollback_clicked = true;
                }
                if ui.input(hotkeys::snapshot_pressed) {
                    snapshot_clicked = true;
                }
                if ctx.input(hotkeys::interactive_canvas_pressed) {
                    canvas_clicked = true;
                }
                if ui.input(hotkeys::help_pressed) {
                    help_clicked = true;
                }
                if ui.input(hotkeys::focus_on_code_pressed) {
                    self.focus_on_code = true;
                }
                if ui.input(hotkeys::file_open_pressed) {
                    open_clicked = true;
                }
                if ui.input(hotkeys::goto_pressed) {
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
                self.last_dt = Some((t.elapsed().as_secs_f64(), "ROLLBACK"));
            }
            if snapshot_clicked && !self.is_trial() {
                let t = Instant::now();
                self.snapshot();
                self.last_dt = Some((t.elapsed().as_secs_f64(), "SNAPSHOT"));
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
                self.last_dt = Some((t.elapsed().as_secs_f64(), ""));
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
                        self.xs.set_insn_limit(self.insn_limit).unwrap();
                        let _res = self.xs.evalxstr(xsrc);
                    }
                    self.debug_token = self.xs.location_from_current_ip();
                    self.last_dt = Some((t.elapsed().as_secs_f64(), ""));
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
                self.last_dt = Some((t.elapsed().as_secs_f64(), "R/NEXT"));
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
                    self.xs.set_insn_limit(self.insn_limit).unwrap();
                    let _ = self.xs.evalxstr(xsrc);
                    self.debug_token = self.xs.location_from_current_ip();
                }
                self.live_code.clear();
                self.last_dt = Some((t.elapsed().as_secs_f64(), ""));
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

    fn ui_mini_status(&mut self, ui: &mut Ui, show_trial_error: bool) {
        if let Some(err) = self.xs.last_error() {
            if show_trial_error {
                let s = format!("ERROR: {}", err);
                let mut request_fix = false;
                ui.horizontal(|ui| {
                    ui.colored_label(self.theme.error, &s);
                    if ui.button("🔧 Fix with Agent").clicked() {
                        request_fix = true;
                    }
                });

                if request_fix {
                    // Find a suitable agent (first generalist or just first)
                    let agent_info = self.agent_system.agents.values().find(|a| matches!(a.config.role, AgentRole::Generalist) || true).map(|a| (a.id, a.config.clone()));

                    if let Some((agent_id, config)) = agent_info {
                         let error_msg = format!("{}", err);
                         let task_desc = format!("Fix error: {}", error_msg);
                         let ctx_docs = self.get_relevant_docs(&error_msg); // Might find docs for error words

                         self.agent_system.spawn_llm_request(
                             agent_id,
                             config,
                             &task_desc,
                             &self.live_code,
                             &ctx_docs
                         );
                         self.agent_system.log(format!("Requested fix for error: {}", error_msg));
                    }
                }
            }
        } else {
            let mut s = String::new();
            write!(s, "OK ").unwrap();
            if let Some((secs, comment)) = self.last_dt {
                if !comment.is_empty() {
                    write!(s, "{} ", comment).unwrap();
                }
                write!(s, "{:.3}s", secs).unwrap();
                write!(s, " ({} vminsn)", self.xs.insn_meter).unwrap();
            }
            if let Some(n) = self.rlog_size() {
                write!(s, " RLOG {}", n).unwrap();
            }
            ui.colored_label(self.theme.comment, s);
        }
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
                include_str!("../assets/examples/cstring.xeh"),
                include_bytes!("../assets/examples/cstring.bin"),
            ));
            ui.close_menu();
        }
        if ui.button("Doom Fire").clicked() {
            self.example_request = Some((include_str!("../assets/examples/doom-fire.xeh"), &[]));
            ui.close_menu();
        }
        if ui.button("Gameboy Tile 2BPP").clicked() {
            self.example_request = Some((
                include_str!("../assets/examples/gb-tile-2bpp.xeh"),
                include_bytes!("../assets/examples/gb-tile-2bpp.bin"),
            ));
            ui.close_menu();
        }
        if ui.button("iNES ROM").clicked() {
            self.example_request = Some((
                include_str!("../assets/examples/ines.xeh"),
                include_bytes!("../assets/examples/ines.bin"),
            ));
            ui.close_menu();
        }
        if ui.button("Quake1Pak").clicked() {
            self.example_request = Some((
                include_str!("../assets/examples/quake-pak.xeh"),
                include_bytes!("../assets/examples/quake-pak.bin"),
            ));
            ui.close_menu();
        }
        if ui.button("Quake1Pak Build").clicked() {
            self.example_request =
                Some((include_str!("../assets/examples/quake-pak-build.xeh"), &[]));
            ui.close_menu();
        }
        if ui.button("VLQ Integer").clicked() {
            self.example_request = Some((
                include_str!("../assets/examples/vint.xeh"),
                include_bytes!("../assets/examples/vint.bin"),
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
        if let Some(d) = ctx.input(|i| i.raw.dropped_files.first().cloned()) {
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

impl eframe::App for TemplateApp {
    fn clear_color(&self, _style: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    /// Called by the frame work to save state before shutdown.
    /// Note that you must enable the `persistence` feature for this to work.
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.theme);
        eframe::set_value(storage, "interval", &self.interval_word);

        // Update current workspace before saving
        let ws = Workspace {
            name: self.current_workspace.clone(),
            code: self.live_code.clone(),
            tasks: self.agent_system.tasks.clone(),
        };
        self.workspaces.insert(self.current_workspace.clone(), ws);

        eframe::set_value(storage, "workspaces", &self.workspaces);
        eframe::set_value(storage, "current_workspace", &self.current_workspace);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::style::tune(ctx, &self.theme);
        if let Some((code, data)) = self.example_request.take() {
            self.frozen_code.clear();
            self.binary_dropped(Xbitstr::from(data));
            self.live_code = code.to_string();
        }
        self.editor(ctx);
    }
}
