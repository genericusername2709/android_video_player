# `src/ui_renderer.rs` - GPU UI Renderer & Layout Components

## Overview
`src/ui_renderer.rs` implements `EguiWgpuRenderer`, which encapsulates `egui::Context` and `egui_wgpu::Renderer`. It renders the application user interface on top of the WGPU hardware pipeline.

## Key Responsibilities

### 1. Visual Theme Configuration
Configures a dark theme with custom palette:
- **Window/Panel Fill:** Dark slate (`#12131A`).
- **Cards/Containers:** Indigo grey (`#222432`).
- **Primary Actions:** Material Purple (`#6750A4`) for Add Favorite, Material Teal (`#00BFA5`) for Play File.
- **Item Action Badges:** Solid Red (`#FF5252`) for Delete (`🗑`), Purple (`#6750A4`) for Rename (`✏`).

### 2. Layout Structure (`draw_ui`)
- **Fixed Bottom Action Bar:** `TopBottomPanel::bottom("bottom_actions")` anchored at screen bottom containing `[➕ ADD FAVOURITE]` and `[📁 PLAY FILE]`.
- **Fixed Pagination Bar:** `TopBottomPanel::bottom("fixed_pagination_bar")` positioned directly above the bottom action bar. Limits list display to 5 items per page with `< PREV` and `NEXT >` controls.
- **Scrollable Favorites List:** `CentralPanel::default()` containing card items. Each card displays:
  - Global index number, display title, media type badge (`AUDIO` / `VIDEO`), file size, and saved playback position timestamp (`Pos MM:SS`).
  - `✏` Rename button.
  - `🗑` Delete button.

### 3. Stable Scoped Widget IDs
- Every favorite card uses `ui.push_id(&item.uri, |ui| { ... })`.
- Scoping IDs strictly to `item.uri` (and omitting array indices) ensures that widget IDs remain 100% stable when items around them are added, renamed, or deleted.

### 4. Single-Tap Detection Helper (`is_widget_tapped`)
- Provides robust tap handling across desktop pointer events and mobile touch events:
  ```rust
  fn is_widget_tapped(resp: &egui::Response, ui: &egui::Ui) -> bool {
      if resp.clicked() {
          return true;
      }
      let pointer = ui.input(|i| i.pointer.clone());
      if pointer.any_released() {
          if let Some(pos) = pointer.latest_pos() {
              if resp.rect.contains(pos) {
                  return true;
              }
          }
      }
      false
  }
  ```

## Interactions with Other Modules
- **`src/lib.rs`:** `EguiWgpuRenderer` is owned by `RenderState` in `src/lib.rs`. `draw_ui()` returns an `EguiAppAction` variant to `src/lib.rs` for execution.
- **`src/favourites.rs`:** Reads `&[FavoriteMediaFile]` slice to build the rendered card views.
