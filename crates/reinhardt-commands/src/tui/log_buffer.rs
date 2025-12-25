use chrono::{DateTime, Utc};
use std::collections::VecDeque;

/// Log source (backend or frontend)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
	Backend,
	Frontend,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
	Trace = 0,
	Debug = 1,
	Info = 2,
	Warn = 3,
	Error = 4,
}

impl LogLevel {
	/// Parse log level from a log line
	/// Supports formats like: [TRACE], TRACE:, [trace], etc.
	pub fn parse(line: &str) -> Option<Self> {
		let line_upper = line.to_uppercase();

		if line_upper.contains("ERROR") {
			Some(LogLevel::Error)
		} else if line_upper.contains("WARN") {
			Some(LogLevel::Warn)
		} else if line_upper.contains("INFO") {
			Some(LogLevel::Info)
		} else if line_upper.contains("DEBUG") {
			Some(LogLevel::Debug)
		} else if line_upper.contains("TRACE") {
			Some(LogLevel::Trace)
		} else {
			None
		}
	}

	/// Cycle to next filter level: Trace → Warn → Error → Trace
	pub fn cycle_filter(&self) -> Self {
		match self {
			LogLevel::Trace => LogLevel::Warn,  // Show WARN and ERROR
			LogLevel::Warn => LogLevel::Error,  // Show ERROR only
			LogLevel::Error => LogLevel::Trace, // Show all
			_ => LogLevel::Trace,
		}
	}
}

impl std::fmt::Display for LogLevel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			LogLevel::Trace => write!(f, "ALL"),
			LogLevel::Debug => write!(f, "DEBUG+"),
			LogLevel::Info => write!(f, "INFO+"),
			LogLevel::Warn => write!(f, "WARN+"),
			LogLevel::Error => write!(f, "ERROR"),
		}
	}
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
	pub source: LogSource,
	pub level: Option<LogLevel>,
	pub timestamp: DateTime<Utc>,
	pub message: String,
}

impl LogEntry {
	/// Create a new log entry
	pub fn new(source: LogSource, message: String) -> Self {
		Self {
			source,
			level: LogLevel::parse(&message),
			timestamp: Utc::now(),
			message,
		}
	}
}

/// Log buffer (scrollable collection of log entries)
pub struct LogBuffer {
	entries: VecDeque<LogEntry>,
	max_lines: usize,
	scroll_offset: usize,
}

impl LogBuffer {
	/// Create a new log buffer
	pub fn new(max_lines: usize) -> Self {
		Self {
			entries: VecDeque::new(),
			max_lines,
			scroll_offset: 0,
		}
	}

	/// Add a log entry
	pub fn push(&mut self, entry: LogEntry) {
		self.entries.push_back(entry);
		if self.entries.len() > self.max_lines {
			self.entries.pop_front();
		}
	}

	/// Get all entries (no filtering)
	pub fn entries(&self) -> impl Iterator<Item = &LogEntry> {
		self.entries.iter()
	}

	/// Get filtered entries based on minimum log level
	pub fn filtered_entries(&self, min_level: LogLevel) -> impl Iterator<Item = &LogEntry> {
		self.entries.iter().filter(move |entry| {
			if let Some(level) = entry.level {
				level >= min_level
			} else {
				// If no level detected, show it (assume it's important)
				true
			}
		})
	}

	/// Get the number of entries
	pub fn len(&self) -> usize {
		self.entries.len()
	}

	/// Check if the buffer is empty
	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}

	/// Get the scroll offset
	pub fn scroll_offset(&self) -> usize {
		self.scroll_offset
	}

	/// Set the scroll offset
	pub fn set_scroll_offset(&mut self, offset: usize) {
		self.scroll_offset = offset.min(self.entries.len().saturating_sub(1));
	}

	/// Scroll up by one line (increase offset)
	pub fn scroll_up(&mut self, lines: usize) {
		let max_offset = self.entries.len().saturating_sub(1);
		self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
	}

	/// Scroll down by one line (decrease offset)
	pub fn scroll_down(&mut self, lines: usize) {
		self.scroll_offset = self.scroll_offset.saturating_sub(lines);
	}

	/// Scroll to the top (maximum offset)
	pub fn scroll_to_top(&mut self) {
		self.scroll_offset = self.entries.len().saturating_sub(1);
	}

	/// Scroll to the bottom (offset 0)
	pub fn scroll_to_bottom(&mut self) {
		self.scroll_offset = 0;
	}

	/// Clear all logs
	pub fn clear(&mut self) {
		self.entries.clear();
		self.scroll_offset = 0;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_log_entry_creation() {
		let entry = LogEntry::new(LogSource::Backend, "Test message".to_string());
		assert_eq!(entry.source, LogSource::Backend);
		assert_eq!(entry.message, "Test message");
	}

	#[test]
	fn test_log_buffer_push() {
		let mut buffer = LogBuffer::new(10);
		buffer.push(LogEntry::new(LogSource::Backend, "Line 1".to_string()));
		buffer.push(LogEntry::new(LogSource::Backend, "Line 2".to_string()));

		assert_eq!(buffer.len(), 2);
	}

	#[test]
	fn test_log_buffer_overflow() {
		let mut buffer = LogBuffer::new(10);
		for i in 0..15 {
			buffer.push(LogEntry::new(LogSource::Backend, format!("Line {}", i)));
		}

		assert_eq!(buffer.len(), 10);
		// Old 5 lines (0-4) are removed, 5-14 remain
		let first_message = &buffer.entries().next().unwrap().message;
		assert!(first_message.contains("Line 5"));
	}

	#[test]
	fn test_log_buffer_clear() {
		let mut buffer = LogBuffer::new(10);
		buffer.push(LogEntry::new(LogSource::Backend, "Test".to_string()));
		assert_eq!(buffer.len(), 1);

		buffer.clear();
		assert_eq!(buffer.len(), 0);
		assert_eq!(buffer.scroll_offset(), 0);
	}
}
