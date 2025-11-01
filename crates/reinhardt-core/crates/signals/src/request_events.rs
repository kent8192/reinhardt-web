//! Request lifecycle events

use crate::core::SignalName;
use crate::registry::get_signal;
use crate::signal::Signal;
use std::collections::HashMap;

/// Request started signal - sent when an HTTP request starts
#[derive(Debug, Clone)]
pub struct RequestStartedEvent {
	pub environ: HashMap<String, String>,
}

impl RequestStartedEvent {
	pub fn new() -> Self {
		Self {
			environ: HashMap::new(),
		}
	}

	pub fn with_environ(mut self, environ: HashMap<String, String>) -> Self {
		self.environ = environ;
		self
	}
}

impl Default for RequestStartedEvent {
	fn default() -> Self {
		Self::new()
	}
}

pub fn request_started() -> Signal<RequestStartedEvent> {
	get_signal::<RequestStartedEvent>(SignalName::REQUEST_STARTED)
}

/// Request finished signal - sent when an HTTP request finishes
#[derive(Debug, Clone)]
pub struct RequestFinishedEvent {
	pub environ: HashMap<String, String>,
}

impl RequestFinishedEvent {
	pub fn new() -> Self {
		Self {
			environ: HashMap::new(),
		}
	}

	pub fn with_environ(mut self, environ: HashMap<String, String>) -> Self {
		self.environ = environ;
		self
	}
}

impl Default for RequestFinishedEvent {
	fn default() -> Self {
		Self::new()
	}
}

pub fn request_finished() -> Signal<RequestFinishedEvent> {
	get_signal::<RequestFinishedEvent>(SignalName::REQUEST_FINISHED)
}

/// Got request exception signal - sent when an exception occurs during request handling
#[derive(Debug, Clone)]
pub struct GotRequestExceptionEvent {
	pub error_message: String,
	pub request_info: HashMap<String, String>,
}

impl GotRequestExceptionEvent {
	pub fn new(error_message: impl Into<String>) -> Self {
		Self {
			error_message: error_message.into(),
			request_info: HashMap::new(),
		}
	}

	pub fn with_request_info(mut self, request_info: HashMap<String, String>) -> Self {
		self.request_info = request_info;
		self
	}
}

pub fn got_request_exception() -> Signal<GotRequestExceptionEvent> {
	get_signal::<GotRequestExceptionEvent>(SignalName::GOT_REQUEST_EXCEPTION)
}

/// Setting changed signal - sent when a setting is changed
#[derive(Debug, Clone)]
pub struct SettingChangedEvent {
	pub setting_name: String,
	pub old_value: Option<String>,
	pub new_value: String,
}

impl SettingChangedEvent {
	pub fn new(
		setting_name: impl Into<String>,
		old_value: Option<String>,
		new_value: impl Into<String>,
	) -> Self {
		Self {
			setting_name: setting_name.into(),
			old_value,
			new_value: new_value.into(),
		}
	}
}

pub fn setting_changed() -> Signal<SettingChangedEvent> {
	get_signal::<SettingChangedEvent>(SignalName::SETTING_CHANGED)
}
