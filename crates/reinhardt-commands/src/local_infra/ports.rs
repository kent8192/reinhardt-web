//! Host port selection for local infrastructure services.

use std::io;
use std::net::TcpListener;

/// Selects available host ports.
#[derive(Debug, Default, Clone, Copy)]
pub struct PortAllocator;

impl PortAllocator {
	/// Return the requested port if free, otherwise return an OS-assigned port.
	pub fn select_port(&self, requested: u16) -> io::Result<u16> {
		if requested != 0 && is_available(requested) {
			return Ok(requested);
		}
		let listener = TcpListener::bind("127.0.0.1:0")?;
		Ok(listener.local_addr()?.port())
	}
}

fn is_available(port: u16) -> bool {
	TcpListener::bind(("127.0.0.1", port)).is_ok()
}
