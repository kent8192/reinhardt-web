//! relationship application module
//!
//! A RESTful API application for user relationships (follow/block)

use reinhardt::AppConfig;

pub mod admin;
pub mod serializers;
pub mod urls;
pub mod views;

#[derive(AppConfig)]
#[app_config(name = "relationship", label = "relationship", verbose_name = "User Relationships")]
pub struct RelationshipConfig;
