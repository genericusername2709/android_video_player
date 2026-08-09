#!/usr/bin/env bash
set -e

# 1. Build Rust + Java APK
./build_apk.sh

# 2. Install onto connected Android device/emulator
echo "=== Installing APK on Android device ==="
adb install -r target/debug/apk/android_video_player.apk

# 3. Start Main Activity
echo "=== Launching App ==="
adb shell am start -n com.example.android_video_player/com.example.android_video_player.MainActivity

# 4. Stream Logcat Logs
echo "=== Streaming Logcat Logs (Press Ctrl+C to exit log stream) ==="
adb logcat -s MainActivity:I Rust:I android_video_player:I
