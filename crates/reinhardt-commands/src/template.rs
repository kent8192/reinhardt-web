//! Template utilities for command code generation

use crate::CommandResult;
use crate::{BaseCommand, CommandContext};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TemplateContext {
    pub variables: HashMap<String, String>,
}

impl TemplateContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.variables.insert(key.into(), value.into());
    }
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TemplateCommand;

impl TemplateCommand {
    pub fn new() -> Self {
        Self
    }

    pub fn handle(
        &self,
        _name: &str,
        _target: Option<&std::path::Path>,
        _template_dir: &std::path::Path,
        _context: TemplateContext,
        _ctx: &CommandContext,
    ) -> CommandResult<()> {
        Ok(())
    }
}

impl Default for TemplateCommand {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BaseCommand for TemplateCommand {
    fn name(&self) -> &str {
        "template"
    }

    async fn execute(&self, _ctx: &CommandContext) -> CommandResult<()> {
        Ok(())
    }
}

/// Generate a Django-compatible secret key
pub fn generate_secret_key() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz\
                             ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                             0123456789\
                             !@#$%^&*(-_=+)";
    let mut rng = rand::rng();
    (0..50)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Convert a string to CamelCase
pub fn to_camel_case(s: &str) -> String {
    s.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
