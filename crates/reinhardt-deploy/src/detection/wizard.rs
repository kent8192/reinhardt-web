use crate::detection::feature_flags::FeatureDetectionResult;
use crate::error::DeployResult;

/// Present detection results to the user for interactive confirmation.
///
/// When the unified detection pipeline produces ambiguous results (e.g., the
/// "full" feature flag is used and code analysis cannot determine exact usage),
/// this wizard allows the user to confirm or override each detected feature.
///
/// This is a stub implementation. The actual `dialoguer`-based interactive
/// prompts will be added when interactive deployment mode is enabled.
pub fn present_detection_results(_result: &mut FeatureDetectionResult) -> DeployResult<()> {
	unimplemented!("interactive wizard requires dialoguer integration")
}
