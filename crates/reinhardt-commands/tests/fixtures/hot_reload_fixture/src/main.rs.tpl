//! Test-fixture `manage` binary.
//!
//! Prints a marker line so test code can confirm which build of the binary
//! is currently running. By default it sleeps for an hour so the hot-reload
//! test can "kill and respawn" us at will. When `HOT_RELOAD_LISTEN_ADDR` is
//! set, it instead binds a tiny TCP listener at that address.

fn main() {
	println!("server marker: {{MARKER}}");
	if let Ok(addr) = std::env::var("HOT_RELOAD_LISTEN_ADDR") {
		let listener = std::net::TcpListener::bind(&addr).expect("bind hot-reload fixture listener");
		for stream in listener.incoming() {
			let mut stream = stream.expect("accept hot-reload fixture connection");
			let _ = std::io::Write::write_all(
				&mut stream,
				b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK",
			);
		}
		return;
	}
	std::thread::sleep(std::time::Duration::from_secs(3600));
}
