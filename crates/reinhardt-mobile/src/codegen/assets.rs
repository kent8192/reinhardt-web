//! Asset generation for mobile applications.
//!
//! Generates HTML, CSS, and JavaScript assets for WebView embedding.

use crate::{MobileConfig, MobileResult, TargetPlatform};

/// Generated assets for a mobile application.
#[derive(Debug, Clone)]
pub struct MobileAssets {
	/// HTML content
	pub html: String,
	/// CSS content
	pub css: String,
	/// JavaScript content
	pub js: String,
}

/// Asset generator for mobile applications.
pub struct AssetGenerator {
	config: MobileConfig,
}

impl AssetGenerator {
	/// Creates a new asset generator with the given configuration.
	pub fn new(config: MobileConfig) -> Self {
		Self { config }
	}

	/// Generates all assets for the mobile application.
	pub fn generate(&self) -> MobileResult<MobileAssets> {
		Ok(MobileAssets {
			html: self.generate_html(),
			css: self.generate_css(),
			js: self.generate_js(),
		})
	}

	/// Generates the HTML template.
	fn generate_html(&self) -> String {
		format!(
			r#"<!DOCTYPE html>
<html lang="en">
<head>
	<meta charset="UTF-8">
	<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
	<title>{}</title>
	<link rel="stylesheet" href="app.css">
</head>
<body>
	<div id="app"></div>
	<script src="app.js"></script>
</body>
</html>"#,
			self.config.app_name
		)
	}

	/// Generates the CSS styles.
	fn generate_css(&self) -> String {
		r#"/* reinhardt-mobile base styles */
* {
	box-sizing: border-box;
	margin: 0;
	padding: 0;
}

html, body {
	width: 100%;
	height: 100%;
	font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
	-webkit-font-smoothing: antialiased;
	-moz-osx-font-smoothing: grayscale;
}

#app {
	width: 100%;
	height: 100%;
}

/* Disable text selection on mobile */
body {
	-webkit-user-select: none;
	-moz-user-select: none;
	user-select: none;
}

/* Safe area insets for notched devices */
body {
	padding-top: env(safe-area-inset-top);
	padding-bottom: env(safe-area-inset-bottom);
	padding-left: env(safe-area-inset-left);
	padding-right: env(safe-area-inset-right);
}
"#
		.to_string()
	}

	/// Generates the JavaScript runtime.
	fn generate_js(&self) -> String {
		let protocol = self.get_protocol_scheme();

		format!(
			r#"// reinhardt-mobile runtime
(function() {{
	'use strict';

	// IPC Bridge
	window.__REINHARDT_IPC__ = {{
		_requestId: 0,
		_pending: new Map(),

		invoke: function(command, payload) {{
			return new Promise((resolve, reject) => {{
				const requestId = String(++this._requestId);
				this._pending.set(requestId, {{ resolve, reject }});

				const message = JSON.stringify({{
					command: command,
					payload: payload || {{}},
					request_id: requestId
				}});

				// Platform-specific IPC
				if (window.ipc) {{
					window.ipc.postMessage(message);
				}} else if (window.webkit && window.webkit.messageHandlers) {{
					window.webkit.messageHandlers.ipc.postMessage(message);
				}} else {{
					reject(new Error('No IPC bridge available'));
				}}
			}});
		}},

		receive: function(response) {{
			const data = typeof response === 'string' ? JSON.parse(response) : response;
			const pending = this._pending.get(data.request_id);

			if (pending) {{
				this._pending.delete(data.request_id);
				if (data.success) {{
					pending.resolve(data.data);
				}} else {{
					pending.reject(new Error(data.error || 'Unknown error'));
				}}
			}}
		}}
	}};

	// Protocol scheme for asset loading
	window.__REINHARDT_PROTOCOL__ = '{protocol}';

	// Initialize app
	document.addEventListener('DOMContentLoaded', function() {{
		console.log('reinhardt-mobile initialized');
		window.__REINHARDT_IPC__.invoke('app_ready', {{}});
	}});
}})();
"#
		)
	}

	/// Returns the protocol scheme based on platform.
	fn get_protocol_scheme(&self) -> &'static str {
		match self.config.platform {
			TargetPlatform::Android => "http://wry",
			TargetPlatform::Ios => "wry",
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_generate_assets() {
		let config = MobileConfig::default();
		let generator = AssetGenerator::new(config);
		let assets = generator.generate().unwrap();

		assert!(assets.html.contains("<!DOCTYPE html>"));
		assert!(assets.css.contains("box-sizing"));
		assert!(assets.js.contains("__REINHARDT_IPC__"));
	}
}
