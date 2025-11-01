//! Model and migration lifecycle events

use crate::core::SignalName;
use crate::registry::get_signal;
use crate::signal::Signal;
use std::fmt;

// ========================================
// Model lifecycle events
// ========================================

/// M2M changed signal - sent when many-to-many relationships change
#[derive(Debug, Clone)]
pub struct M2MChangeEvent<T, R> {
	pub instance: T,
	pub action: M2MAction,
	pub related: Vec<R>,
	pub reverse: bool,
	pub model_name: String,
}

impl<T, R> M2MChangeEvent<T, R> {
	pub fn new(instance: T, action: M2MAction, related: Vec<R>) -> Self {
		Self {
			instance,
			action,
			related,
			reverse: false,
			model_name: String::new(),
		}
	}

	pub fn with_reverse(mut self, reverse: bool) -> Self {
		self.reverse = reverse;
		self
	}

	pub fn with_model_name(mut self, model_name: impl Into<String>) -> Self {
		self.model_name = model_name.into();
		self
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum M2MAction {
	PreAdd,
	PostAdd,
	PreRemove,
	PostRemove,
	PreClear,
	PostClear,
}

impl fmt::Display for M2MAction {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			M2MAction::PreAdd => write!(f, "pre_add"),
			M2MAction::PostAdd => write!(f, "post_add"),
			M2MAction::PreRemove => write!(f, "pre_remove"),
			M2MAction::PostRemove => write!(f, "post_remove"),
			M2MAction::PreClear => write!(f, "pre_clear"),
			M2MAction::PostClear => write!(f, "post_clear"),
		}
	}
}

pub fn m2m_changed<T: Send + Sync + 'static, R: Send + Sync + 'static>()
-> Signal<M2MChangeEvent<T, R>> {
	get_signal::<M2MChangeEvent<T, R>>(SignalName::M2M_CHANGED)
}

/// Pre-init signal - sent at the beginning of a model's __init__ method
#[derive(Debug, Clone)]
pub struct PreInitEvent<T> {
	pub model_type: String,
	pub args: Vec<String>,
	_phantom: std::marker::PhantomData<T>,
}

impl<T> PreInitEvent<T> {
	pub fn new(model_type: impl Into<String>) -> Self {
		Self {
			model_type: model_type.into(),
			args: Vec::new(),
			_phantom: std::marker::PhantomData,
		}
	}

	pub fn with_args(mut self, args: Vec<String>) -> Self {
		self.args = args;
		self
	}
}

pub fn pre_init<T: Send + Sync + 'static>() -> Signal<PreInitEvent<T>> {
	get_signal::<PreInitEvent<T>>(SignalName::PRE_INIT)
}

/// Post-init signal - sent at the end of a model's __init__ method
#[derive(Debug, Clone)]
pub struct PostInitEvent<T> {
	pub instance: T,
}

impl<T> PostInitEvent<T> {
	pub fn new(instance: T) -> Self {
		Self { instance }
	}
}

pub fn post_init<T: Send + Sync + 'static>() -> Signal<PostInitEvent<T>> {
	get_signal::<PostInitEvent<T>>(SignalName::POST_INIT)
}

// ========================================
// Migration lifecycle events
// ========================================

/// Pre-migrate signal - sent before running migrations
#[derive(Debug, Clone)]
pub struct MigrationEvent {
	pub app_name: String,
	pub migration_name: String,
	pub plan: Vec<String>,
}

impl MigrationEvent {
	pub fn new(app_name: impl Into<String>, migration_name: impl Into<String>) -> Self {
		Self {
			app_name: app_name.into(),
			migration_name: migration_name.into(),
			plan: Vec::new(),
		}
	}

	pub fn with_plan(mut self, plan: Vec<String>) -> Self {
		self.plan = plan;
		self
	}
}

pub fn pre_migrate() -> Signal<MigrationEvent> {
	get_signal::<MigrationEvent>(SignalName::PRE_MIGRATE)
}

/// Post-migrate signal - sent after running migrations
pub fn post_migrate() -> Signal<MigrationEvent> {
	get_signal::<MigrationEvent>(SignalName::POST_MIGRATE)
}

/// Class prepared signal - sent when a model class is prepared
#[derive(Debug, Clone)]
pub struct ClassPreparedEvent {
	pub model_name: String,
	pub app_label: String,
}

impl ClassPreparedEvent {
	pub fn new(model_name: impl Into<String>, app_label: impl Into<String>) -> Self {
		Self {
			model_name: model_name.into(),
			app_label: app_label.into(),
		}
	}
}

pub fn class_prepared() -> Signal<ClassPreparedEvent> {
	get_signal::<ClassPreparedEvent>(SignalName::CLASS_PREPARED)
}
