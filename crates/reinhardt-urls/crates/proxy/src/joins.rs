//! Join operations for proxy relationships

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinConfig {
    pub eager_load: bool,
    pub max_depth: Option<usize>,
}

impl JoinConfig {
    pub fn new() -> Self {
        Self {
            eager_load: false,
            max_depth: None,
        }
    }
}

impl Default for JoinConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadingStrategy {
    Eager,
    Lazy,
    Select,
}

#[derive(Debug, Clone)]
pub struct NestedProxy {
    pub path: Vec<String>,
}

impl NestedProxy {
    pub fn new(path: Vec<String>) -> Self {
        Self { path }
    }
}

#[derive(Debug, Clone)]
pub struct RelationshipPath {
    pub segments: Vec<String>,
}

impl RelationshipPath {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }
}

pub fn extract_through_path(path: &str) -> Vec<String> {
    path.split('.').map(|s| s.to_string()).collect()
}

pub fn filter_through_path(path: &RelationshipPath, predicate: impl Fn(&str) -> bool) -> bool {
    path.segments.iter().any(|s| predicate(s))
}

pub fn traverse_and_extract(proxy: &NestedProxy) -> Vec<String> {
    proxy.path.clone()
}

pub fn traverse_relationships(path: &RelationshipPath) -> Vec<String> {
    path.segments.clone()
}
