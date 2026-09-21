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

    /// Create a Storage instance pointing at a custom directory (used in tests).
    #[cfg(test)]
    pub fn with_dir(config_dir: PathBuf) -> Self {
        Storage {
            history: VecDeque::with_capacity(MAX_HISTORY),
            collections: Vec::new(),
            environments: Vec::new(),
            current_env: None,
            config_dir,
        }
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
    #[allow(dead_code)]
    fn ensure_dir(&self) -> Result<()> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir)?;
        }
        Ok(())
    }

    /// Save a collection to file
    #[allow(dead_code)]
    pub fn save_collection(&self, collection: &Collection) -> Result<()> {
        self.ensure_dir()?;
        let path = self.config_dir.join(format!("{}.yaml", collection.name));
        let content = serde_yaml::to_string(collection)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Add a collection and persist to disk
    #[allow(dead_code)]
    pub fn add_collection(&mut self, collection: Collection) -> Result<()> {
        self.save_collection(&collection)?;
        self.collections.push(collection);
        Ok(())
    }

    /// Save an environment to file
    #[allow(dead_code)]
    pub fn save_environment(&self, environment: &Environment) -> Result<()> {
        self.ensure_dir()?;
        let path = self
            .config_dir
            .join(format!("{}.env.yaml", environment.name));
        let content = serde_yaml::to_string(environment)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Add an environment and persist to disk
    #[allow(dead_code)]
    pub fn add_environment(&mut self, environment: Environment) -> Result<()> {
        self.save_environment(&environment)?;
        self.environments.push(environment);
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
                        if let Ok(env) = serde_yaml::from_str::<Environment>(&content) {
                            self.environments.push(env);
                        }
                    }
                } else if filename.ends_with(".yaml") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(col) = serde_yaml::from_str::<Collection>(&content) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Collection, Environment, HistoryEntry, HttpMethod, Request, Response};
    use tempfile::tempdir;

    fn make_entry() -> HistoryEntry {
        HistoryEntry {
            request: Request::default(),
            response: Response::default(),
            timestamp: chrono::Utc::now(),
        }
    }

    // ── History ──────────────────────────────────────────────────────────────

    #[test]
    fn test_history_add_single() {
        let mut storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        storage.add_to_history(make_entry());
        assert_eq!(storage.history_len(), 1);
    }

    #[test]
    fn test_history_lifo_order() {
        let mut storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        let mut req_a = Request::default();
        req_a.url = "https://a.example.com".to_string();
        let mut req_b = Request::default();
        req_b.url = "https://b.example.com".to_string();

        storage.add_to_history(HistoryEntry {
            request: req_a,
            response: Response::default(),
            timestamp: chrono::Utc::now(),
        });
        storage.add_to_history(HistoryEntry {
            request: req_b,
            response: Response::default(),
            timestamp: chrono::Utc::now(),
        });

        // Most recent (b) should be at index 0
        assert_eq!(
            storage.get_history(0).unwrap().request.url,
            "https://b.example.com"
        );
    }

    #[test]
    fn test_history_respects_max_limit() {
        let mut storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        // Add MAX_HISTORY + 1 entries
        for _ in 0..=MAX_HISTORY {
            storage.add_to_history(make_entry());
        }
        assert_eq!(storage.history_len(), MAX_HISTORY);
    }

    #[test]
    fn test_get_history_valid_index() {
        let mut storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        storage.add_to_history(make_entry());
        assert!(storage.get_history(0).is_some());
    }

    #[test]
    fn test_get_history_out_of_range() {
        let storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        assert!(storage.get_history(999).is_none());
    }

    // ── Environment ──────────────────────────────────────────────────────────

    #[test]
    fn test_current_environment_none_by_default() {
        let storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        assert!(storage.current_environment().is_none());
    }

    #[test]
    fn test_current_environment_returns_correct_env() {
        let mut storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        let env = Environment::new("production");
        storage.environments.push(env);
        storage.current_env = Some(0);

        let current = storage.current_environment();
        assert!(current.is_some());
        assert_eq!(current.unwrap().name, "production");
    }

    #[test]
    fn test_substitute_without_environment() {
        let storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        let result = storage.substitute("https://{{base_url}}/users");
        // No env active — text returned as-is
        assert_eq!(result, "https://{{base_url}}/users");
    }

    // ── Persistence (I/O) ───────────────────────────────────────────────────

    #[test]
    fn test_save_and_load_collection() {
        let dir = tempdir().unwrap();
        let mut storage = Storage::with_dir(dir.path().to_path_buf());

        let mut col = Collection::new("test-collection");
        col.requests.push(Request::default());
        storage.add_collection(col).unwrap();

        // Load into a fresh instance pointing at same dir
        let mut storage2 = Storage::with_dir(dir.path().to_path_buf());
        storage2.load_all().unwrap();

        assert_eq!(storage2.collections.len(), 1);
        assert_eq!(storage2.collections[0].name, "test-collection");
    }

    #[test]
    fn test_load_all_nonexistent_dir_does_not_fail() {
        let mut storage = Storage::with_dir(PathBuf::from("/nonexistent/path/that/does/not/exist"));
        let result = storage.load_all();
        assert!(result.is_ok());
    }

    #[test]
    fn test_save_and_load_environment() {
        let dir = tempdir().unwrap();
        let mut storage = Storage::with_dir(dir.path().to_path_buf());

        let mut env = Environment::new("staging");
        env.set("base_url", "https://staging.example.com");
        storage.add_environment(env).unwrap();

        let mut storage2 = Storage::with_dir(dir.path().to_path_buf());
        storage2.load_all().unwrap();

        assert_eq!(storage2.environments.len(), 1);
        assert_eq!(storage2.environments[0].name, "staging");
        assert_eq!(
            storage2.environments[0].get("base_url"),
            Some(&"https://staging.example.com".to_string())
        );
    }

    #[test]
    fn test_history_method_reflects_request() {
        let mut storage = Storage::with_dir(PathBuf::from("/tmp/freeman_test_unused"));
        let mut req = Request::default();
        req.method = HttpMethod::POST;
        req.url = "https://api.example.com/items".to_string();
        storage.add_to_history(HistoryEntry {
            request: req,
            response: Response::default(),
            timestamp: chrono::Utc::now(),
        });
        let entry = storage.get_history(0).unwrap();
        assert_eq!(entry.request.method, HttpMethod::POST);
        assert_eq!(entry.request.url, "https://api.example.com/items");
    }
}
