use egui::{Context, Key, RichText, ScrollArea, TextEdit, Window, vec2};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(Clone, Debug)]
pub enum CommandAction {
    ToggleTheme,
    OpenBinary,
    ToggleCanvas,
    ToggleBytecode,
    ToggleVariables,
    ToggleAgents,
    ToggleToDo,
    Run,
    Snapshot,
    Rollback,
    HelpHotkeys,
    HelpIndex,
    HelpQuickRef,
    ConnectNetwork,
    ToggleWorkspaces,
    SaveWorkspace,
}

#[derive(Clone)]
pub struct Command {
    pub name: String,
    pub action: CommandAction,
}

pub struct Palette {
    pub is_open: bool,
    pub query: String,
    pub commands: Vec<Command>,
    pub selected_index: usize,
    matcher: SkimMatcherV2,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            commands: vec![
                Command { name: "Theme: Toggle Editor".into(), action: CommandAction::ToggleTheme },
                Command { name: "File: Open Binary...".into(), action: CommandAction::OpenBinary },
                Command { name: "View: Toggle Canvas".into(), action: CommandAction::ToggleCanvas },
                Command { name: "View: Toggle Bytecode".into(), action: CommandAction::ToggleBytecode },
                Command { name: "View: Toggle Variables".into(), action: CommandAction::ToggleVariables },
                Command { name: "Agents: Toggle Dashboard".into(), action: CommandAction::ToggleAgents },
                Command { name: "Agents: Toggle ToDo List".into(), action: CommandAction::ToggleToDo },
                Command { name: "Program: Run".into(), action: CommandAction::Run },
                Command { name: "Program: Snapshot".into(), action: CommandAction::Snapshot },
                Command { name: "Program: Rollback".into(), action: CommandAction::Rollback },
                Command { name: "Help: Hotkeys".into(), action: CommandAction::HelpHotkeys },
                Command { name: "Help: Index".into(), action: CommandAction::HelpIndex },
                Command { name: "Help: Quick Reference".into(), action: CommandAction::HelpQuickRef },
                Command { name: "Network: Connect...".into(), action: CommandAction::ConnectNetwork },
                Command { name: "Workspaces: Manage...".into(), action: CommandAction::ToggleWorkspaces },
                Command { name: "Workspaces: Save".into(), action: CommandAction::SaveWorkspace },
            ],
            selected_index: 0,
            matcher: SkimMatcherV2::default(),
        }
    }
}

impl Palette {
    pub fn show(&mut self, ctx: &Context) -> Option<CommandAction> {
        if !self.is_open {
            return None;
        }

        let mut action_to_execute = None;
        let mut open = self.is_open;

        let screen_rect = ctx.available_rect();
        let width = 600.0;
        let height = 400.0;
        let pos = screen_rect.center() - vec2(width / 2.0, height / 2.0);

        Window::new("Command Palette")
            .open(&mut open)
            .fixed_pos(pos)
            .fixed_size(vec2(width, height))
            .collapsible(false)
            .title_bar(false)
            .show(ctx, |ui| {
                ui.style_mut().visuals.extreme_bg_color = egui::Color32::from_gray(30);

                let text_edit_response = ui.add(
                    TextEdit::singleline(&mut self.query)
                        .hint_text("Type a command...")
                        .desired_width(f32::INFINITY)
                        .lock_focus(true)
                );

                if text_edit_response.changed() {
                    self.selected_index = 0;
                }

                if text_edit_response.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter)) {
                     // Handle selection via enter key logic below
                } else {
                    text_edit_response.request_focus();
                }

                ui.separator();

                let filtered_commands: Vec<(usize, &Command, i64)> = self.commands.iter().enumerate()
                    .filter_map(|(idx, cmd)| {
                        if self.query.is_empty() {
                            Some((idx, cmd, 0))
                        } else {
                            self.matcher.fuzzy_match(&cmd.name, &self.query).map(|score| (idx, cmd, score))
                        }
                    })
                    .collect();

                let mut sorted_commands = filtered_commands;
                if !self.query.is_empty() {
                    sorted_commands.sort_by(|a, b| b.2.cmp(&a.2));
                }

                if sorted_commands.is_empty() {
                    ui.label("No matching commands");
                } else {
                    // Navigation
                    if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                        self.selected_index = (self.selected_index + 1).min(sorted_commands.len().saturating_sub(1));
                    }
                    if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
                        self.selected_index = self.selected_index.saturating_sub(1);
                    }
                    if self.selected_index >= sorted_commands.len() {
                         self.selected_index = 0;
                    }

                    if ctx.input(|i| i.key_pressed(Key::Enter)) {
                         if let Some((_, cmd, _)) = sorted_commands.get(self.selected_index) {
                             action_to_execute = Some(cmd.action.clone());
                             self.is_open = false;
                         }
                    }

                    ScrollArea::vertical().show(ui, |ui| {
                        for (i, (_, cmd, _)) in sorted_commands.iter().enumerate() {
                            let is_selected = i == self.selected_index;
                            let text = if is_selected {
                                RichText::new(&cmd.name).strong().background_color(egui::Color32::from_rgb(60, 60, 60))
                            } else {
                                RichText::new(&cmd.name)
                            };

                            if ui.selectable_label(is_selected, text).clicked() {
                                action_to_execute = Some(cmd.action.clone());
                                self.is_open = false;
                            }
                        }
                    });
                }
            });

        self.is_open = open;
        action_to_execute
    }
}
