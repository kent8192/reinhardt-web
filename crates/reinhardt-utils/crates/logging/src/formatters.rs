pub fn escape_control_chars(s: &str) -> String {
	let mut result = String::with_capacity(s.len());

	for ch in s.chars() {
		if ch.is_control() || !ch.is_ascii() {
			// Escape control characters and non-ASCII characters as \xNN
			for byte in ch.to_string().as_bytes() {
				result.push_str(&format!("\\x{:02x}", byte));
			}
		} else {
			result.push(ch);
		}
	}

	result
}
pub trait Formatter {}
pub struct ServerFormatter;
pub struct StandardFormatter;
