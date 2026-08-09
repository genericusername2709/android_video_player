# `src/media_picker.rs` - JNI Bridge & Android Platform Integration

## Overview
`src/media_picker.rs` provides the JNI (Java Native Interface) bridge connecting Rust native code with the Java Android platform (`MainActivity.java` and Android framework APIs).

## Key Responsibilities

### 1. File Picker Integration
- **`open_android_file_picker(app, purpose)`:** Invokes Java `MainActivity.openFilePicker(requestCode)`. Launches `Intent.ACTION_OPEN_DOCUMENT` with MIME filters `video/*` and `audio/*`.
- **`query_last_selected_uri(app)`:** Invokes `MainActivity.consumeSelectedUri()` to retrieve the selected file's `content://` URI string and persistable read permissions.

### 2. Media Metadata Resolution (`resolve_favorite_media_file`)
- Queries Android `ContentResolver` using the selected `Uri`.
- Queries the `_display_name` and `_size` columns from `android.database.Cursor`.
- Queries MIME type via `ContentResolver.getType()` to classify media as `Audio` or `Video`.

### 3. In-App Video Playback Bridge
- **`play_media_in_app(app, uri, start_pos, title)`:** Invokes Java `MainActivity.playMediaInApp(uri, start_pos, title)`.
- Passes the URI, start timestamp in milliseconds, and display title to launch `VideoPlayerActivity`.
- **`query_playback_position(app)`:** Queries `MainActivity.getPlaybackPosition()` to fetch active video playback position.

### 4. Native Dialog Integration
- **`show_rename_dialog(app, index, current_name)`:** Invokes Java `MainActivity.showRenameDialog()`. Opens a native Android `AlertDialog` with an `EditText`.
- **`query_rename_result(app)`:** Consumes renamed title and target index returned from `MainActivity`.
- **`show_delete_dialog(app, index, current_name)`:** Invokes Java `MainActivity.showDeleteDialog()`. Opens a native Android confirmation `AlertDialog`.
- **`query_delete_result(app)`:** Consumes confirmed delete index from `MainActivity`.

## Interactions with Other Modules
- **`src/lib.rs`:** Invoked by `android_main` lifecycle loop to check intent results and launch dialogs/players.
- **`src/favourites.rs`:** Converts resolved metadata into `FavoriteMediaFile` structs for storage.
- **`MainActivity.java`:** Communicates across the JNI border with `MainActivity` Java class methods.
