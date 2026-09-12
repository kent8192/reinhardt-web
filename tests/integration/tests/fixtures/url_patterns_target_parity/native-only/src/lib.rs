//! Native-only dependency used by real consumer endpoint code.

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
compile_error!("native-only fixture dependency reached the browser WASM graph");

pub struct NativeMarker;

impl NativeMarker {
	pub fn body() -> &'static str {
		"native-handler"
	}
}
