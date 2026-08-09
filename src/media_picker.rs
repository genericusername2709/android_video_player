use crate::favourites::FavoriteMediaFile;
use android_activity::AndroidApp;
use jni::JavaVM;
use jni::objects::{JClass, JObject, JString, JValue};
use log::info;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerPurpose {
    PlayFile,
    AddToFavorites,
}

static LAST_PURPOSE: Mutex<Option<PickerPurpose>> = Mutex::new(None);

pub fn set_last_purpose(purpose: PickerPurpose) {
    let mut p = LAST_PURPOSE.lock().unwrap();
    *p = Some(purpose);
}

pub fn get_last_purpose() -> Option<PickerPurpose> {
    let p = LAST_PURPOSE.lock().unwrap();
    *p
}

pub fn clear_last_purpose() {
    let mut p = LAST_PURPOSE.lock().unwrap();
    *p = None;
}

/// Helper function to safely extract Java Activity jobject from AndroidApp or ndk_context
fn get_java_activity<'a>(app: &AndroidApp) -> Result<JObject<'a>, String> {
    let ctx = ndk_context::android_context();
    let jobj_ptr = ctx.context();
    if !jobj_ptr.is_null() {
        return Ok(unsafe { JObject::from_raw(jobj_ptr.cast()) });
    }

    let activity_ptr = app.activity_as_ptr();
    if activity_ptr.is_null() {
        return Err("ANativeActivity pointer is null".to_string());
    }

    let clazz_ptr = unsafe {
        let native_act = std::ptr::read_unaligned(activity_ptr as *const ndk_sys::ANativeActivity);
        native_act.clazz
    };

    if clazz_ptr.is_null() {
        return Err("ANativeActivity.clazz jobject pointer is null".to_string());
    }

    Ok(unsafe { JObject::from_raw(clazz_ptr.cast()) })
}

/// Loads the custom MainActivity JClass using the App's ClassLoader
fn get_main_activity_class<'a>(
    env: &mut jni::JNIEnv<'a>,
    app: &AndroidApp,
) -> Result<JClass<'a>, String> {
    let activity = get_java_activity(app)?;

    let class_loader = env
        .call_method(
            &activity,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .map_err(|e| format!("Failed to get ClassLoader: {:?}", e))?
        .l()
        .map_err(|e| format!("Invalid ClassLoader object: {:?}", e))?;

    let class_name = env
        .new_string("com.example.android_video_player.MainActivity")
        .map_err(|e| format!("Failed to create class name string: {:?}", e))?;

    let cls_obj = env
        .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name)],
        )
        .map_err(|e| format!("Failed to loadClass MainActivity: {:?}", e))?
        .l()
        .map_err(|e| format!("Invalid MainActivity class object: {:?}", e))?;

    Ok(JClass::from(cls_obj))
}

/// Triggers Android's native file selection dialog via MainActivity.openFilePicker
pub fn open_android_file_picker(app: &AndroidApp, purpose: PickerPurpose) -> Result<(), String> {
    if query_picker_finished(app) {
        clear_last_purpose();
    }

    // PREVENT DUPLICATE INTENT SPAM: Ignore duplicate tap events if picker is already active
    if get_last_purpose().is_some() {
        info!("File picker is already active, ignoring duplicate tap");
        return Ok(());
    }

    info!(
        "Opening Android native file selection popup for purpose: {:?}",
        purpose
    );
    set_last_purpose(purpose);

    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        clear_last_purpose();
        return Err("Java VM pointer is null".to_string());
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }.map_err(|e| {
        clear_last_purpose();
        format!("Failed to attach JavaVM: {:?}", e)
    })?;
    let mut env = vm.attach_current_thread().map_err(|e| {
        clear_last_purpose();
        format!("Failed to attach thread: {:?}", e)
    })?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let main_cls = match get_main_activity_class(&mut env, app) {
        Ok(cls) => cls,
        Err(e) => {
            clear_last_purpose();
            return Err(e);
        }
    };

    let request_code = match purpose {
        PickerPurpose::PlayFile => 101,
        PickerPurpose::AddToFavorites => 102,
    };

    let res = env.call_static_method(
        &main_cls,
        "openFilePicker",
        "(I)V",
        &[JValue::Int(request_code)],
    );

    if let Err(e) = res {
        clear_last_purpose();
        return Err(format!("Failed to call openFilePicker: {:?}", e));
    }

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    info!(
        "openFilePicker method invoked successfully for request code {}",
        request_code
    );
    Ok(())
}

