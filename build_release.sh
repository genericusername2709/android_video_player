#!/usr/bin/env bash
set -e

NDK_PATH="/home/mustafa/android-ndk/android-ndk-r26b"
if [ -n "$ANDROID_NDK" ]; then
    NDK_PATH="$ANDROID_NDK"
fi

NDK_CLANG="$NDK_PATH/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android23-clang"
NDK_AR="$NDK_PATH/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar"

echo "=== 1. Building Rust Native CDYLIB (Production Release) ==="
CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_CLANG" \
CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$NDK_AR" \
cargo build --target aarch64-linux-android --release

mkdir -p target/release/apk/lib/arm64-v8a
cp target/aarch64-linux-android/release/libandroid_video_player.so target/release/apk/lib/arm64-v8a/

KEYSTORE_PATH="release.keystore"
KEY_ALIAS="release-key"

if [ ! -f "$KEYSTORE_PATH" ]; then
    echo "=== Release keystore not found at $KEYSTORE_PATH ==="
    echo "Generating new production keystore '$KEYSTORE_PATH'..."
    keytool -genkey -v -keystore "$KEYSTORE_PATH" \
        -alias "$KEY_ALIAS" \
        -keyalg RSA -keysize 2048 \
        -validity 10000 \
        -storepass "androidrelease" \
        -keypass "androidrelease" \
        -dname "CN=Android Player, OU=Dev, O=App, L=City, ST=State, C=US"
fi

echo "=== 2. Compiling Java MainActivity (Release) ==="
rm -rf target/release_java_classes target/release_dex target/release_compiled_manifest
mkdir -p target/release_java_classes target/release_dex target/release_compiled_manifest

javac -g:none -source 8 -target 8 -cp "$ANDROID_HOME/platforms/android-34/android.jar" \
  java/com/example/android_video_player/*.java -d target/release_java_classes

echo "=== 3. Generating Optimized Dalvik Executable (classes.dex) ==="
"$ANDROID_HOME/build-tools/34.0.0/d8" --release --lib "$ANDROID_HOME/platforms/android-34/android.jar" \
  --output target/release_dex target/release_java_classes/com/example/android_video_player/*.class

echo "=== 4. Compiling Binary AndroidManifest.xml ==="
"$ANDROID_HOME/build-tools/34.0.0/aapt" package -f -M AndroidManifest.xml \
  -I "$ANDROID_HOME/platforms/android-34/android.jar" -F target/release_manifest_out.apk
unzip -o target/release_manifest_out.apk AndroidManifest.xml -d target/release_compiled_manifest/

echo "=== 5. Packaging AndroidManifest.xml, classes.dex & libandroid_video_player.so into Release APK ==="
rm -f target/release/apk/android_video_player_unaligned.apk
(cd target/release/apk && zip -r android_video_player_unaligned.apk lib/ > /dev/null)
zip -j target/release/apk/android_video_player_unaligned.apk target/release_compiled_manifest/AndroidManifest.xml > /dev/null
zip -j target/release/apk/android_video_player_unaligned.apk target/release_dex/classes.dex > /dev/null

echo "=== 6. Zipaligning Release APK ==="
"$ANDROID_HOME/build-tools/34.0.0/zipalign" -f 4 \
  target/release/apk/android_video_player_unaligned.apk target/release/apk/android_video_player_aligned.apk

echo "=== 7. Signing Final Release APK with Release Keystore ==="
"$ANDROID_HOME/build-tools/34.0.0/apksigner" sign --ks "$KEYSTORE_PATH" \
  --ks-pass pass:androidrelease \
  --key-pass pass:androidrelease \
  --ks-key-alias "$KEY_ALIAS" \
  target/release/apk/android_video_player_aligned.apk

echo "=== SUCCESS! Production Release APK built and signed at: target/release/apk/android_video_player_aligned.apk ==="
