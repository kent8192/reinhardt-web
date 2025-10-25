//! Metadata configuration options

/// Options for configuring metadata
#[derive(Debug, Clone)]
pub struct MetadataOptions {
    pub name: String,
    pub description: String,
    pub allowed_methods: Vec<String>,
    pub renders: Vec<String>,
    pub parses: Vec<String>,
}

impl Default for MetadataOptions {
    fn default() -> Self {
        Self {
            name: "API View".to_string(),
            description: "API endpoint".to_string(),
            allowed_methods: vec!["GET".to_string()],
            renders: vec!["application/json".to_string()],
            parses: vec!["application/json".to_string()],
        }
    }
}
