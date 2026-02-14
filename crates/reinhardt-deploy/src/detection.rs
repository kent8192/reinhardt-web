pub mod code_analysis;
pub mod feature_flags;

pub use code_analysis::analyze_code;
pub use feature_flags::{FeatureDetectionResult, analyze_feature_flags};
