//! Command registry

use crate::BaseCommand;
use std::collections::HashMap;

pub struct CommandRegistry {
	commands: HashMap<String, Box<dyn BaseCommand>>,
}

impl CommandRegistry {
	pub fn new() -> Self {
		Self {
			commands: HashMap::new(),
		}
	}

	pub fn register(&mut self, command: Box<dyn BaseCommand>) {
		let name = command.name().to_string();
		self.commands.insert(name, command);
	}

	pub fn get(&self, name: &str) -> Option<&dyn BaseCommand> {
		self.commands.get(name).map(|cmd| &**cmd)
	}

	pub fn list(&self) -> Vec<&str> {
		self.commands.keys().map(|s| s.as_str()).collect()
	}
}

impl Default for CommandRegistry {
	fn default() -> Self {
		Self::new()
	}
}
