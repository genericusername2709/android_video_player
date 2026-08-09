# Architecture & Operational Summary - Android GPU Video Player App

## 1. What This App Does

The **Android GPU Video Player** is a high-performance native Android application written in **Rust** (compiled to `libandroid_video_player.so`) and **Java** (`MainActivity` and `VideoPlayerActivity`). 

### Core Features
1. **Favorites Media Library:**
   - Add local video and audio files from Android storage into a persistent favorites list using the system Storage Access Framework (SAF) Document Picker.
   - Persists metadata (display title, file size, media type, creation date, and last playback timestamp) in JSON format at `/data/user/0/com.example.android_video_player/files/favourites.json`.
2. **In-App Media Playback & Resume:**
   - Dedicated `VideoPlayerActivity` utilizing Android `VideoView` and `MediaController`.
   - Tracks exact playback position in milliseconds (`last_position`) continuously.
   - Resumes video playback automatically from the exact saved timestamp when a favorite is launched.
3. **Single-Tap Management Controls:**
   - **Rename Favorite:** Native Android `AlertDialog` with text input to rename favorite titles.
   - **Delete Favorite:** Native Android confirmation `AlertDialog` modal to remove favorites safely.
   - **Pagination:** Fixed 5-item pagination control bar directly above bottom action buttons (`< PREV`, `Page X of Y`, `NEXT >`).
4. **Direct Media Playback:**
   - Play any video or audio file directly from Android storage via `[📁 PLAY FILE]`.

---

## 2. System Architecture & Interaction Flow

```
+-------------------------------------------------------------------------------+
|                             Android OS Framework                              |
+-------------------+-----------------------------------+-----------------------+
                    |                                   |
                    v                                   v
+-------------------+-------------------+   +-----------+-----------------------+
|  MainActivity.java (NativeActivity)   |   |    VideoPlayerActivity.java       |
|  - JNI Bridges                        |   |    - VideoView & MediaController  |
|  - SAF Intent Picker                  |   |    - Position Tracker Runnable    |
|  - Rename & Delete Alert Dialogs      |   |    - Returns last_position        |
+-------------------+-------------------+   +-----------+-----------------------+
                    |                                   ^
                    | JNI Calls                         | Intent Launch
                    v                                   |
+-------------------+-----------------------------------+-----------------------+
|                            media_picker.rs                                    |
|  - Rust JNI Wrapper Functions                                                 |
|  - ContentResolver Metadata Query (_display_name, _size, MIME type)           |
+-------------------+-----------------------------------------------------------+
                    |
                    v
+-------------------+-----------------------------------------------------------+
|                                lib.rs                                         |
|  - android_main Event Loop                                                   |
|  - WGPU Surface & Device Initialization (Max 2048 Texture Size)               |
|  - Touch Event Dispatcher (PointerMoved, PointerButton, Touch)                |
|  - Lifecycle Handlers (Pause, Resume, Destroy)                                |
+-------------------+-------------------+---------------------------------------+
                    |                   |
                    v                   v
+-------------------+-------+   +-------+---------------------------------------+
|    ui_renderer.rs         |   |             favourites.rs                     |
|  - EguiWgpuRenderer       |   |  - FavoriteMediaFile Model                    |
|  - Material Dark Theme    |   |  - FavoritesManager (JSON File I/O)           |
|  - Stable Scoped IDs      |   |  - Timestamp & Rename Storage Updates         |
|  - is_widget_tapped()     |   +-----------------------------------------------+
+---------------------------+
```

---

## 3. Detailed Component Breakdown

| Module | Location | Primary Role |
| :--- | :--- | :--- |
| **`lib.rs`** | [`src/lib.rs`](file:///home/mustafa/projects/rust-stuff/break-player/android_video_player/src/lib.rs) | Core entry point, event loop, WGPU initialization, touch event translation, and action dispatching. |
| **`ui_renderer.rs`** | [`src/ui_renderer.rs`](file:///home/mustafa/projects/rust-stuff/break-player/android_video_player/src/ui_renderer.rs) | `egui` UI rendering pass, Material Dark theme, fixed pagination bar, action buttons, item cards, and single-tap detection. |
| **`favourites.rs`** | [`src/favourites.rs`](file:///home/mustafa/projects/rust-stuff/break-player/android_video_player/src/favourites.rs) | JSON persistence (`favourites.json`), thread-safe list mutations (`add`, `remove`, `rename`, `update_position`). |
| **`media_picker.rs`** | [`src/media_picker.rs`](file:///home/mustafa/projects/rust-stuff/break-player/android_video_player/src/media_picker.rs) | JNI bridge to Android framework (File Picker Intent, ContentResolver metadata query, `AlertDialog` modals, playback launching). |
| **`ui.rs`** | [`src/ui.rs`](file:///home/mustafa/projects/rust-stuff/break-player/android_video_player/src/ui.rs) | Standalone layout geometry math and 2D bounding box hit-testing helper functions. |
| **`MainActivity.java`** | [`java/.../MainActivity.java`](file:///home/mustafa/projects/rust-stuff/break-player/android_video_player/java/com/example/android_video_player/MainActivity.java) | `NativeActivity` subclass in Java bridging SAF file pickers, Android `AlertDialog` modals, and Activity intent lifecycle. |
| **`VideoPlayerActivity.java`** | [`java/.../VideoPlayerActivity.java`](file:///home/mustafa/projects/rust-stuff/break-player/android_video_player/java/com/example/android_video_player/VideoPlayerActivity.java) | Dedicated video playback activity running `VideoView` with `MediaController`, position tracking, and floating `[X]` exit button. |

---

