//! relationship application module
//!
//! User relationship models for examples-twitter

use reinhardt::AppConfig;

#[derive(AppConfig)]
#[app_config(
	name = "relationship",
	label = "relationship",
	verbose_name = "User Relationships"
)]
pub struct RelationshipConfig;
