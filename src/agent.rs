
use std::collections::{HashMap, VecDeque, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use eframe::egui::{self, Color32};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AgentRole {
    Generalist,
    Specialist(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Thinking(String),
    Typing,
    Voting,
    Error(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub role: AgentRole,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub system_prompt: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub sender: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub config: AgentConfig,
    pub status: AgentStatus,
    pub current_task_id: Option<Uuid>,
    pub cursor_idx: Option<usize>,
    pub snapshot: Option<String>,
    // Chat
    pub chat_history: Vec<ChatMessage>,
    pub chat_input: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
    Blocked,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub description: String,
    pub status: TaskStatus,
    pub assignee: Option<Uuid>,
}

pub enum AgentEvent {
    LlmResponse(Uuid, String), // AgentID, ResponseBody
    ChatResponse(Uuid, String), // AgentID, ResponseBody
    PlanResponse(Uuid, String), // AgentID, ResponseBody (List of tasks)
    LlmError(Uuid, String),
}

pub struct AgentSystem {
    pub agents: HashMap<Uuid, Agent>,
    pub local_ids: HashSet<Uuid>,
    pub tasks: Vec<Task>,
    pub locks: HashMap<Uuid, Uuid>,
    pub message_log: VecDeque<String>,
    pub pending_changes: VecDeque<(Uuid, String)>,
    pub last_poll: f64,
    pub event_queue: Arc<Mutex<VecDeque<AgentEvent>>>, // Shared with async callbacks
}

impl Default for AgentSystem {
    fn default() -> Self {
        Self {
            agents: HashMap::new(),
            local_ids: HashSet::new(),
            tasks: Vec::new(),
            locks: HashMap::new(),
            message_log: VecDeque::new(),
            pending_changes: VecDeque::new(),
            last_poll: 0.0,
            event_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl AgentSystem {
    pub fn add_agent(&mut self, config: AgentConfig) {
        let id = Uuid::new_v4();
        self.agents.insert(id, Agent {
            id,
            config,
            status: AgentStatus::Idle,
            current_task_id: None,
            cursor_idx: None,
            snapshot: None,
            chat_history: Vec::new(),
            chat_input: String::new(),
        });
        self.local_ids.insert(id);
        self.log(format!("Agent added: {}", self.agents.get(&id).unwrap().config.name));
    }

    pub fn add_task(&mut self, description: String) {
        let id = Uuid::new_v4();
        self.tasks.push(Task {
            id,
            description,
            status: TaskStatus::Pending,
            assignee: None,
        });
        self.log(format!("Task added: {}", self.tasks.last().unwrap().description));
    }

    pub fn log(&mut self, msg: String) {
        self.message_log.push_back(msg);
        if self.message_log.len() > 100 {
            self.message_log.pop_front();
        }
    }

    pub fn poll(&mut self, time: f64, current_code: &str) {
        // 1. Process Events from Async Callbacks
        let mut events = Vec::new();
        if let Ok(mut queue) = self.event_queue.lock() {
            while let Some(evt) = queue.pop_front() {
                events.push(evt);
            }
        }

        for event in events {
            match event {
                AgentEvent::LlmResponse(aid, body) => {
                    if let Some(agent) = self.agents.get_mut(&aid) {
                        // Mock parsing of response. Assuming raw text for now or simple JSON field "response"
                        let code_snippet = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                             v["response"].as_str().unwrap_or(&body).to_string()
                        } else {
                             body
                        };

                        agent.status = AgentStatus::Idle;
                        // Mark task done
                        if let Some(tid) = agent.current_task_id {
                             if let Some(t) = self.tasks.iter_mut().find(|t| t.id == tid) {
                                 t.status = TaskStatus::Done;
                                 self.message_log.push_back(format!("{} finished task.", agent.config.name));
                             }
                        }
                        agent.current_task_id = None;

                        // Queue the code change
                        self.pending_changes.push_back((aid, code_snippet));
                    }
                }
                AgentEvent::ChatResponse(aid, body) => {
                    if let Some(agent) = self.agents.get_mut(&aid) {
                        let response_text = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                             v["response"].as_str().unwrap_or(&body).to_string()
                        } else {
                             body
                        };
                        agent.chat_history.push(ChatMessage {
                            sender: agent.config.name.clone(),
                            content: response_text,
                        });
                        agent.status = AgentStatus::Idle;
                    }
                }
                AgentEvent::PlanResponse(aid, body) => {
                    if let Some(agent) = self.agents.get_mut(&aid) {
                        let response_text = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                             v["response"].as_str().unwrap_or(&body).to_string()
                        } else {
                             body
                        };

                        // Parse lines starting with "- " or "* "
                        let new_tasks: Vec<String> = response_text.lines()
                            .filter_map(|line| {
                                let trimmed = line.trim();
                                if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                                    Some(trimmed[2..].to_string())
                                } else if let Some(idx) = trimmed.find(". ") {
                                    // Handle "1. Task" format
                                    if trimmed[..idx].chars().all(char::is_numeric) {
                                        Some(trimmed[idx+2..].to_string())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                            .collect();

                        for desc in new_tasks {
                            self.tasks.push(Task {
                                id: Uuid::new_v4(),
                                description: desc,
                                status: TaskStatus::Pending,
                                assignee: None,
                            });
                        }

                        self.message_log.push_back(format!("{} generated a plan.", agent.config.name));
                        agent.status = AgentStatus::Idle;
                    }
                }
                AgentEvent::LlmError(aid, err) => {
                    if let Some(agent) = self.agents.get_mut(&aid) {
                        agent.status = AgentStatus::Error(err.clone());
                        self.message_log.push_back(format!("Agent {} error: {}", agent.config.name, err));
                    }
                }
            }
        }

        // 2. Throttle Poll
        if time - self.last_poll < 1.0 {
            return;
        }
        self.last_poll = time;

        // 3. Assign Tasks
        let idle_agents: Vec<Uuid> = self.agents.iter()
            .filter(|(id, a)| self.local_ids.contains(id) && a.status == AgentStatus::Idle)
            .map(|(id, _)| *id)
            .collect();

        for agent_id in idle_agents {
            if let Some(task_idx) = self.tasks.iter().position(|t| t.status == TaskStatus::Pending) {
                let task_id = self.tasks[task_idx].id;
                let task_desc = self.tasks[task_idx].description.clone();

                // 1. Update State (Mutate)
                let request_data = if let Some(agent) = self.agents.get_mut(&agent_id) {
                    self.tasks[task_idx].status = TaskStatus::InProgress;
                    self.tasks[task_idx].assignee = Some(agent_id);

                    agent.status = AgentStatus::Thinking("Requesting LLM...".to_string());
                    agent.current_task_id = Some(task_id);
                    agent.snapshot = Some(current_code.to_string());

                    Some((agent.config.clone(), task_desc))
                } else {
                    None
                };

                // 2. Perform Actions (Immutable/No Borrow)
                if let Some((config, desc)) = request_data {
                    self.log(format!("{} started: {}", config.name, desc));
                    self.spawn_llm_request(agent_id, config, &desc, current_code);
                }
            }
        }
    }

    pub fn spawn_llm_request(&self, agent_id: Uuid, config: AgentConfig, task: &str, code: &str) {
        let prompt = format!("Task: {}\nCode:\n{}", task, code);

        let body_json = serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "stream": false
        });
        let mut request = ehttp::Request::post(
            format!("{}/api/generate", config.base_url),
            serde_json::to_vec(&body_json).unwrap_or_default(),
        );
        request.headers.insert("Content-Type".to_string(), "application/json".to_string());

        let queue_clone = self.event_queue.clone();

        ehttp::fetch(request, move |result: ehttp::Result<ehttp::Response>| {
            let event = match result {
                Ok(response) => {
                    if response.status == 200 {
                        // Try parse text
                        let text = response.text().unwrap_or_default();
                        AgentEvent::LlmResponse(agent_id, text.to_string())
                    } else {
                        AgentEvent::LlmError(agent_id, format!("Status: {}", response.status))
                    }
                }
                Err(e) => AgentEvent::LlmError(agent_id, e),
            };

            if let Ok(mut q) = queue_clone.lock() {
                q.push_back(event);
            }
        });
    }

    pub fn spawn_planning_request(&self, agent_id: Uuid, config: AgentConfig, goal: &str, current_code: &str) {
        let prompt = format!(
            "You are a technical lead. Your goal is: {}\n\
            Current Code:\n{}\n\
            Break this goal down into a list of small, actionable coding tasks.\n\
            Format the output as a bulleted list (start lines with '- ').\n\
            Do not include extra chatter, just the list.",
            goal, current_code
        );

        let body_json = serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "stream": false
        });

        let mut request = ehttp::Request::post(
            format!("{}/api/generate", config.base_url),
            serde_json::to_vec(&body_json).unwrap_or_default(),
        );
        request.headers.insert("Content-Type".to_string(), "application/json".to_string());

        let queue_clone = self.event_queue.clone();

        ehttp::fetch(request, move |result: ehttp::Result<ehttp::Response>| {
            let event = match result {
                Ok(response) => {
                    if response.status == 200 {
                         let text = response.text().unwrap_or_default();
                         AgentEvent::PlanResponse(agent_id, text.to_string())
                    } else {
                         AgentEvent::LlmError(agent_id, format!("Status: {}", response.status))
                    }
                }
                Err(e) => AgentEvent::LlmError(agent_id, e),
            };

            if let Ok(mut q) = queue_clone.lock() {
                q.push_back(event);
            }
        });
    }

    pub fn spawn_chat_request(&self, agent_id: Uuid, config: AgentConfig, history: Vec<ChatMessage>) {
        // Build a prompt from history
        // For standard Ollama/Llama, prompt format is just appending text or using template.
        // Here we just concat for simplicity:
        let mut prompt = String::new();
        if !config.system_prompt.is_empty() {
            prompt.push_str(&format!("System: {}\n", config.system_prompt));
        }
        for msg in history {
             prompt.push_str(&format!("{}: {}\n", msg.sender, msg.content));
        }

        let body_json = serde_json::json!({
            "model": config.model,
            "prompt": prompt,
            "stream": false
        });

        let mut request = ehttp::Request::post(
            format!("{}/api/generate", config.base_url),
            serde_json::to_vec(&body_json).unwrap_or_default(),
        );
        request.headers.insert("Content-Type".to_string(), "application/json".to_string());

        let queue_clone = self.event_queue.clone();

        ehttp::fetch(request, move |result: ehttp::Result<ehttp::Response>| {
            let event = match result {
                Ok(response) => {
                    if response.status == 200 {
                         let text = response.text().unwrap_or_default();
                         AgentEvent::ChatResponse(agent_id, text.to_string())
                    } else {
                         AgentEvent::LlmError(agent_id, format!("Status: {}", response.status))
                    }
                }
                Err(e) => AgentEvent::LlmError(agent_id, e),
            };

            if let Ok(mut q) = queue_clone.lock() {
                q.push_back(event);
            }
        });
    }
}

// UI Helpers
impl AgentSystem {
    pub fn ui_agents(&mut self, ui: &mut egui::Ui) {
        ui.heading("Active Agents");
        let mut agent_chat_req: Option<(Uuid, AgentConfig, Vec<ChatMessage>)> = None;

        for (id, agent) in self.agents.iter_mut() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&agent.config.name).strong());
                    let status_text = match &agent.status {
                        AgentStatus::Idle => egui::RichText::new("Idle").color(Color32::GRAY),
                        AgentStatus::Thinking(s) => egui::RichText::new(format!("Thinking: {}", s)).color(Color32::YELLOW),
                        AgentStatus::Typing => egui::RichText::new("Typing...").color(Color32::GREEN),
                        AgentStatus::Voting => egui::RichText::new("Voting").color(Color32::LIGHT_BLUE),
                        AgentStatus::Error(e) => egui::RichText::new(format!("Error: {}", e)).color(Color32::RED),
                    };
                    ui.label(status_text);
                });
                if let Some(tid) = agent.current_task_id {
                     if let Some(task) = self.tasks.iter().find(|t| t.id == tid) {
                         ui.label(format!("Working on: {}", task.description));
                     }
                }

                // Chat UI
                ui.collapsing("Chat", |ui| {
                     egui::ScrollArea::vertical().id_salt(format!("chat_scroll_{}", id)).max_height(200.0).show(ui, |ui| {
                         for msg in &agent.chat_history {
                             ui.horizontal(|ui| {
                                 ui.strong(format!("{}: ", msg.sender));
                                 ui.label(&msg.content);
                             });
                         }
                     });
                     ui.horizontal(|ui| {
                         ui.text_edit_singleline(&mut agent.chat_input);
                         if ui.button("Send").clicked() && !agent.chat_input.is_empty() {
                             let content = agent.chat_input.clone();
                             agent.chat_input.clear();

                             agent.chat_history.push(ChatMessage {
                                 sender: "User".to_string(),
                                 content: content,
                             });

                             agent.status = AgentStatus::Thinking("Chatting...".to_string());
                             agent_chat_req = Some((*id, agent.config.clone(), agent.chat_history.clone()));
                         }
                     });
                });
            });
        }

        if let Some((id, config, history)) = agent_chat_req {
             self.spawn_chat_request(id, config, history);
        }
    }

    pub fn ui_tasks(&mut self, ui: &mut egui::Ui) {
        ui.heading("ToDo List");
        for (_i, task) in self.tasks.iter().enumerate() {
            ui.horizontal(|ui| {
                let icon = match task.status {
                    TaskStatus::Pending => "⬜",
                    TaskStatus::InProgress => "🔄",
                    TaskStatus::Done => "✅",
                    TaskStatus::Blocked => "⛔",
                };
                ui.label(icon);
                ui.label(&task.description);
                if let Some(aid) = task.assignee {
                     if let Some(agent) = self.agents.get(&aid) {
                         ui.small(format!("({})", agent.config.name));
                     }
                }
            });
        }
    }
}
