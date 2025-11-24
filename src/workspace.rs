use serde::{Deserialize, Serialize};
use crate::agent::Task;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    pub code: String,
    pub tasks: Vec<Task>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            code: String::new(),
            tasks: Vec::new(),
        }
    }
}
