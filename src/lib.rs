mod favourites;
mod media_picker;
mod ui_renderer;

use android_activity::{AndroidApp, MainEvent, PollEvent};
use favourites::{FavoriteMediaFile, FavoritesManager};
use log::{error, info};
use media_picker::{
    PickerPurpose, clear_last_purpose, get_last_purpose, open_android_file_picker,
    play_media_in_app, query_delete_result, query_last_selected_uri, query_picker_finished,
    query_playback_position, query_rename_result, resolve_favorite_media_file, show_delete_dialog,
    show_rename_dialog,
};
use raw_window_handle::{AndroidDisplayHandle, HasRawWindowHandle, RawDisplayHandle};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::time::Duration;
use ui_renderer::{EguiAppAction, EguiWgpuRenderer};

static ACTIVE_PLAYING_FAV_INDEX: AtomicIsize = AtomicIsize::new(-1);

struct RenderState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    egui_renderer: EguiWgpuRenderer,
    max_texture_dimension: u32,
    native_window_size: (f32, f32),
    raw_input: egui::RawInput,
}

impl RenderState {
    fn new(app: &AndroidApp) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = unsafe {
            let window = app.native_window().unwrap();
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: RawDisplayHandle::Android(AndroidDisplayHandle::new()),
                    raw_window_handle: window.raw_window_handle().unwrap(),
                })
                .unwrap()
        };

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (w, h) = if let Some(wh) = app.native_window() {
            (wh.width() as u32, wh.height() as u32)
        } else {
            (1080, 1920)
        };

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Device Descriptor"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            },
            None,
        ))
        .unwrap();

        let max_texture_dimension = device.limits().max_texture_dimension_2d;
        info!("GPU Max texture dimension: {}", max_texture_dimension);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let clamped_w = w.clamp(1, max_texture_dimension);
        let clamped_h = h.clamp(1, max_texture_dimension);

        info!(
            "Native window: ({}, {}), Configured surface: ({}, {})",
            w, h, clamped_w, clamped_h
        );

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: clamped_w,
            height: clamped_h,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let egui_renderer = EguiWgpuRenderer::new(&device, surface_format);

        Self {
            device,
            queue,
            surface,
            config,
            egui_renderer,
            max_texture_dimension,
            native_window_size: (w as f32, h as f32),
            raw_input: egui::RawInput::default(),
        }
    }

    fn resize(&mut self, app: &AndroidApp) {
        let (w, h) = if let Some(wh) = app.native_window() {
            (wh.width() as u32, wh.height() as u32)
        } else {
            (1080, 1920)
        };

        self.native_window_size = (w as f32, h as f32);
        let clamped_w = w.clamp(1, self.max_texture_dimension);
        let clamped_h = h.clamp(1, self.max_texture_dimension);

        if clamped_w > 0 && clamped_h > 0 {
            self.config.width = clamped_w;
            self.config.height = clamped_h;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn get_pixels_per_point(&self) -> f32 {
        (self.config.width as f32 / 380.0).clamp(2.0, 3.5)
    }

    fn map_touch_pos(&self, touch_x: f32, touch_y: f32) -> egui::Pos2 {
        let (native_w, native_h) = self.native_window_size;
        let scale_x = if native_w > 0.0 {
            self.config.width as f32 / native_w
        } else {
            1.0
        };
        let scale_y = if native_h > 0.0 {
            self.config.height as f32 / native_h
        } else {
            1.0
        };

        let surface_x = touch_x * scale_x;
        let surface_y = touch_y * scale_y;

        let ppp = self.get_pixels_per_point();
        egui::pos2(surface_x / ppp, surface_y / ppp)
    }

    fn render(
        &mut self,
        favorites: &[FavoriteMediaFile],
        status: &str,
        is_playing_in_app: bool,
        playing_title: Option<&str>,
        current_page: &mut usize,
    ) -> EguiAppAction {
        if self.config.width == 0 || self.config.height == 0 {
            return EguiAppAction::None;
        }

        let pixels_per_point = self.get_pixels_per_point();
        let logical_width = self.config.width as f32 / pixels_per_point;
        let logical_height = self.config.height as f32 / pixels_per_point;

        self.raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(logical_width, logical_height),
        ));

        let mut action = EguiAppAction::None;
        let egui_ctx = self.egui_renderer.egui_ctx.clone();
        egui_ctx.set_pixels_per_point(pixels_per_point);

        let full_output = egui_ctx.run(self.raw_input.take(), |_ctx| {
            self.egui_renderer.draw_ui(
                favorites,
                status,
                is_playing_in_app,
                playing_title,
                current_page,
                &mut action,
            );
        });

        let clipped_primitives =
            egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        let output = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return action;
            }
            Err(e) => {
                error!("Surface error: {:?}", e);
                return action;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point,
        };

        for (id, delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .renderer
                .update_texture(&self.device, &self.queue, *id, delta);
        }

        self.egui_renderer.renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.07,
                            g: 0.07,
                            b: 0.10,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.egui_renderer.renderer.render(
                &mut render_pass,
                &clipped_primitives,
                &screen_descriptor,
            );
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        action
    }
}

