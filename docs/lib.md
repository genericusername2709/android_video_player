# `src/lib.rs` - Main Entry Point & Event Loop

## Overview
`src/lib.rs` is the central orchestrator of the Android video player application. It initializes the `android_activity` runtime, configures the WGPU GPU rendering surface, manages the event loop, translates Android touch input events into `egui` raw input, and handles application lifecycle events.

## Key Responsibilities

### 1. Application Entry Point (`android_main`)
- Annotated with `#[no_mangle] pub fn android_main(app: AndroidApp)` as required by `android_activity` / `NativeActivity`.
- Sets up logger (`android_logger::init_once`).
- Initializes `FavoritesManager` and loads saved favorites from app storage (`/data/user/0/com.example.android_video_player/files/favourites.json`).

### 2. GPU Render State (`RenderState`)
- Configures `wgpu::Instance`, `wgpu::Adapter`, `wgpu::Device`, and `wgpu::Queue`.
- Creates a `wgpu::Surface` bound to the native Android window (`ANativeWindow`).
- Clamps surface texture dimensions to the GPU max texture size (`2048x2048`) to avoid device driver panics.
- Houses `EguiWgpuRenderer` for UI rendering pass generation.

### 3. Touch Input Dispatcher
- Intercepts Android `MotionEvent` instances (`Down`, `Move`, `Up`).
- Maps raw screen pixel touch coordinates to logical `egui` points using `map_touch_pos()`.
- Forwards `egui::Event::PointerMoved`, `egui::Event::PointerButton`, and `egui::Event::Touch` (`TouchPhase::Start`, `Move`, `End`) to ensure 100% click/tap compatibility across egui widgets.

### 4. Application Lifecycle Management
- **`MainEvent::Pause` / `Destroy`:** Synchronizes the active video playback position (`sync_active_playback_position`) across JNI and persists it to JSON storage before suspending.
- **`MainEvent::Resume`:** Triggers `check_picker_result()`, `check_rename_updates()`, and `check_delete_updates()` to process intent data returned from file pickers or modal dialogs.

### 5. Action Dispatcher
Processes `EguiAppAction` values returned by `draw_ui()`:
- `AddFavorite` $\rightarrow$ Launches SAF File Picker via JNI.
- `PlayExternalFile` $\rightarrow$ Launches SAF File Picker for immediate playback.
- `PlayFavorite(idx)` $\rightarrow$ Launches `VideoPlayerActivity` with URI, timestamp, and title.
- `RenameFavorite(idx)` $\rightarrow$ Opens Android Native `AlertDialog` with EditText.
- `DeleteFavorite(idx)` $\rightarrow$ Opens Android Native confirmation `AlertDialog`.

## Interactions with Other Modules
- **`src/ui_renderer.rs`:** Calls `EguiWgpuRenderer::draw_ui()` and `render()` to generate WGPU render pass command buffers.
- **`src/favourites.rs`:** Mutates and reads `FavoritesManager` in response to user actions.
- **`src/media_picker.rs`:** Invokes JNI wrapper functions to communicate with `MainActivity.java`.
