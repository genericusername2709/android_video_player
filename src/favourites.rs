use android_activity::AndroidApp;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FavoriteMediaFile {
    pub uri: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub media_type: String,
    pub added_date: String,
    #[serde(default)]
    pub last_position_ms: u64,
}

#[allow(dead_code)]
impl FavoriteMediaFile {
    pub fn formatted_size(&self) -> String {
        if self.size_bytes == 0 {
            return "Unknown size".to_string();
        }
        let kb = self.size_bytes as f64 / 1024.0;
        let mb = kb / 1024.0;
        if mb >= 1.0 {
            format!("{:.2} MB", mb)
        } else {
            format!("{:.1} KB", kb)
        }
    }
}

#[derive(Debug, Clone)]
pub struct FavoritesManager {
    favorites: Arc<Mutex<Vec<FavoriteMediaFile>>>,
}

#[allow(dead_code)]
impl FavoritesManager {
    pub fn new() -> Self {
        Self {
            favorites: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Resolves local app storage path (internal_data_path / favourites.json)
    fn get_storage_path(app: &AndroidApp) -> PathBuf {
        let dir = app.internal_data_path().unwrap_or_else(|| {
            PathBuf::from("/data/user/0/com.example.android_video_player/files")
        });
        if !dir.exists() {
            let _ = fs::create_dir_all(&dir);
        }
        dir.join("favourites.json")
    }

    /// Load favorites list from persistent app storage
    pub fn load_from_storage(&self, app: &AndroidApp) {
        let path = Self::get_storage_path(app);
        info!("Loading favourites list from app storage: {:?}", path);

        if !path.exists() {
            info!("No favourites file found yet at {:?}, starting with empty list", path);
            return;
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<Vec<FavoriteMediaFile>>(&content) {
                Ok(items) => {
                    info!("Successfully loaded {} favourite files from app storage", items.len());
                    let mut list = self.favorites.lock().unwrap();
                    *list = items;
                }
                Err(e) => {
                    error!("Failed to parse favourites JSON file: {:?}", e);
                }
            },
            Err(e) => {
                error!("Failed to read favourites file from app storage: {:?}", e);
            }
        }
    }

    /// Save current favorites list to persistent app storage
    pub fn save_to_storage(&self, app: &AndroidApp) -> bool {
        let path = Self::get_storage_path(app);
        let list = self.favorites.lock().unwrap();

        match serde_json::to_string_pretty(&*list) {
            Ok(json) => match fs::write(&path, json) {
                Ok(_) => {
                    info!(
                        "Successfully saved {} favourite items to app storage ({:?})",
                        list.len(),
                        path
                    );
                    true
                }
                Err(e) => {
                    error!("Failed to write favourites to app storage: {:?}", e);
                    false
                }
            },
            Err(e) => {
                error!("Failed to serialize favourites to JSON: {:?}", e);
                false
            }
        }
    }

    /// Add a new media file to favourites and persist to storage
    pub fn add_favorite(&self, file: FavoriteMediaFile, app: &AndroidApp) -> bool {
        {
            let mut list = self.favorites.lock().unwrap();
            if list.iter().any(|f| f.uri == file.uri) {
                info!("File is already in favourites list: {}", file.display_name);
                return false;
            }
            info!("Adding new file to favourites: {}", file.display_name);
            list.push(file);
        }
        self.save_to_storage(app)
    }

    /// Remove a file from favourites by index and update storage
    pub fn remove_favorite(&self, index: usize, app: &AndroidApp) -> bool {
        {
            let mut list = self.favorites.lock().unwrap();
            if index < list.len() {
                let removed = list.remove(index);
                info!("Removed favourite file: {}", removed.display_name);
            } else {
                return false;
            }
        }
        self.save_to_storage(app)
    }

    /// Update playback position timestamp for a favorite file by index and persist
    pub fn update_position(&self, index: usize, position_ms: u64, app: &AndroidApp) -> bool {
        {
            let mut list = self.favorites.lock().unwrap();
            if index < list.len() {
                list[index].last_position_ms = position_ms;
                info!(
                    "Updated playback position for favourite index {}: {} ms",
                    index, position_ms
                );
            } else {
                return false;
            }
        }
        self.save_to_storage(app)
    }

    /// Rename a favorite file title by index and persist
    pub fn rename_favorite(&self, index: usize, new_title: &str, app: &AndroidApp) -> bool {
        {
            let mut list = self.favorites.lock().unwrap();
            if index < list.len() {
                info!(
                    "Renaming favourite at index {} from '{}' to '{}'",
                    index, list[index].display_name, new_title
                );
                list[index].display_name = new_title.to_string();
            } else {
                return false;
            }
        }
        self.save_to_storage(app)
    }

    /// Clear all favourites and update storage
    pub fn clear_all(&self, app: &AndroidApp) -> bool {
        {
            let mut list = self.favorites.lock().unwrap();
            list.clear();
        }
        self.save_to_storage(app)
    }

    /// Get a clone of all current favourite files
    pub fn get_all(&self) -> Vec<FavoriteMediaFile> {
        let list = self.favorites.lock().unwrap();
        list.clone()
    }

    /// Get total number of favourite files stored
    pub fn count(&self) -> usize {
        let list = self.favorites.lock().unwrap();
        list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }
}

pub fn current_formatted_date() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("Saved @ {}", now)
}
