# reinhardt-desktop

Desktop application framework for Reinhardt using wry/tao.

## Overview

This crate enables building cross-platform desktop applications from the same
`reinhardt-manouche` DSL used for web applications. It uses `tao` for window
management and `wry` for WebView embedding.

## Features

- **Cross-platform**: Supports Linux, macOS, and Windows
- **Shared DSL**: Same `reinhardt-manouche` source produces identical behavior
- **WebView-based**: Uses native WebView for rendering
- **Custom Protocol**: `reinhardt://` scheme for secure asset loading
- **IPC Bridge**: Bidirectional communication between Rust and JavaScript

## Architecture

```text
┌────────────────────────────────────────────────────────┐
│                    reinhardt-desktop                   │
├────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐ │
│  │    tao      │    │    wry      │    │    IPC      │ │
│  │  (Window)   │───▶│  (WebView)  │◀──▶│  (Bridge)   │ │
│  └─────────────┘    └─────────────┘    └─────────────┘ │
│                            │                           │
│                            ▼                           │
│              ┌─────────────────────────┐               │
│              │  Custom Protocol        │               │
│              │  reinhardt://localhost/ │               │
│              └─────────────────────────┘               │
│                            │                           │
│                            ▼                           │
│              ┌─────────────────────────┐               │
│              │  reinhardt-manouche     │               │
│              │  Generated HTML/CSS/JS  │               │
│              └─────────────────────────┘               │
└────────────────────────────────────────────────────────┘
```

## Quick Start

```rust,ignore
use reinhardt_desktop::{DesktopApp, WindowConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = DesktopApp::builder()
        .title("My App")
        .size(800, 600)
        .build()?;

    app.run()
}
```

## License

MIT OR Apache-2.0
