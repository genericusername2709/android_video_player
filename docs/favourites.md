# `src/favourites.rs` - Data Model & Storage Persistence

## Overview
`src/favourites.rs` defines the data structure for saved media items (`FavoriteMediaFile`) and manages thread-safe JSON file persistence in Android app internal storage.

## Data Models

### `FavoriteMediaFile`
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoriteMediaFile {
    pub uri: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub media_type: String,
    pub added_date: String,
    pub last_position_ms: u64,
}
```
- **`uri`:** The persistent `content://` or `file://` URI path pointing to the media file.
- **`display_name`:** User-facing file title.
- **`size_bytes`:** File size formatted dynamically into KB / MB / GB via `formatted_size()`.
- **`media_type`:** Type string (`Video` or `Audio`).
- **`last_position_ms`:** Saved playback timestamp in milliseconds.

### `FavoritesManager`
Thread-safe wrapper (`RwLock<Vec<FavoriteMediaFile>>`) that manages list mutations and disk synchronization:
- **`load_from_storage(app)`:** Reads `/data/user/0/com.example.android_video_player/files/favourites.json` at app launch.
- **`save_to_storage(app)`:** Writes the JSON string array back to internal files directory atomically on mutation.
- **`add_favorite(item, app)`:** Appends a new item if URI is not already present.
- **`remove_favorite(index, app)`:** Deletes item at `index` and saves changes.
- **`rename_favorite(index, new_title, app)`:** Renames item title at `index` and saves changes.
- **`update_position(index, position_ms, app)`:** Updates playback timestamp at `index` and saves changes.

## Interactions with Other Modules
- **`src/lib.rs`:** `FavoritesManager` is initialized in `android_main`. Its content is queried every frame to pass to `draw_ui()`.
- **`src/ui_renderer.rs`:** Receives `&[FavoriteMediaFile]` to render item titles, badges, sizes, and saved timestamps.
- **`src/media_picker.rs`:** `resolve_favorite_media_file()` constructs `FavoriteMediaFile` instances from Android ContentProvider queries.
