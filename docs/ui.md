# `src/ui.rs` - Legacy Layout Geometry & Hit-Testing Utilities

## Overview
`src/ui.rs` provides standalone math utilities for 2D geometry bounding box calculations (`Rect2D`) and hit-testing coordinate checks.

## Key Components

### `Rect2D`
```rust
pub struct Rect2D {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
```
- **`contains(px, py)`:** Evaluates whether coordinate point $(px, py)$ lies within the bounding box $[x, x + width] \times [y, y + height]$.

### `UiButton`
Enum defining UI interaction hit-test targets:
- `AddFavorite`
- `PlayFile`
- `FavoriteItem(usize)`
- `RenameFavorite(usize)`
- `DeleteFavorite(usize)`
- `PrevPage`
- `NextPage`

### `MainViewLayout`
Utility struct that computes static screen region bounds (Header, Scroll Cards, Action Bar, Pagination Bar) given screen dimensions `(width, height)`.

## Role in Current Architecture
With the migration to `egui` in `src/ui_renderer.rs`, layout math and hit-testing are managed directly by `egui::Context` widget layout passes. `src/ui.rs` remains in the codebase as a standalone layout calculation fallback reference.