/// Launch Android default system player for the given media URI via MainActivity.playMedia
#[allow(dead_code)]
pub fn play_media_intent(app: &AndroidApp, uri_str: &str) -> Result<(), String> {
    info!("Launching Android system media player for URI: {}", uri_str);
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return Err("Java VM pointer is null".to_string());
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }
        .map_err(|e| format!("Failed to attach JavaVM: {:?}", e))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Failed to attach thread: {:?}", e))?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let main_cls = get_main_activity_class(&mut env, app)?;
    let j_uri_str = env
        .new_string(uri_str)
        .map_err(|e| format!("Failed to create URI string: {:?}", e))?;

    let res = env.call_static_method(
        &main_cls,
        "playMedia",
        "(Ljava/lang/String;)V",
        &[JValue::Object(&j_uri_str)],
    );

    if let Err(e) = res {
        return Err(format!("Failed to call playMedia: {:?}", e));
    }

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    info!("playMedia method invoked successfully for URI: {}", uri_str);
    Ok(())
}

/// Play video directly inside the app using VideoView overlay with title
pub fn play_media_in_app(
    app: &AndroidApp,
    uri_str: &str,
    start_position_ms: u64,
    title: &str,
) -> Result<(), String> {
    info!(
        "Playing media in-app for URI: {} at pos: {} ms, title: {}",
        uri_str, start_position_ms, title
    );
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return Err("Java VM pointer is null".to_string());
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }
        .map_err(|e| format!("Failed to attach JavaVM: {:?}", e))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Failed to attach thread: {:?}", e))?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let main_cls = get_main_activity_class(&mut env, app)?;
    let j_uri_str = env
        .new_string(uri_str)
        .map_err(|e| format!("Failed to create URI string: {:?}", e))?;
    let j_title_str = env
        .new_string(title)
        .map_err(|e| format!("Failed to create Title string: {:?}", e))?;

    let res = env.call_static_method(
        &main_cls,
        "playMediaInApp",
        "(Ljava/lang/String;JLjava/lang/String;)V",
        &[
            JValue::Object(&j_uri_str),
            JValue::Long(start_position_ms as i64),
            JValue::Object(&j_title_str),
        ],
    );

    if let Err(e) = res {
        return Err(format!("Failed to call playMediaInApp: {:?}", e));
    }

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    info!("playMediaInApp invoked successfully");
    Ok(())
}

/// Query current video playback position in milliseconds
pub fn query_playback_position(app: &AndroidApp) -> u64 {
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return 0;
    }
    let vm = match unsafe { JavaVM::from_raw(vm_ptr.cast()) } {
        Ok(v) => v,
        Err(_) => return 0,
    };
    let mut env = match vm.attach_current_thread() {
        Ok(e) => e,
        Err(_) => return 0,
    };

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let res: Result<u64, jni::errors::Error> = env.with_local_frame(16, |env| {
        if let Ok(main_cls) = get_main_activity_class(env, app) {
            if let Ok(val) = env.call_static_method(&main_cls, "getPlaybackPosition", "()J", &[]) {
                if let Ok(pos) = val.j() {
                    if pos > 0 {
                        return Ok(pos as u64);
                    }
                }
            }
        }
        Ok(0)
    });
    res.unwrap_or(0)
}

/// Close and stop in-app video player overlay
#[allow(dead_code)]
pub fn close_video_player(app: &AndroidApp) -> Result<(), String> {
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return Err("Java VM pointer is null".to_string());
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }
        .map_err(|e| format!("Failed to attach JavaVM: {:?}", e))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Failed to attach thread: {:?}", e))?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let main_cls = get_main_activity_class(&mut env, app)?;
    let _ = env.call_static_method(&main_cls, "closeVideoPlayer", "()V", &[]);
    Ok(())
}

/// Displays an AlertDialog to rename a favorite item
pub fn show_rename_dialog(
    app: &AndroidApp,
    index: usize,
    current_name: &str,
) -> Result<(), String> {
    info!(
        "Showing rename dialog for favorite index {}: {}",
        index, current_name
    );
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return Err("Java VM pointer is null".to_string());
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }
        .map_err(|e| format!("Failed to attach JavaVM: {:?}", e))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Failed to attach thread: {:?}", e))?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let main_cls = get_main_activity_class(&mut env, app)?;
    let j_name_str = env
        .new_string(current_name)
        .map_err(|e| format!("Failed to create String: {:?}", e))?;

    let res = env.call_static_method(
        &main_cls,
        "showRenameDialog",
        "(ILjava/lang/String;)V",
        &[JValue::Int(index as i32), JValue::Object(&j_name_str)],
    );

    if let Err(e) = res {
        return Err(format!("Failed to call showRenameDialog: {:?}", e));
    }

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    Ok(())
}

