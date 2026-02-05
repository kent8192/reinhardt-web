# iOS Build Guide

This guide covers building reinhardt-mobile applications for iOS.

## Prerequisites

### System Requirements

- **macOS**: 13.0 (Ventura) or later
- **Xcode**: 14.0 or later (install from App Store)
- **Rust**: Latest stable version
- **Command Line Tools**: Install via `xcode-select --install`

### Rust iOS Targets

Install the required iOS targets:

```bash
# For physical devices (arm64)
rustup target add aarch64-apple-ios

# For simulator (arm64 Apple Silicon)
rustup target add aarch64-apple-ios-sim

# For simulator (x86_64 Intel)
rustup target add x86_64-apple-ios
```

Verify installation:

```bash
rustup target list --installed | grep ios
```

## Environment Setup

### Install cargo-mobile2

cargo-mobile2 is the recommended tool for mobile Rust development:

```bash
cargo install cargo-mobile2
```

### Initialize Mobile Project

Initialize mobile support in your project:

```bash
cargo mobile init
```

This creates:
- `gen/apple/` - Xcode project files
- Platform-specific configuration files

### Verify Setup

```bash
cargo mobile doctor
```

Address any issues reported before proceeding.

## Build Steps

### Debug Build (Simulator)

```bash
# Build for iOS simulator
cargo mobile apple build --target aarch64-apple-ios-sim

# Run on simulator
cargo mobile apple run --target aarch64-apple-ios-sim
```

### Release Build (Device)

```bash
# Build for physical device
cargo mobile apple build --target aarch64-apple-ios --release
```

### Open in Xcode

For advanced configuration or debugging:

```bash
cargo mobile apple open
```

This opens the generated Xcode project in `gen/apple/`.

### Build via Xcode

1. Open `gen/apple/<project-name>.xcodeproj`
2. Select target device or simulator
3. Press `Cmd + B` to build or `Cmd + R` to run

## Code Signing and Provisioning

### Development Setup

1. **Apple Developer Account**: Required for device testing
2. **Xcode Signing**: Open project in Xcode and configure:
   - Select your Team in Signing & Capabilities
   - Enable "Automatically manage signing"

### Manual Signing Configuration

Edit `gen/apple/<project-name>.xcodeproj/project.pbxproj` or configure in Xcode:

| Setting | Value |
|---------|-------|
| Development Team | Your Team ID (10-character string) |
| Bundle Identifier | Unique reverse-domain identifier |
| Provisioning Profile | Automatic or specific profile |

### Distribution Build

For App Store or Ad Hoc distribution:

1. Create distribution certificate in Apple Developer Portal
2. Create App ID matching your bundle identifier
3. Create provisioning profile (App Store or Ad Hoc)
4. In Xcode, select the profile under Signing & Capabilities
5. Archive: Product > Archive

## Troubleshooting

### Target Not Found

**Error**: `error: linking with cc failed`

**Solution**: Install missing target:
```bash
rustup target add aarch64-apple-ios-sim
```

### Xcode Command Line Tools

**Error**: `xcrun: error: unable to find utility`

**Solution**:
```bash
xcode-select --install
sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
```

### Code Signing Errors

**Error**: `Signing requires a development team`

**Solution**:
1. Open Xcode project
2. Select target > Signing & Capabilities
3. Select your development team
4. Enable automatic signing

### Simulator Architecture Mismatch

**Error**: `building for iOS Simulator, but linking in dylib built for iOS`

**Solution**: Ensure correct target:
```bash
# Apple Silicon Mac
cargo mobile apple build --target aarch64-apple-ios-sim

# Intel Mac
cargo mobile apple build --target x86_64-apple-ios
```

### WRY WebView Issues

**Error**: WebView fails to initialize on simulator

**Solution**: iOS Simulator has WebView limitations. Test on physical device for full functionality.

### Minimum iOS Version

**Error**: `deployment target is too low`

**Solution**: Edit `gen/apple/project.yml` or Xcode project:
- Set minimum deployment target to iOS 14.0 or higher

## Additional Resources

- [cargo-mobile2 Documentation](https://github.com/nickelpack/cargo-mobile2)
- [WRY iOS Support](https://github.com/nickelpack/nickelpack/tree/main/nickel-wry)
- [Apple Developer Documentation](https://developer.apple.com/documentation/)
