use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    pub target: String,
    pub args: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,
    pub launch_count: u64,
}

impl Default for Shortcut {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: String::new(),
            target: String::new(),
            args: String::new(),
            category: "General".to_string(),
            icon_path: None,
            hotkey: None,
            launch_count: 0,
        }
    }
}

impl Shortcut {
    pub fn new(name: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            target: target.into(),
            ..Default::default()
        }
    }
}

pub fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ShortcutManager")
}

pub fn get_data_file() -> PathBuf {
    get_data_dir().join("shortcuts.json")
}

pub fn get_icon_cache_dir() -> PathBuf {
    get_data_dir().join("icon_cache")
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StoreError>;

pub struct ShortcutStore {
    shortcuts: Arc<RwLock<Vec<Shortcut>>>,
    file_path: PathBuf,
}

impl ShortcutStore {
    pub fn new(file_path: Option<PathBuf>) -> Result<Self> {
        let file_path = file_path.unwrap_or_else(get_data_file);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            shortcuts: Arc::new(RwLock::new(Vec::new())),
            file_path,
        };
        store.load()?;
        Ok(store)
    }

    fn load(&self) -> Result<()> {
        if self.file_path.exists() {
            let data = std::fs::read_to_string(&self.file_path)?;
            let shortcuts: Vec<Shortcut> = serde_json::from_str(&data)?;
            *self.shortcuts.write().unwrap() = shortcuts;
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let shortcuts = self.shortcuts.read().unwrap();
        let data = serde_json::to_string_pretty(&*shortcuts)?;
        std::fs::write(&self.file_path, data)?;
        Ok(())
    }

    pub fn get_all(&self) -> Vec<Shortcut> {
        self.shortcuts.read().unwrap().clone()
    }

    pub fn add(&self, shortcut: Shortcut) -> Result<()> {
        self.shortcuts.write().unwrap().push(shortcut);
        self.save()
    }

    pub fn update(&self, shortcut: Shortcut) -> Result<()> {
        let mut shortcuts = self.shortcuts.write().unwrap();
        if let Some(idx) = shortcuts.iter().position(|s| s.id == shortcut.id) {
            shortcuts[idx] = shortcut;
            drop(shortcuts);
            self.save()
        } else {
            Ok(())
        }
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        self.shortcuts.write().unwrap().retain(|s| s.id != id);
        self.save()
    }

    pub fn bump_launch_count(&self, id: &str) -> Result<()> {
        let mut shortcuts = self.shortcuts.write().unwrap();
        if let Some(s) = shortcuts.iter_mut().find(|s| s.id == id) {
            s.launch_count += 1;
            drop(shortcuts);
            self.save()
        } else {
            Ok(())
        }
    }

    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .shortcuts
            .read()
            .unwrap()
            .iter()
            .filter_map(|s| {
                if !s.category.is_empty() {
                    Some(s.category.clone())
                } else {
                    None
                }
            })
            .collect();
        cats.sort();
        cats.dedup();
        if cats.is_empty() {
            vec!["General".to_string()]
        } else {
            cats
        }
    }

    pub fn search(&self, query: &str, category: Option<&str>) -> Vec<Shortcut> {
        let query = query.trim().to_lowercase();
        let mut results: Vec<Shortcut> = self
            .shortcuts
            .read()
            .unwrap()
            .iter()
            .filter(|s| {
                let cat_match = category.map_or(true, |c| c == "All" || s.category == c);
                let query_match = query.is_empty()
                    || s.name.to_lowercase().contains(&query)
                    || s.category.to_lowercase().contains(&query);
                cat_match && query_match
            })
            .cloned()
            .collect();

        results.sort_by(|a, b| {
            b.launch_count
                .cmp(&a.launch_count)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        results
    }

    pub fn get_by_id(&self, id: &str) -> Option<Shortcut> {
        self.shortcuts.read().unwrap().iter().find(|s| s.id == id).cloned()
    }
}

impl Clone for ShortcutStore {
    fn clone(&self) -> Self {
        Self {
            shortcuts: Arc::clone(&self.shortcuts),
            file_path: self.file_path.clone(),
        }
    }
}