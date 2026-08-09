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

# Load or generate secure keystore password stored in git-ignored .env.release_key
ENV_FILE=".env.release_key"
KEYSTORE_PATH="release.keystore"
KEY_ALIAS="release-key"

if [ -f "$ENV_FILE" ]; then
    source "$ENV_FILE"
fi

if [ -z "$KEYSTORE_PASS" ]; then
    # Generate secure 32-character random password
    KEYSTORE_PASS=$(openssl rand -hex 16 2>/dev/null || date +%s | sha256sum | base64 | head -c 32)
    KEY_PASS="$KEYSTORE_PASS"
    echo "KEYSTORE_PASS=\"$KEYSTORE_PASS\"" > "$ENV_FILE"
    echo "KEY_PASS=\"$KEY_PASS\"" >> "$ENV_FILE"
    chmod 600 "$ENV_FILE"
    echo "=== Generated secure random password stored in git-ignored '$ENV_FILE' ==="
fi

if [ ! -f "$KEYSTORE_PATH" ]; then
    echo "=== Generating fresh secure production keystore '$KEYSTORE_PATH' ==="
    keytool -genkey -v -keystore "$KEYSTORE_PATH" \
        -alias "$KEY_ALIAS" \
        -keyalg RSA -keysize 2048 \
        -validity 10000 \
        -storepass "$KEYSTORE_PASS" \
        -keypass "$KEY_PASS" \
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

echo "=== 4. Compiling Binary AndroidManifest.xml & Resources ==="
"$ANDROID_HOME/build-tools/34.0.0/aapt" package -f -M AndroidManifest.xml -S res \
  -I "$ANDROID_HOME/platforms/android-34/android.jar" -F target/release_manifest_out.apk
unzip -o target/release_manifest_out.apk -d target/release_compiled_manifest/

echo "=== 5. Packaging AndroidManifest.xml, Resources, classes.dex & libandroid_video_player.so into Release APK ==="
rm -f target/release/apk/android_video_player_unaligned.apk
(cd target/release/apk && zip -r android_video_player_unaligned.apk lib/ > /dev/null)
(cd target/release_compiled_manifest && zip -r ../release/apk/android_video_player_unaligned.apk . > /dev/null)
zip -j target/release/apk/android_video_player_unaligned.apk target/release_dex/classes.dex > /dev/null

echo "=== 6. Zipaligning Release APK ==="
"$ANDROID_HOME/build-tools/34.0.0/zipalign" -f 4 \
  target/release/apk/android_video_player_unaligned.apk target/release/apk/android_video_player_aligned.apk

echo "=== 7. Signing Final Release APK with Release Keystore ==="
"$ANDROID_HOME/build-tools/34.0.0/apksigner" sign --ks "$KEYSTORE_PATH" \
  --ks-pass "pass:$KEYSTORE_PASS" \
  --key-pass "pass:$KEY_PASS" \
  --ks-key-alias "$KEY_ALIAS" \
  target/release/apk/android_video_player_aligned.apk

echo "=== SUCCESS! Production Release APK built and signed at: target/release/apk/android_video_player_aligned.apk ==="
