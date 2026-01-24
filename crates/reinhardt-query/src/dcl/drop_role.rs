//! DROP ROLE statement builder

/// DROP ROLE statement builder
#[derive(Debug, Clone, Default)]
pub struct DropRoleStatement {
	/// Role names to drop
	pub role_names: Vec<String>,
	/// IF EXISTS clause
	pub if_exists: bool,
}

impl DropRoleStatement {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn role(mut self, name: impl Into<String>) -> Self {
		self.role_names.push(name.into());
		self
	}

	pub fn roles(mut self, names: Vec<String>) -> Self {
		self.role_names = names;
		self
	}

	pub fn if_exists(mut self, flag: bool) -> Self {
		self.if_exists = flag;
		self
	}

	pub fn validate(&self) -> Result<(), String> {
		if self.role_names.is_empty() {
			return Err("At least one role name is required".to_string());
		}
		Ok(())
	}
}
