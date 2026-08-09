use crate::favourites::FavoriteMediaFile;

#[derive(Debug, Clone, PartialEq)]
pub enum EguiAppAction {
    None,
    AddFavorite,
    PlayExternalFile,
    PlayFavorite(usize),
    RenameFavorite(usize),
    DeleteFavorite(usize),
}

pub struct EguiWgpuRenderer {
    pub egui_ctx: egui::Context,
    pub renderer: egui_wgpu::Renderer,
}

impl EguiWgpuRenderer {
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        let egui_ctx = egui::Context::default();

        // Configure Material Dark theme
        let mut visuals = egui::Visuals::dark();
        visuals.window_fill = egui::Color32::from_rgb(18, 19, 26);
        visuals.panel_fill = egui::Color32::from_rgb(18, 19, 26);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(34, 36, 50);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(43, 45, 62);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(60, 62, 85);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 191, 165);
        visuals.selection.stroke = egui::Stroke::NONE;
        egui_ctx.set_visuals(visuals);

        let renderer = egui_wgpu::Renderer::new(device, output_format, None, 1);

        Self {
            egui_ctx,
            renderer,
        }
    }

    pub fn draw_ui(
        &mut self,
        favorites: &[FavoriteMediaFile],
        _status_msg: &str,
        is_playing_in_app: bool,
        playing_title: Option<&str>,
        current_page: &mut usize,
        action: &mut EguiAppAction,
    ) {
        let ctx = &self.egui_ctx;

        if is_playing_in_app {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.heading(
                        egui::RichText::new("PLAYING IN-APP")
                            .color(egui::Color32::from_rgb(128, 222, 234))
                            .size(24.0)
                            .strong(),
                    );
                    ui.add_space(10.0);
                    if let Some(title) = playing_title {
                        ui.label(
                            egui::RichText::new(title)
                                .color(egui::Color32::WHITE)
                                .size(18.0),
                        );
                    }
                });
            });
            return;
        }

        let items_per_page = 5;
        let total_pages = if favorites.is_empty() {
            1
        } else {
            (favorites.len() + items_per_page - 1) / items_per_page
        };

        if *current_page >= total_pages {
            *current_page = 0;
        }

        // Fixed Bottom Action Bar with [ADD FAVOURITE] and [PLAY FILE]
        egui::TopBottomPanel::bottom("bottom_actions")
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(28, 29, 40))
                    .inner_margin(egui::Margin {
                        left: 16.0,
                        right: 16.0,
                        top: 10.0,
                        bottom: 32.0, // Clears bottom navigation bar cleanly
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let available_w = ui.available_width();
                    let btn_w = (available_w - 12.0) / 2.0;

                    let add_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("➕ ADD FAVOURITE")
                                .color(egui::Color32::WHITE)
                                .size(14.0)
                                .strong(),
                        )
                        .fill(egui::Color32::from_rgb(103, 80, 164))
                        .min_size(egui::vec2(btn_w, 48.0)),
                    );

                    if is_widget_tapped(&add_btn, ui) {
                        *action = EguiAppAction::AddFavorite;
                    }

                    ui.add_space(8.0);

                    let play_file_btn = ui.add(
                        egui::Button::new(
                            egui::RichText::new("📁 PLAY FILE")
                                .color(egui::Color32::WHITE)
                                .size(14.0)
                                .strong(),
                        )
                        .fill(egui::Color32::from_rgb(0, 191, 165))
                        .min_size(egui::vec2(btn_w, 48.0)),
                    );

                    if is_widget_tapped(&play_file_btn, ui) {
                        *action = EguiAppAction::PlayExternalFile;
                    }
                });
            });

        // Fixed Pagination Control Panel directly above the action buttons
        if total_pages > 1 {
            egui::TopBottomPanel::bottom("fixed_pagination_bar")
                .frame(
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(22, 23, 32))
                        .inner_margin(egui::Margin::symmetric(16.0, 8.0)),
                )
                .show(ctx, |ui| {
                    ui.horizontal_centered(|ui| {
                        let prev_btn = ui.add_enabled(
                            *current_page > 0,
                            egui::Button::new(
                                egui::RichText::new("< PREV")
                                    .color(egui::Color32::WHITE)
                                    .size(13.0)
                                    .strong(),
                            )
                            .min_size(egui::vec2(90.0, 38.0)),
                        );

                        if is_widget_tapped(&prev_btn, ui) && *current_page > 0 {
                            *current_page -= 1;
                            *action = EguiAppAction::None;
                            return;
                        }

                        ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                            ui.label(
                                egui::RichText::new(format!("Page {} of {}", *current_page + 1, total_pages))
                                    .color(egui::Color32::from_rgb(180, 182, 200))
                                    .size(13.0)
                                    .strong(),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let next_btn = ui.add_enabled(
                                *current_page + 1 < total_pages,
                                egui::Button::new(
                                    egui::RichText::new("NEXT >")
                                        .color(egui::Color32::WHITE)
                                        .size(13.0)
                                        .strong(),
                                )
                                .min_size(egui::vec2(90.0, 38.0)),
                            );

                            if is_widget_tapped(&next_btn, ui) && *current_page + 1 < total_pages {
                                *current_page += 1;
                                *action = EguiAppAction::None;
                            }
                        });
                    });
                });
        }

        // Main Content Area (Favorites List)
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(18, 19, 26))
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                ui.add_space(32.0); // Clear top status bar cutout

                // Favorites Header
                ui.heading(
                    egui::RichText::new(format!("FAVOURITES ({})", favorites.len()))
                        .color(egui::Color32::from_rgb(128, 222, 234))
                        .size(18.0)
                        .strong(),
                );

                ui.add_space(12.0);

                if favorites.is_empty() {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(34, 36, 50))
                        .rounding(12.0)
                        .inner_margin(egui::Margin::same(20.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new("No favourite media saved yet.")
                                    .color(egui::Color32::WHITE)
                                    .size(16.0),
                            );
                            ui.add_space(6.0);
                            ui.label(
                                egui::RichText::new("Tap [➕ ADD FAVOURITE] or [📁 PLAY FILE] below to start!")
                                    .color(egui::Color32::from_rgb(180, 182, 200))
                                    .size(13.0),
                            );
                        });
                } else {
                    let start_idx = *current_page * items_per_page;
                    let end_idx = (start_idx + items_per_page).min(favorites.len());

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (i, item) in favorites[start_idx..end_idx].iter().enumerate() {
                            let global_idx = start_idx + i;

                            ui.push_id(&item.uri, |ui| {
                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(34, 36, 50))
                                    .rounding(12.0)
                                    .inner_margin(egui::Margin::same(12.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            let is_audio = item.media_type.to_lowercase().contains("audio");
                                            let badge_text = if is_audio { "AUDIO" } else { "VIDEO" };

                                            let display_title = if item.display_name.chars().count() > 18 {
                                                let truncated: String = item.display_name.chars().take(15).collect();
                                                format!("{}...", truncated)
                                            } else {
                                                item.display_name.clone()
                                            };

                                            let pos_str = if item.last_position_ms > 0 {
                                                let secs = item.last_position_ms / 1000;
                                                let mins = secs / 60;
                                                let rem_secs = secs % 60;
                                                format!(" | Pos {:02}:{:02}", mins, rem_secs)
                                            } else {
                                                "".to_string()
                                            };

                                            // Left Item Button
                                            let item_label = format!("{}. {} [{}]\nSize: {}{}", global_idx + 1, display_title, badge_text, item.formatted_size(), pos_str);

                                            let item_btn = ui.add(
                                                egui::Button::new(
                                                    egui::RichText::new(item_label)
                                                        .color(egui::Color32::WHITE)
                                                        .size(14.0)
                                                        .strong(),
                                                )
                                                .fill(egui::Color32::TRANSPARENT)
                                                .frame(false),
                                            );

                                            if is_widget_tapped(&item_btn, ui) {
                                                *action = EguiAppAction::PlayFavorite(global_idx);
                                            }

                                            // Right Area: Action buttons (✏ Rename, 🗑 Delete)
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                // Delete Button (Dustbin)
                                                let del_btn = ui.add(
                                                    egui::Button::new(
                                                        egui::RichText::new("🗑")
                                                            .color(egui::Color32::WHITE)
                                                            .size(14.0),
                                                    )
                                                    .fill(egui::Color32::from_rgb(255, 82, 82))
                                                    .min_size(egui::vec2(44.0, 44.0)),
                                                );

                                                if is_widget_tapped(&del_btn, ui) {
                                                    *action = EguiAppAction::DeleteFavorite(global_idx);
                                                }

                                                ui.add_space(6.0);

                                                // Rename Button (Pencil)
                                                let rename_btn = ui.add(
                                                    egui::Button::new(
                                                        egui::RichText::new("✏")
                                                            .color(egui::Color32::WHITE)
                                                            .size(14.0),
                                                    )
                                                    .fill(egui::Color32::from_rgb(103, 80, 164))
                                                    .min_size(egui::vec2(44.0, 44.0)),
                                                );

                                                if is_widget_tapped(&rename_btn, ui) {
                                                    *action = EguiAppAction::RenameFavorite(global_idx);
                                                }
                                            });
                                        });
                                    });
                            });

                            ui.add_space(8.0);
                        }
                    });
                }
            });
    }
}

/// Helper function to detect single tap on any widget cleanly across desktop and touch
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
