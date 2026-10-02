//! Command registry

use crate::BaseCommand;
use crate::capabilities::CapabilityCommand;
use std::collections::HashMap;

/// Registry that stores and provides access to management commands by name.
pub struct CommandRegistry {
	commands: HashMap<String, Box<dyn BaseCommand>>,
	capability_commands: HashMap<String, Box<dyn CapabilityCommand>>,
}

impl CommandRegistry {
	/// Creates a new empty command registry.
	pub fn new() -> Self {
		Self {
			commands: HashMap::new(),
			capability_commands: HashMap::new(),
		}
	}

	/// Registers a command, overwriting any existing command with the same name.
	pub fn register(&mut self, command: Box<dyn BaseCommand>) {
		let name = command.name().to_string();
		self.capability_commands.remove(&name);
		self.commands.insert(name, command);
	}

	/// Register a capability-aware command under its clap command name.
	pub fn register_capability(&mut self, command: Box<dyn CapabilityCommand>) {
		let name = command.cli().get_name().to_owned();
		self.commands.remove(&name);
		self.capability_commands.insert(name, command);
	}

	/// Find a capability-aware command.
	pub fn get_capability(&self, name: &str) -> Option<&dyn CapabilityCommand> {
		self.capability_commands.get(name).map(|command| &**command)
	}

	/// Returns a reference to the command with the given name, if registered.
	pub fn get(&self, name: &str) -> Option<&dyn BaseCommand> {
		self.commands.get(name).map(|cmd| &**cmd)
	}

	/// Returns a list of all registered command names.
	pub fn list(&self) -> Vec<&str> {
		let mut commands: Vec<&str> = self.commands.keys().map(|name| name.as_str()).collect();
		commands.extend(self.capability_commands.keys().map(String::as_str));
		commands.sort_unstable();
		commands
	}
}

impl Default for CommandRegistry {
	fn default() -> Self {
		Self::new()
	}
}
