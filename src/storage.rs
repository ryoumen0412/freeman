use crate::models::{Collection, Environment, HistoryEntry};
use anyhow::Result;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

const MAX_HISTORY: usize = 50;

/// Manages request history and file storage
pub struct Storage {
    pub history: VecDeque<HistoryEntry>,
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub current_env: Option<usize>,
    config_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Self {
        let config_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".freeman");

        let mut storage = Storage {
            history: VecDeque::with_capacity(MAX_HISTORY),
            collections: Vec::new(),
            environments: Vec::new(),
            current_env: None,
            config_dir,
        };

        // Try to load saved data
        let _ = storage.load_all();
        storage
    }

    /// Add entry to history
    pub fn add_to_history(&mut self, entry: HistoryEntry) {
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_back();
        }
        self.history.push_front(entry);
    }

    /// Get current environment
    pub fn current_environment(&self) -> Option<&Environment> {
        self.current_env.and_then(|i| self.environments.get(i))
    }

    /// Substitute variables in text using current environment
    #[allow(dead_code)] // Prepared for future environment variable feature
    pub fn substitute(&self, text: &str) -> String {
        if let Some(env) = self.current_environment() {
            env.substitute(text)
        } else {
            text.to_string()
        }
    }

    /// Ensure config directory exists
    #[allow(dead_code)] // Used by save methods
    fn ensure_dir(&self) -> Result<()> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir)?;
        }
        Ok(())
    }

    /// Save a collection to file
    #[allow(dead_code)] // Prepared for future collection persistence
    pub fn save_collection(&self, collection: &Collection) -> Result<()> {
        self.ensure_dir()?;
        let path = self.config_dir.join(format!("{}.yaml", collection.name));
        let content = serde_yml::to_string(collection)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Save an environment to file
    #[allow(dead_code)] // Prepared for future environment persistence
    pub fn save_environment(&self, environment: &Environment) -> Result<()> {
        self.ensure_dir()?;
        let path = self
            .config_dir
            .join(format!("{}.env.yaml", environment.name));
        let content = serde_yml::to_string(environment)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load all collections and environments from disk
    pub fn load_all(&mut self) -> Result<()> {
        if !self.config_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.config_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.ends_with(".env.yaml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(env) = serde_yml::from_str::<Environment>(&content) {
                            self.environments.push(env);
                        }
                    }
                } else if filename.ends_with(".yaml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(col) = serde_yml::from_str::<Collection>(&content) {
                            self.collections.push(col);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get history item by index (0 = most recent)
    pub fn get_history(&self, index: usize) -> Option<&HistoryEntry> {
        self.history.get(index)
    }

    /// History length
    pub fn history_len(&self) -> usize {
        self.history.len()
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}
