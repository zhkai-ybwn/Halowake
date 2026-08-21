pub mod models;
pub mod planner;
pub mod prompt;
pub mod repository;
pub mod runner;
pub mod validator;

use std::{collections::HashMap, sync::Mutex};

use tokio::task::AbortHandle;

#[derive(Default)]
pub struct ReviewTaskRegistry {
    tasks: Mutex<HashMap<String, AbortHandle>>,
}

impl ReviewTaskRegistry {
    pub fn insert(&self, id: String, handle: AbortHandle) -> Result<(), String> {
        self.tasks
            .lock()
            .map_err(|_| "Review 任务状态不可用".to_string())?
            .insert(id, handle);
        Ok(())
    }

    pub fn cancel(&self, id: &str) -> Result<bool, String> {
        let handle = self
            .tasks
            .lock()
            .map_err(|_| "Review 任务状态不可用".to_string())?
            .remove(id);
        if let Some(handle) = handle {
            handle.abort();
            return Ok(true);
        }
        Ok(false)
    }

    pub fn remove(&self, id: &str) {
        if let Ok(mut tasks) = self.tasks.lock() {
            tasks.remove(id);
        }
    }
}
