//! ALTER ROLE statement builder

use super::{RoleAttribute, UserOption};

/// ALTER ROLE statement builder
#[derive(Debug, Clone, Default)]
pub struct AlterRoleStatement {
	/// Role name to alter
	pub role_name: String,
	/// PostgreSQL role attributes
	pub attributes: Vec<RoleAttribute>,
	/// MySQL user options
	pub options: Vec<UserOption>,
	/// New role name (RENAME TO)
	pub rename_to: Option<String>,
}

impl AlterRoleStatement {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn role(mut self, name: impl Into<String>) -> Self {
		self.role_name = name.into();
		self
	}

	pub fn attribute(mut self, attr: RoleAttribute) -> Self {
		self.attributes.push(attr);
		self
	}

	pub fn attributes(mut self, attrs: Vec<RoleAttribute>) -> Self {
		self.attributes = attrs;
		self
	}

	pub fn option(mut self, opt: UserOption) -> Self {
		self.options.push(opt);
		self
	}

	pub fn options(mut self, opts: Vec<UserOption>) -> Self {
		self.options = opts;
		self
	}

	pub fn rename_to(mut self, new_name: impl Into<String>) -> Self {
		self.rename_to = Some(new_name.into());
		self
	}

	pub fn validate(&self) -> Result<(), String> {
		if self.role_name.is_empty() {
			return Err("Role name cannot be empty".to_string());
		}
		if self.attributes.is_empty() && self.options.is_empty() && self.rename_to.is_none() {
			return Err("At least one attribute, option, or rename must be specified".to_string());
		}
		Ok(())
	}
}