/// Query MainActivity for any updated rename result
pub fn query_rename_result(app: &AndroidApp) -> Option<(usize, String)> {
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return None;
    }
    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let res: Result<Option<(usize, String)>, jni::errors::Error> =
        env.with_local_frame(16, |env| {
            if let Ok(main_cls) = get_main_activity_class(env, app) {
                if let Ok(val) = env.call_static_method(
                    &main_cls,
                    "consumeRenamedTitle",
                    "()Ljava/lang/String;",
                    &[],
                ) {
                    if let Ok(obj) = val.l() {
                        if !obj.is_null() {
                            let j_str: JString = obj.into();
                            if let Ok(rust_str) = env.get_string(&j_str) {
                                let title: String = rust_str.into();
                                if let Ok(idx_val) = env.call_static_method(
                                    &main_cls,
                                    "getRenamedIndexAndClear",
                                    "()I",
                                    &[],
                                ) {
                                    if let Ok(idx) = idx_val.i() {
                                        if idx >= 0 {
                                            return Ok(Some((idx as usize, title)));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(None)
        });
    res.unwrap_or(None)
}

/// Displays an AlertDialog to confirm deleting a favorite item
pub fn show_delete_dialog(
    app: &AndroidApp,
    index: usize,
    current_name: &str,
) -> Result<(), String> {
    info!(
        "Showing delete dialog for favorite index {}: {}",
        index, current_name
    );
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return Err("Java VM pointer is null".to_string());
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }
        .map_err(|e| format!("Failed to attach JavaVM: {:?}", e))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Failed to attach thread: {:?}", e))?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let main_cls = get_main_activity_class(&mut env, app)?;
    let j_name_str = env
        .new_string(current_name)
        .map_err(|e| format!("Failed to create String: {:?}", e))?;

    let res = env.call_static_method(
        &main_cls,
        "showDeleteDialog",
        "(ILjava/lang/String;)V",
        &[JValue::Int(index as i32), JValue::Object(&j_name_str)],
    );

    if let Err(e) = res {
        return Err(format!("Failed to call showDeleteDialog: {:?}", e));
    }

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    Ok(())
}

/// Query MainActivity for confirmed delete index
pub fn query_delete_result(app: &AndroidApp) -> Option<usize> {
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return None;
    }
    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let res: Result<Option<usize>, jni::errors::Error> = env.with_local_frame(16, |env| {
        if let Ok(main_cls) = get_main_activity_class(env, app) {
            if let Ok(val) = env.call_static_method(&main_cls, "consumeDeletedIndex", "()I", &[]) {
                if let Ok(idx) = val.i() {
                    if idx >= 0 {
                        return Ok(Some(idx as usize));
                    }
                }
            }
        }
        Ok(None)
    });
    res.unwrap_or(None)
}

/// Query Activity for last selected URI intent data (safely framed in local JNI frame)
pub fn query_last_selected_uri(app: &AndroidApp) -> Option<String> {
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return None;
    }
    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let res: Result<Option<String>, jni::errors::Error> = env.with_local_frame(16, |env| {
        if let Ok(main_cls) = get_main_activity_class(env, app) {
            if let Ok(val) =
                env.call_static_method(&main_cls, "consumeSelectedUri", "()Ljava/lang/String;", &[])
            {
                if let Ok(obj) = val.l() {
                    if !obj.is_null() {
                        let j_str: JString = obj.into();
                        if let Ok(rust_str) = env.get_string(&j_str) {
                            let s: String = rust_str.into();
                            if !s.is_empty() {
                                info!("Retrieved URI from MainActivity consumeSelectedUri: {}", s);
                                return Ok(Some(s));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    });
    res.unwrap_or(None)
}

/// Query Activity if file picker has finished (or was cancelled)
pub fn query_picker_finished(app: &AndroidApp) -> bool {
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return false;
    }
    let vm = match unsafe { JavaVM::from_raw(vm_ptr.cast()) } {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut env = match vm.attach_current_thread() {
        Ok(e) => e,
        Err(_) => return false,
    };

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let res: Result<bool, jni::errors::Error> = env.with_local_frame(16, |env| {
        if let Ok(main_cls) = get_main_activity_class(env, app) {
            if let Ok(val) = env.call_static_method(&main_cls, "consumePickerFinished", "()Z", &[])
            {
                if let Ok(b) = val.z() {
                    return Ok(b);
                }
            }
        }
        Ok(false)
    });
    res.unwrap_or(false)
}

/// Resolves a content:// or file:// URI to a FavoriteMediaFile struct
pub fn resolve_favorite_media_file(
    app: &AndroidApp,
    uri_str: &str,
) -> Result<FavoriteMediaFile, String> {
    let vm_ptr = app.vm_as_ptr();
    if vm_ptr.is_null() {
        return Err("Java VM pointer is null".to_string());
    }

    let vm = unsafe { JavaVM::from_raw(vm_ptr.cast()) }
        .map_err(|e| format!("Failed to attach JavaVM: {:?}", e))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|e| format!("Failed to attach thread: {:?}", e))?;

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }

    let activity = get_java_activity(app)?;

    let raw_name = uri_str.split('/').last().unwrap_or(uri_str);
    let mut display_name = urlencoding_decode(raw_name);
    let mut size_bytes: u64 = 0;
    let mut media_type_str = if uri_str.to_lowercase().contains("audio")
        || uri_str.ends_with(".mp3")
        || uri_str.ends_with(".wav")
    {
        "Audio".to_string()
    } else {
        "Video".to_string()
    };

    if let Ok(uri_cls) = env.find_class("android/net/Uri") {
        if let Ok(j_uri_str) = env.new_string(uri_str) {
            if let Ok(parse_res) = env.call_static_method(
                &uri_cls,
                "parse",
                "(Ljava/lang/String;)Landroid/net/Uri;",
                &[JValue::Object(&j_uri_str)],
            ) {
                if let Ok(uri_obj) = parse_res.l() {
                    if !uri_obj.is_null() {
                        if let Ok(resolver_val) = env.call_method(
                            &activity,
                            "getContentResolver",
                            "()Landroid/content/ContentResolver;",
                            &[],
                        ) {
                            if let Ok(resolver) = resolver_val.l() {
                                if let Ok(type_val) = env.call_method(
                                    &resolver,
                                    "getType",
                                    "(Landroid/net/Uri;)Ljava/lang/String;",
                                    &[JValue::Object(&uri_obj)],
                                ) {
                                    if let Ok(type_obj) = type_val.l() {
                                        if !type_obj.is_null() {
                                            let j_str: JString = type_obj.into();
                                            if let Ok(rust_str) = env.get_string(&j_str) {
                                                let s: String = rust_str.into();
                                                if s.starts_with("audio/") {
                                                    media_type_str = "Audio".to_string();
                                                } else if s.starts_with("video/") {
                                                    media_type_str = "Video".to_string();
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Ok(cursor_val) = env.call_method(
                                    &resolver,
                                    "query",
                                    "(Landroid/net/Uri;[Ljava/lang/String;Ljava/lang/String;[Ljava/lang/String;Ljava/lang/String;)Landroid/database/Cursor;",
                                    &[
                                        JValue::Object(&uri_obj),
                                        JValue::Object(&JObject::null()),
                                        JValue::Object(&JObject::null()),
                                        JValue::Object(&JObject::null()),
                                        JValue::Object(&JObject::null()),
                                    ],
                                ) {
                                    if let Ok(cursor) = cursor_val.l() {
                                        if !cursor.is_null() {
                                            if let Ok(moved) = env.call_method(&cursor, "moveToFirst", "()Z", &[]) {
                                                if moved.z().unwrap_or(false) {
                                                    if let Ok(col_name) = env.new_string("_display_name") {
                                                        if let Ok(idx_val) = env.call_method(
                                                            &cursor,
                                                            "getColumnIndex",
                                                            "(Ljava/lang/String;)I",
                                                            &[JValue::Object(&col_name)],
                                                        ) {
                                                            let idx = idx_val.i().unwrap_or(-1);
                                                            if idx >= 0 {
                                                                if let Ok(name_val) = env.call_method(
                                                                    &cursor,
                                                                    "getString",
                                                                    "(I)Ljava/lang/String;",
                                                                    &[JValue::Int(idx)],
                                                                ) {
                                                                    if let Ok(name_obj) = name_val.l() {
                                                                        if !name_obj.is_null() {
                                                                            let j_s: JString = name_obj.into();
                                                                            if let Ok(s) = env.get_string(&j_s) {
                                                                                display_name = s.into();
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }

                                                    if let Ok(col_size) = env.new_string("_size") {
                                                        if let Ok(size_idx_val) = env.call_method(
                                                            &cursor,
                                                            "getColumnIndex",
                                                            "(Ljava/lang/String;)I",
                                                            &[JValue::Object(&col_size)],
                                                        ) {
                                                            let idx = size_idx_val.i().unwrap_or(-1);
                                                            if idx >= 0 {
                                                                if let Ok(s_val) = env.call_method(
                                                                    &cursor,
                                                                    "getLong",
                                                                    "(I)J",
                                                                    &[JValue::Int(idx)],
                                                                ) {
                                                                    if let Ok(sz) = s_val.j() {
                                                                        if sz > 0 {
                                                                            size_bytes = sz as u64;
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(FavoriteMediaFile {
        uri: uri_str.to_string(),
        display_name,
        size_bytes,
        media_type: media_type_str,
        added_date: crate::favourites::current_formatted_date(),
        last_position_ms: 0,
    })
}

fn urlencoding_decode(s: &str) -> String {
    s.replace("%20", " ")
        .replace("%3A", ":")
        .replace("%2F", "/")
        .replace("%2C", ",")
}
