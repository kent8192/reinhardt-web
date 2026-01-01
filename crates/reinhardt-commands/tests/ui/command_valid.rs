//! Valid command definition test case
//!
//! This file should compile successfully.

use reinhardt_commands::{BaseCommand, CommandContext, CommandArgument, CommandOption, CommandError};
use async_trait::async_trait;

/// A valid custom command implementation
pub struct ValidCommand;

#[async_trait]
impl BaseCommand for ValidCommand {
	fn name(&self) -> &str {
		"valid"
	}

	fn description(&self) -> &str {
		"A valid command for testing"
	}

	fn arguments(&self) -> Vec<CommandArgument> {
		vec![
			CommandArgument::required("input", "Input file path"),
		]
	}

	fn options(&self) -> Vec<CommandOption> {
		vec![
			CommandOption::flag(Some('v'), "verbose", "Enable verbose output"),
		]
	}

	async fn execute(&self, ctx: &CommandContext) -> Result<(), CommandError> {
		let _input = ctx.arg(0);
		let _verbose = ctx.has_option("verbose");
		Ok(())
	}
}

fn main() {
	let cmd = ValidCommand;
	assert_eq!(cmd.name(), "valid");
	assert!(!cmd.description().is_empty());
}
