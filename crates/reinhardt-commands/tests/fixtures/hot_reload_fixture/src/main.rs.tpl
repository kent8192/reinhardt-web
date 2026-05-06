//! Test-fixture `manage` binary.
//!
//! Prints a marker line so test code can confirm which build of the binary
//! is currently running, then sleeps for an hour so the hot-reload test can
//! "kill and respawn" us at will.

fn main() {
	println!("server marker: {{MARKER}}");
	std::thread::sleep(std::time::Duration::from_secs(3600));
}
