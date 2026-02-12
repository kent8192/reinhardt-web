# reinhardt-mobile

Mobile application support for the Reinhardt framework, enabling cross-platform mobile development using WebView technology.

## Overview

`reinhardt-mobile` provides mobile application capabilities for Reinhardt by integrating with WRY (WebView Rendering) and TAO (Windowing) libraries. This crate enables developers to build native mobile applications for iOS and Android using Reinhardt's web framework features.

## Features

- **Cross-Platform WebView**: Leverage WRY for native WebView rendering on mobile platforms
- **Native Window Management**: Use TAO for platform-native window and event handling
- **Platform-Specific APIs**: Access Android (JNI/NDK) and iOS (Objective-C) native features
- **Manouche DSL Integration**: Utilize Reinhardt's Manouche DSL for UI component definitions

## Supported Platforms

- **Android**: Requires JNI and NDK for native integration
- **iOS**: Uses Objective-C runtime for native feature access
- **Desktop**: Also supports desktop platforms via WRY/TAO

## Architecture

This crate bridges Reinhardt's web framework with native mobile platforms:

1. **WebView Layer**: WRY provides the rendering engine (WebKit on iOS, Chromium on Android)
2. **Window Layer**: TAO handles native window creation and event loops
3. **Bridge Layer**: Platform-specific bindings for native feature access
4. **Reinhardt Integration**: Seamless integration with Reinhardt's routing and component system

## Dependencies

### Core Dependencies
- `reinhardt-manouche`: DSL definitions for UI components
- `wry`: Cross-platform WebView rendering
- `tao`: Cross-platform windowing and event handling
- `serde`/`serde_json`: Serialization for bridge communication

### Platform-Specific Dependencies
- **Android**: `jni`, `ndk`, `ndk-context`
- **iOS**: `objc2`, `objc2-foundation`

## Development Status

This crate is in **alpha** stage. APIs may change in future releases.

## Version

Current version: `0.1.0-alpha.1`

## License

Licensed under MIT OR Apache-2.0, consistent with the Reinhardt framework.
