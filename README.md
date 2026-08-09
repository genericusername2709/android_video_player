# Simple Android Video Player

An Android video and audio player built with Rust using `android-activity`, `winit`, and `wgpu`.

## Overview

This project is a native Android application written in Rust that provides a video and audio player interface with support for:
- Playing local video and audio files via Android's default system player
- Modern Material Design 3 dark theme interface with custom 2D canvas rendering
- Fast, persistent favorites list stored directly in app-private JSON storage
- File management capabilities (add and delete favorites)
- Automatic permission handling (`takePersistableUriPermission`) for robust file access across app restarts

## Architecture

- **`android-activity`**: Event loop glue between native C activity and Rust `android_main` thread.
- **`wgpu`**: Cross-platform WebGPU graphics rendering pipeline.
- **`MainActivity.java`**: Java wrapper class extending `NativeActivity` that overrides `onActivityResult` for persistent file picking permissions and handles thread dispatching via `runOnUiThread`.
- **`ui_renderer.rs` & `ui.rs`**: Pure Rust Material 2D rendering engine with custom text glyph drawing, chip badges, and layout positioning.
- **`media_picker.rs`**: Safe JNI bindings to Java helper methods with complete error handling.
- **`favourites.rs`**: Thread-safe JSON storage management for favorite media items.

## Building and Running

### Prerequisites
- Rust with `aarch66-linux-android` target installed (`rustup target add aarch64-linux-android`)
- Android NDK (r26b or newer)
- Android SDK with build-tools (34.0.0 or newer)
- `cargo-apk` (`cargo install cargo-apk`)

### One-Command Build & Run
To compile, package into APK, install on a connected Android device/emulator, and run:

```bash
./run.sh
```

### Build Only
To build the signed APK without installing:

```bash
./build_apk.sh
```

The output APK will be placed at `target/debug/apk/android_video_player.apk`.
