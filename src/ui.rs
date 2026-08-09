#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect2D {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect2D {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= (self.x + self.width) && py >= self.y && py <= (self.y + self.height)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiButton {
    PlayFile,
    AddToFavorites,
    FavoriteItem(usize),
    RemoveFavorite(usize),
    RenameFavorite(usize),
    PrevPage,
    NextPage,
}

pub struct MainViewLayout {
    pub play_button_bounds: Rect2D,
    pub add_fav_button_bounds: Rect2D,
    pub favorite_item_bounds: Vec<(usize, Rect2D)>,
    pub remove_fav_item_bounds: Vec<(usize, Rect2D)>,
    pub rename_fav_item_bounds: Vec<(usize, Rect2D)>,
    pub prev_page_bounds: Rect2D,
    pub next_page_bounds: Rect2D,
    pub current_page: usize,
    pub total_pages: usize,
}

impl MainViewLayout {
    pub fn calculate(
        width: f32,
        height: f32,
        favorite_count: usize,
        current_page: usize,
    ) -> Self {
        let w = if width > 0.0 { width } else { 1080.0 };
        let h = if height > 0.0 { height } else { 1920.0 };

        let items_per_page = 5;
        let total_pages = if favorite_count == 0 {
            1
        } else {
            (favorite_count + items_per_page - 1) / items_per_page
        };
        let page = current_page.min(total_pages.saturating_sub(1));

        // Safe screen edge padding: 8% inset on left/right for round screen/notch safety
        let margin_x = w * 0.08;
        let card_w = w * 0.84;

        // Header / App Bar starts at y = h * 0.20 (20% down from top)
        // Position main Material Action Buttons safely above status bar
        let action_y = h * 0.785;
        let action_h = h * 0.075;
        let btn_width = card_w * 0.48;

        let play_button_bounds = Rect2D {
            x: margin_x,
            y: action_y,
            width: btn_width,
            height: action_h,
        };

        let add_fav_button_bounds = Rect2D {
            x: margin_x + card_w - btn_width,
            y: action_y,
            width: btn_width,
            height: action_h,
        };

        // Page navigation controls bounds
        let page_bar_y = h * 0.725;
        let page_bar_h = h * 0.050;
        let page_btn_w = card_w * 0.30;

        let prev_page_bounds = Rect2D {
            x: margin_x,
            y: page_bar_y,
            width: page_btn_w,
            height: page_bar_h,
        };

        let next_page_bounds = Rect2D {
            x: margin_x + card_w - page_btn_w,
            y: page_bar_y,
            width: page_btn_w,
            height: page_bar_h,
        };

        // Favorites list area items (5 items max per page)
        let mut favorite_item_bounds = Vec::new();
        let mut remove_fav_item_bounds = Vec::new();
        let mut rename_fav_item_bounds = Vec::new();

        let list_top = h * 0.355;
        let item_height = h * 0.065;
        let gap = h * 0.008;

        let start_idx = page * items_per_page;
        let end_idx = (start_idx + items_per_page).min(favorite_count);

        for (slot, global_idx) in (start_idx..end_idx).enumerate() {
            let item_y = list_top + (slot as f32 * (item_height + gap));
            let btn_w = card_w * 0.15;

            // Main card hit area
            favorite_item_bounds.push((
                global_idx,
                Rect2D {
                    x: margin_x,
                    y: item_y,
                    width: card_w,
                    height: item_height,
                },
            ));

            // Remove button hit area (right edge of card)
            remove_fav_item_bounds.push((
                global_idx,
                Rect2D {
                    x: margin_x + card_w - btn_w,
                    y: item_y,
                    width: btn_w,
                    height: item_height,
                },
            ));

            // Rename button hit area (left of remove button)
            rename_fav_item_bounds.push((
                global_idx,
                Rect2D {
                    x: margin_x + card_w - (btn_w * 2.0) - (card_w * 0.01),
                    y: item_y,
                    width: btn_w,
                    height: item_height,
                },
            ));
        }

        Self {
            play_button_bounds,
            add_fav_button_bounds,
            favorite_item_bounds,
            remove_fav_item_bounds,
            rename_fav_item_bounds,
            prev_page_bounds,
            next_page_bounds,
            current_page: page,
            total_pages,
        }
    }

    pub fn hit_test(&self, touch_x: f32, touch_y: f32) -> Option<UiButton> {
        // Check delete buttons first
        for (global_idx, bounds) in &self.remove_fav_item_bounds {
            if bounds.contains(touch_x, touch_y) {
                return Some(UiButton::RemoveFavorite(*global_idx));
            }
        }

        // Check rename buttons
        for (global_idx, bounds) in &self.rename_fav_item_bounds {
            if bounds.contains(touch_x, touch_y) {
                return Some(UiButton::RenameFavorite(*global_idx));
            }
        }

        // Check Prev / Next page buttons
        if self.current_page > 0 && self.prev_page_bounds.contains(touch_x, touch_y) {
            return Some(UiButton::PrevPage);
        }
        if self.current_page + 1 < self.total_pages
            && self.next_page_bounds.contains(touch_x, touch_y)
        {
            return Some(UiButton::NextPage);
        }

        if self.play_button_bounds.contains(touch_x, touch_y) {
            return Some(UiButton::PlayFile);
        }
        if self.add_fav_button_bounds.contains(touch_x, touch_y) {
            return Some(UiButton::AddToFavorites);
        }

        for (global_idx, bounds) in &self.favorite_item_bounds {
            if bounds.contains(touch_x, touch_y) {
                return Some(UiButton::FavoriteItem(*global_idx));
            }
        }

        None
    }
}
