#!/usr/bin/env bash
set -e

echo "=== 1. Building Rust Native CDYLIB ==="
cargo apk build

echo "=== 2. Compiling Java MainActivity ==="
rm -rf target/java_classes target/dex target/compiled_manifest
mkdir -p target/java_classes target/dex target/compiled_manifest

javac -g:none -source 8 -target 8 -cp "$ANDROID_HOME/platforms/android-34/android.jar" \
  java/com/example/android_video_player/*.java -d target/java_classes

echo "=== 3. Generating Dalvik Executable (classes.dex) ==="
"$ANDROID_HOME/build-tools/34.0.0/d8" --lib "$ANDROID_HOME/platforms/android-34/android.jar" \
  --output target/dex target/java_classes/com/example/android_video_player/*.class

echo "=== 4. Compiling Binary AndroidManifest.xml & Resources ==="
"$ANDROID_HOME/build-tools/34.0.0/aapt" package -f -M AndroidManifest.xml -S res \
  -I "$ANDROID_HOME/platforms/android-34/android.jar" -F target/manifest_out.apk
unzip -o target/manifest_out.apk -d target/compiled_manifest/

echo "=== 5. Packaging Native Library, AndroidManifest.xml, Uncompressed Resources & classes.dex into APK ==="
rm -f target/debug/apk/android_video_player_unaligned.apk
mkdir -p target/debug/apk/lib/arm64-v8a
cp target/aarch64-linux-android/debug/libandroid_video_player.so target/debug/apk/lib/arm64-v8a/

(cd target/debug/apk && zip -r0 android_video_player_unaligned.apk lib/ > /dev/null)
(cd target/compiled_manifest && zip -r0 ../debug/apk/android_video_player_unaligned.apk . > /dev/null)
zip -j target/debug/apk/android_video_player_unaligned.apk target/dex/classes.dex > /dev/null

echo "=== 6. Zipaligning APK ==="
"$ANDROID_HOME/build-tools/34.0.0/zipalign" -f 4 \
  target/debug/apk/android_video_player_unaligned.apk target/debug/apk/android_video_player.apk

echo "=== 7. Signing Final APK with Debug Key ==="
"$ANDROID_HOME/build-tools/34.0.0/apksigner" sign --ks ~/.android/debug.keystore --ks-pass pass:android \
  target/debug/apk/android_video_player.apk

echo "=== SUCCESS! APK built with package com.example.android_video_player at: target/debug/apk/android_video_player.apk ==="
