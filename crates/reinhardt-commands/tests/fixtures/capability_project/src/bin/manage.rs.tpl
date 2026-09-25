use async_trait::async_trait;
use capability_consumer::{ProjectSettings, settings_builder};
use reinhardt::commands::{
    CapabilityCommand, CapabilityContext, CapabilityProvider, CapabilityRequirement,
    CommandRegistry, CommandResult, execute_from_command_line_with_capabilities,
};
use reinhardt::conf::settings::PendingSettings;
use reinhardt::conf::settings::builder::BuildError;
use reinhardt::conf::settings::scoped::ScopedSettings;

struct CloudProvider;

#[async_trait]
impl CapabilityProvider for CloudProvider {
    type Settings = ProjectSettings;

    fn scoped_settings(&self) -> Result<ScopedSettings, BuildError> {
        settings_builder().build_scoped()
    }

    fn full_settings(&self) -> Result<PendingSettings<ProjectSettings>, BuildError> {
        settings_builder().build_pending_composed::<ProjectSettings>()
    }

    async fn prepare_service(
        &self,
        _requirement: &CapabilityRequirement,
        _context: &CapabilityContext,
    ) -> CommandResult<reinhardt::commands::capabilities::PreparedValue> {
        panic!("unrelated Cloud runtime provider was initialized")
    }
}

struct RuntimeService;
struct CloudJob;

#[async_trait]
impl CapabilityCommand for CloudJob {
    fn cli(&self) -> clap::Command {
        clap::Command::new("cloud-job")
    }

    fn requirements(&self, _matches: &clap::ArgMatches) -> Vec<CapabilityRequirement> {
        vec![CapabilityRequirement::service::<RuntimeService>("Cloud runtime", None)]
    }

    async fn execute(&self, _matches: &clap::ArgMatches, _context: &CapabilityContext) -> CommandResult<()> {
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let mut registry = CommandRegistry::new();
    registry.register_capability(Box::new(CloudJob));
    if let Err(error) = execute_from_command_line_with_capabilities(registry, CloudProvider, None).await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