#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    info!("GPU egui Rust Media Player App Started!");

    let favorites_mgr = FavoritesManager::new();
    favorites_mgr.load_from_storage(&app);

    let mut render_state: Option<RenderState> = None;
    let mut current_page: usize = 0;
    let mut status_msg = "Ready.".to_string();
    let mut needs_redraw = true;
    let mut is_playing_video = false;

    loop {
        let timeout = Duration::from_millis(100);
        let mut should_exit = false;

        app.poll_events(Some(timeout), |event| match event {
            PollEvent::Main(MainEvent::InitWindow { .. }) => {
                info!("Native window initialized - setting up GPU render state");
                let state = RenderState::new(&app);
                render_state = Some(state);
                needs_redraw = true;
            }
            PollEvent::Main(MainEvent::TerminateWindow { .. }) => {
                info!("Native window destroyed");
                render_state = None;
            }
            PollEvent::Main(MainEvent::WindowResized { .. }) => {
                if let Some(ref mut state) = render_state {
                    state.resize(&app);
                    needs_redraw = true;
                }
            }
            PollEvent::Main(MainEvent::RedrawNeeded { .. }) => {
                needs_redraw = true;
            }
            PollEvent::Main(MainEvent::Pause { .. }) => {
                info!("Activity paused");
                sync_active_playback_position(&app, &favorites_mgr);
            }
            PollEvent::Main(MainEvent::Resume { .. }) => {
                info!(
                    "Activity resumed - checking for selected file intent data or rename result..."
                );
                needs_redraw = true;
                if check_picker_result(&app, &favorites_mgr, &mut status_msg, &mut is_playing_video)
                {
                    needs_redraw = true;
                }
                if check_rename_updates(&app, &favorites_mgr, &mut status_msg) {
                    needs_redraw = true;
                }
                if is_playing_video {
                    info!("In-app video player overlay closed - syncing position on resume");
                    sync_active_playback_position(&app, &favorites_mgr);
                    ACTIVE_PLAYING_FAV_INDEX.store(-1, Ordering::SeqCst);
                    is_playing_video = false;
                    status_msg = "Ready.".to_string();
                    needs_redraw = true;
                }
            }
            PollEvent::Main(MainEvent::Destroy) => {
                info!("Native Activity being destroyed");
                sync_active_playback_position(&app, &favorites_mgr);
                ACTIVE_PLAYING_FAV_INDEX.store(-1, Ordering::SeqCst);
                is_playing_video = false;
                should_exit = true;
            }
            _ => {}
        });

        if should_exit {
            break;
        }

        // Process touch inputs and feed into egui natively
        if let Ok(mut iter) = app.input_events_iter() {
            while iter.next(|event| {
                if let android_activity::input::InputEvent::MotionEvent(motion_event) = event {
                    if let Some(pointer) = motion_event.pointers().next() {
                        if let Some(ref mut state) = render_state {
                            let pos = state.map_touch_pos(pointer.x(), pointer.y());
                            let touch_id = egui::TouchId(pointer.pointer_id() as u64);

                            match motion_event.action() {
                                android_activity::input::MotionAction::Down => {
                                    state.raw_input.events.push(egui::Event::PointerMoved(pos));
                                    state.raw_input.events.push(egui::Event::PointerButton {
                                        pos,
                                        button: egui::PointerButton::Primary,
                                        pressed: true,
                                        modifiers: egui::Modifiers::default(),
                                    });
                                    state.raw_input.events.push(egui::Event::Touch {
                                        device_id: egui::TouchDeviceId(0),
                                        id: touch_id,
                                        phase: egui::TouchPhase::Start,
                                        pos,
                                        force: None,
                                    });
                                    needs_redraw = true;
                                }
                                android_activity::input::MotionAction::Move => {
                                    state.raw_input.events.push(egui::Event::PointerMoved(pos));
                                    state.raw_input.events.push(egui::Event::Touch {
                                        device_id: egui::TouchDeviceId(0),
                                        id: touch_id,
                                        phase: egui::TouchPhase::Move,
                                        pos,
                                        force: None,
                                    });
                                    needs_redraw = true;
                                }
                                android_activity::input::MotionAction::Up => {
                                    state.raw_input.events.push(egui::Event::PointerMoved(pos));
                                    state.raw_input.events.push(egui::Event::PointerButton {
                                        pos,
                                        button: egui::PointerButton::Primary,
                                        pressed: false,
                                        modifiers: egui::Modifiers::default(),
                                    });
                                    state.raw_input.events.push(egui::Event::Touch {
                                        device_id: egui::TouchDeviceId(0),
                                        id: touch_id,
                                        phase: egui::TouchPhase::End,
                                        pos,
                                        force: None,
                                    });
                                    needs_redraw = true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                android_activity::InputStatus::Handled
            }) {}
        }

        if check_picker_result(&app, &favorites_mgr, &mut status_msg, &mut is_playing_video) {
            needs_redraw = true;
        }

        if check_rename_updates(&app, &favorites_mgr, &mut status_msg) {
            needs_redraw = true;
        }

        if check_delete_updates(&app, &favorites_mgr, &mut status_msg) {
            needs_redraw = true;
        }

        if needs_redraw || is_playing_video {
            if let Some(ref mut state) = render_state {
                let favorites_list = favorites_mgr.get_all();

                let playing_title = if is_playing_video {
                    let active_idx = ACTIVE_PLAYING_FAV_INDEX.load(Ordering::SeqCst);
                    if active_idx >= 0 {
                        favorites_list
                            .get(active_idx as usize)
                            .map(|f| f.display_name.as_str())
                    } else {
                        Some("Playing Video")
                    }
                } else {
                    None
                };

                let action = state.render(
                    &favorites_list,
                    &status_msg,
                    is_playing_video,
                    playing_title,
                    &mut current_page,
                );

                match action {
                    EguiAppAction::AddFavorite => {
                        info!("User tapped [ADD FAVOURITE]");
                        status_msg = "Opening file picker...".to_string();
                        if let Err(e) =
                            open_android_file_picker(&app, PickerPurpose::AddToFavorites)
                        {
                            error!("Failed to open file picker: {}", e);
                            status_msg = format!("Error: {}", e);
                        }
                    }
                    EguiAppAction::PlayExternalFile => {
                        info!("User tapped [PLAY FILE]");
                        status_msg = "Opening file picker for playback...".to_string();
                        if let Err(e) = open_android_file_picker(&app, PickerPurpose::PlayFile) {
                            error!("Failed to open file picker: {}", e);
                            status_msg = format!("Error: {}", e);
                        }
                    }
                    EguiAppAction::PlayFavorite(idx) => {
                        let favs = favorites_mgr.get_all();
                        if let Some(fav) = favs.get(idx) {
                            info!(
                                "User tapped Play on favourite #{}: {}",
                                idx + 1,
                                fav.display_name
                            );
                            status_msg = format!("Playing '{}'...", fav.display_name);
                            ACTIVE_PLAYING_FAV_INDEX.store(idx as isize, Ordering::SeqCst);
                            is_playing_video = true;
                            if let Err(e) = play_media_in_app(
                                &app,
                                &fav.uri,
                                fav.last_position_ms,
                                &fav.display_name,
                            ) {
                                error!("Failed to play favourite: {}", e);
                                status_msg = format!("Error playing: {}", e);
                                is_playing_video = false;
                            }
                        }
                    }
                    EguiAppAction::RenameFavorite(idx) => {
                        let favs = favorites_mgr.get_all();
                        if let Some(fav) = favs.get(idx) {
                            info!(
                                "User tapped Rename on favourite #{}: {}",
                                idx + 1,
                                fav.display_name
                            );
                            status_msg = format!("Renaming '{}'...", fav.display_name);
                            if let Err(e) = show_rename_dialog(&app, idx, &fav.display_name) {
                                error!("Failed to show rename dialog: {}", e);
                                status_msg = format!("Error: {}", e);
                            }
                        }
                    }
                    EguiAppAction::DeleteFavorite(idx) => {
                        let favs = favorites_mgr.get_all();
                        if let Some(fav) = favs.get(idx) {
                            info!(
                                "User tapped Delete on favourite #{}: {}",
                                idx + 1,
                                fav.display_name
                            );
                            status_msg = format!("Deleting '{}'...", fav.display_name);
                            if let Err(e) = show_delete_dialog(&app, idx, &fav.display_name) {
                                error!("Failed to show delete dialog: {}", e);
                                status_msg = format!("Error: {}", e);
                            }
                        }
                    }
                    EguiAppAction::None => {}
                }

                if action != EguiAppAction::None {
                    state.raw_input.events.clear();
                }

                needs_redraw = false;
            }
        }
    }
}

fn sync_active_playback_position(app: &AndroidApp, fav_mgr: &FavoritesManager) {
    let active_idx = ACTIVE_PLAYING_FAV_INDEX.load(Ordering::SeqCst);
    if active_idx >= 0 {
        let pos = query_playback_position(app);
        fav_mgr.update_position(active_idx as usize, pos, app);
    }
}

fn check_picker_result(
    app: &AndroidApp,
    fav_mgr: &FavoritesManager,
    status_msg: &mut String,
    is_playing_video: &mut bool,
) -> bool {
    let mut updated = false;
    if let Some(uri) = query_last_selected_uri(app) {
        if let Some(purpose) = get_last_purpose() {
            match purpose {
                PickerPurpose::PlayFile => {
                    info!("Playing selected file URI: {}", uri);
                    *status_msg = "Playing selected file...".to_string();

                    // Automatically add played file to favourites list and set active index
                    let mut start_pos = 0;
                    let mut title = "Selected Video".to_string();
                    if let Ok(fav_file) = resolve_favorite_media_file(app, &uri) {
                        title = fav_file.display_name.clone();
                        let _ = fav_mgr.add_favorite(fav_file, app);
                        let all_favs = fav_mgr.get_all();
                        if let Some(idx) = all_favs.iter().position(|f| f.uri == uri) {
                            ACTIVE_PLAYING_FAV_INDEX.store(idx as isize, Ordering::SeqCst);
                            start_pos = all_favs[idx].last_position_ms;
                        } else {
                            ACTIVE_PLAYING_FAV_INDEX.store(-1, Ordering::SeqCst);
                        }
                    } else {
                        ACTIVE_PLAYING_FAV_INDEX.store(-1, Ordering::SeqCst);
                    }

                    *is_playing_video = true;
                    if let Err(e) = play_media_in_app(app, &uri, start_pos, &title) {
                        error!("Failed to play selected file in-app: {}", e);
                        *status_msg = format!("Error playing: {}", e);
                        *is_playing_video = false;
                    }
                    updated = true;
                }
                PickerPurpose::AddToFavorites => match resolve_favorite_media_file(app, &uri) {
                    Ok(fav_file) => {
                        let name = fav_file.display_name.clone();
                        if fav_mgr.add_favorite(fav_file, app) {
                            *status_msg = format!("Saved '{}' to favourites!", name);
                        } else {
                            *status_msg = format!("'{}' already in favourites", name);
                        }
                        updated = true;
                    }
                    Err(e) => {
                        error!("Failed to resolve favourite file info: {}", e);
                        *status_msg = format!("Error adding favourite: {}", e);
                    }
                },
            }
            clear_last_purpose();
            return updated;
        } else {
            clear_last_purpose();
        }
    } else if query_picker_finished(app) {
        info!("File picker activity finished or was cancelled without selection");
        clear_last_purpose();
    }
    updated
}

fn check_rename_updates(
    app: &AndroidApp,
    fav_mgr: &FavoritesManager,
    status_msg: &mut String,
) -> bool {
    if let Some((index, new_title)) = query_rename_result(app) {
        if fav_mgr.rename_favorite(index, &new_title, app) {
            *status_msg = format!("Renamed favourite #{} to '{}'", index + 1, new_title);
            return true;
        }
    }
    false
}

fn check_delete_updates(
    app: &AndroidApp,
    fav_mgr: &FavoritesManager,
    status_msg: &mut String,
) -> bool {
    if let Some(index) = query_delete_result(app) {
        let favs = fav_mgr.get_all();
        if let Some(fav) = favs.get(index) {
            let name = fav.display_name.clone();
            if fav_mgr.remove_favorite(index, app) {
                *status_msg = format!("Removed '{}' from favourites", name);
                return true;
            }
        }
    }
    false
}
