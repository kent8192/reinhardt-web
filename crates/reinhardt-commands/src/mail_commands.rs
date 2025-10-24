//! Mail related commands

use crate::CommandResult;
use crate::{BaseCommand, CommandContext};
use async_trait::async_trait;

pub struct SendTestEmailCommand;

impl SendTestEmailCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SendTestEmailCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseCommand for SendTestEmailCommand {
    fn name(&self) -> &str {
        "sendtestemail"
    }

    async fn execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
        Ok(())
    }
}
