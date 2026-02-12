# Android Build Guide

This guide covers building reinhardt-mobile applications for Android.

## Prerequisites

### Android NDK

- **Version**: NDK r25 or higher (r25c recommended)
- **Download**: [Android NDK Downloads](https://developer.android.com/ndk/downloads)

Set environment variables:

```bash
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/25.2.9519653
export NDK_HOME=$ANDROID_NDK_HOME
```

### Android SDK

- **Minimum API Level**: 26 (Android 8.0 Oreo)
- **Target API Level**: 34 (Android 14) recommended

```bash
export ANDROID_HOME=$HOME/Library/Android/sdk
export PATH=$PATH:$ANDROID_HOME/platform-tools:$ANDROID_HOME/tools
```

### Rust Targets

Install Android targets:

```bash
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android
rustup target add i686-linux-android
```

### Java Development Kit

- **Version**: JDK 17 or higher

```bash
# macOS (Homebrew)
brew install openjdk@17

# Set JAVA_HOME
export JAVA_HOME=$(/usr/libexec/java_home -v 17)
```

## Environment Setup

### Install cargo-mobile2

```bash
cargo install cargo-mobile2
```

Verify installation:

```bash
cargo mobile --version
```

### Initialize Mobile Project

Navigate to your project and initialize:

```bash
cargo mobile init
```

This generates:
- `gen/android/` - Android project files
- `.cargo/config.toml` - Cargo build configuration

### Configure Android Build

Edit `gen/android/app/build.gradle.kts`:

```kotlin
android {
    namespace = "com.example.reinhardt"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.example.reinhardt"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"
    }
}
```

## Build Steps

### Debug Build

```bash
cargo mobile android build
```

### Release Build

```bash
cargo mobile android build --release
```

### Run on Device/Emulator

```bash
# List available devices
cargo mobile android devices

# Run on connected device
cargo mobile android run
```

### Build APK

```bash
# Debug APK
cargo mobile android apk

# Release APK (requires signing)
cargo mobile android apk --release
```

Output location: `gen/android/app/build/outputs/apk/`

## JNI Binding Setup

### Add JNI Dependencies

In `Cargo.toml`:

```toml
[target.'cfg(target_os = "android")'.dependencies]
jni = "0.21"
ndk = "0.9"
ndk-context = "0.1"
```

### Create JNI Entry Point

```rust
#[cfg(target_os = "android")]
use jni::JNIEnv;
#[cfg(target_os = "android")]
use jni::objects::JClass;

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_example_reinhardt_MainActivity_initReinhardt(
    _env: JNIEnv,
    _class: JClass,
) {
    // Initialize Reinhardt mobile runtime
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Debug)
    );
}
```

### Access Android Context

```rust
#[cfg(target_os = "android")]
use ndk_context::android_context;

#[cfg(target_os = "android")]
fn get_android_context() {
    let ctx = android_context();
    let vm = unsafe { ctx.vm().as_ref() }.expect("No JavaVM");
    let activity = unsafe { ctx.context().as_ref() };
    // Use vm and activity
}
```

### Configure ProGuard

Add to `gen/android/app/proguard-rules.pro`:

```proguard
-keep class com.example.reinhardt.** { *; }
-keepclassmembers class * {
    native <methods>;
}
```

## Troubleshooting

### NDK Not Found

**Error**: `NDK not found`

**Solution**: Verify `ANDROID_NDK_HOME` is set correctly:

```bash
echo $ANDROID_NDK_HOME
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/
```

### Linker Errors

**Error**: `cannot find -lc++_shared`

**Solution**: Add to `.cargo/config.toml`:

```toml
[target.aarch64-linux-android]
linker = "aarch64-linux-android21-clang"

[target.armv7-linux-androideabi]
linker = "armv7a-linux-androideabi21-clang"
```

### API Level Mismatch

**Error**: `requires API level 26 but got 21`

**Solution**: Update minimum SDK in `build.gradle.kts`:

```kotlin
defaultConfig {
    minSdk = 26
}
```

### JNI Symbol Not Found

**Error**: `java.lang.UnsatisfiedLinkError`

**Solution**:
1. Verify function naming follows JNI convention
2. Ensure `#[no_mangle]` attribute is present
3. Check library is loaded in Java:

```java
static {
    System.loadLibrary("reinhardt");
}
```

### Gradle Sync Failed

**Error**: `Could not resolve all dependencies`

**Solution**: Update Gradle wrapper and dependencies:

```bash
cd gen/android
./gradlew wrapper --gradle-version=8.5
./gradlew --refresh-dependencies
```

### WebView Not Loading

**Error**: WebView shows blank or error

**Solution**: Add internet permission to `AndroidManifest.xml`:

```xml
<uses-permission android:name="android.permission.INTERNET" />
```

For local content, enable file access:

```kotlin
webView.settings.apply {
    allowFileAccess = true
    allowContentAccess = true
}
```

## See Also

- [reinhardt-mobile README](../README.md)
- [WRY Documentation](https://docs.rs/wry)
- [TAO Documentation](https://docs.rs/tao)
- [cargo-mobile2 Documentation](https://github.com/nickytonline/cargo-mobile2)
