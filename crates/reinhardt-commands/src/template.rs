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
        name: &str,
        target: Option<&std::path::Path>,
        template_dir: &std::path::Path,
        context: TemplateContext,
        ctx: &CommandContext,
    ) -> CommandResult<()> {
        use crate::CommandError;
        use std::fs;

        // Validate template directory exists
        if !template_dir.exists() {
            return Err(CommandError::ExecutionError(format!(
                "Template directory does not exist: {}",
                template_dir.display()
            )));
        }

        // Determine output directory
        let output_dir = if let Some(t) = target {
            t.to_path_buf()
        } else {
            std::path::PathBuf::from(name)
        };

        // Create output directory
        if output_dir.exists() {
            ctx.verbose(&format!(
                "Directory '{}' already exists, will write into it",
                output_dir.display()
            ));
        } else {
            fs::create_dir_all(&output_dir).map_err(|e| {
                CommandError::ExecutionError(format!(
                    "Failed to create output directory '{}': {}",
                    output_dir.display(),
                    e
                ))
            })?;
        }

        // Process all files in template directory recursively
        self.process_directory(template_dir, &output_dir, template_dir, &context, ctx)?;

        Ok(())
    }

    fn process_directory(
        &self,
        current_dir: &std::path::Path,
        output_base: &std::path::Path,
        template_base: &std::path::Path,
        context: &TemplateContext,
        ctx: &CommandContext,
    ) -> CommandResult<()> {
        use crate::CommandError;
        use std::fs;

        let entries = fs::read_dir(current_dir).map_err(|e| {
            CommandError::ExecutionError(format!(
                "Failed to read template directory '{}': {}",
                current_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                CommandError::ExecutionError(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // Skip hidden files and __pycache__
            if file_name_str.starts_with('.') || file_name_str == "__pycache__" {
                continue;
            }

            // Calculate relative path from template base
            let relative_path = path.strip_prefix(template_base).map_err(|e| {
                CommandError::ExecutionError(format!("Failed to compute relative path: {}", e))
            })?;

            if path.is_dir() {
                // Create corresponding directory in output
                let output_dir = output_base.join(relative_path);
                fs::create_dir_all(&output_dir).map_err(|e| {
                    CommandError::ExecutionError(format!(
                        "Failed to create directory '{}': {}",
                        output_dir.display(),
                        e
                    ))
                })?;

                // Recursively process subdirectory
                self.process_directory(&path, output_base, template_base, context, ctx)?;
            } else {
                // Process file
                self.process_file(&path, output_base, template_base, context, ctx)?;
            }
        }

        Ok(())
    }

    fn process_file(
        &self,
        template_file: &std::path::Path,
        output_base: &std::path::Path,
        template_base: &std::path::Path,
        context: &TemplateContext,
        ctx: &CommandContext,
    ) -> CommandResult<()> {
        use crate::CommandError;
        use std::fs;
        use std::io::Write;

        // Calculate relative path from template base
        let relative_path = template_file.strip_prefix(template_base).map_err(|e| {
            CommandError::ExecutionError(format!("Failed to compute relative path: {}", e))
        })?;

        // Determine output file name (remove .tpl extension if present)
        let output_path = if let Some(stem) = relative_path.to_str() {
            if stem.ends_with(".tpl") {
                output_base.join(&stem[..stem.len() - 4])
            } else {
                output_base.join(relative_path)
            }
        } else {
            output_base.join(relative_path)
        };

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                CommandError::ExecutionError(format!(
                    "Failed to create parent directory for '{}': {}",
                    output_path.display(),
                    e
                ))
            })?;
        }

        // Read template content
        let template_content = fs::read_to_string(template_file).map_err(|e| {
            CommandError::ExecutionError(format!(
                "Failed to read template file '{}': {}",
                template_file.display(),
                e
            ))
        })?;

        // Replace template variables
        let rendered_content = self.render_template(&template_content, context);

        // Write to output file
        let mut output_file = fs::File::create(&output_path).map_err(|e| {
            CommandError::ExecutionError(format!(
                "Failed to create output file '{}': {}",
                output_path.display(),
                e
            ))
        })?;

        output_file
            .write_all(rendered_content.as_bytes())
            .map_err(|e| {
                CommandError::ExecutionError(format!(
                    "Failed to write to output file '{}': {}",
                    output_path.display(),
                    e
                ))
            })?;

        ctx.verbose(&format!(
            "Created: {}",
            output_path
                .strip_prefix(output_base)
                .unwrap_or(&output_path)
                .display()
        ));

        Ok(())
    }

    fn render_template(&self, template: &str, context: &TemplateContext) -> String {
        let mut result = template.to_string();

        // Replace all {{ variable_name }} with values from context
        for (key, value) in &context.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        result
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

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        use crate::CommandError;

        let name = ctx
            .arg(0)
            .ok_or_else(|| CommandError::InvalidArguments("You must provide a name.".to_string()))?
            .clone();

        let target = ctx.arg(1).map(std::path::PathBuf::from);

        let template_dir = ctx.option("template").ok_or_else(|| {
            CommandError::InvalidArguments(
                "You must provide a template directory via --template.".to_string(),
            )
        })?;

        let template_path = std::path::PathBuf::from(template_dir);

        let context = TemplateContext::new();

        self.handle(&name, target.as_deref(), &template_path, context, ctx)?;

        ctx.success("Template processed successfully");

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
