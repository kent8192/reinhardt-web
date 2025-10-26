//! # Start Commands
//!
//! Django's startproject and startapp commands translation to Rust
//!
//! Source:
//! - django/core/management/commands/startproject.py
//! - django/core/management/commands/startapp.py

use crate::{
    BaseCommand, CommandArgument, CommandContext, CommandError, CommandOption, CommandResult,
    TemplateCommand, TemplateContext, generate_secret_key, to_camel_case,
};
use async_trait::async_trait;
use std::env;
use std::path::{Path, PathBuf};

/// Create a Reinhardt project directory structure
///
/// Translation of Django's startproject command
pub struct StartProjectCommand;

#[async_trait]
impl BaseCommand for StartProjectCommand {
    fn name(&self) -> &str {
        "startproject"
    }

    fn description(&self) -> &str {
        "Creates a Reinhardt project directory structure for the given project name in the current directory or optionally in the given directory."
    }

    fn arguments(&self) -> Vec<CommandArgument> {
        vec![
            CommandArgument::required("name", "Name of the project"),
            CommandArgument::optional("directory", "Optional destination directory"),
        ]
    }

    fn options(&self) -> Vec<CommandOption> {
        vec![
            CommandOption::option(None, "template", "The path to load the template from"),
            CommandOption::option(
                Some('e'),
                "extension",
                "The file extension(s) to render (default: \"rs\")",
            )
            .with_default("rs"),
            CommandOption::flag(
                None,
                "mtv",
                "Create a MTV-style project (Model-Template-View, Django-style)",
            ),
            CommandOption::flag(None, "restful", "Create a RESTful API project (default)"),
        ]
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        let project_name = ctx
            .arg(0)
            .ok_or_else(|| {
                CommandError::InvalidArguments("You must provide a project name.".to_string())
            })?
            .clone();

        let target = ctx.arg(1).map(|s| PathBuf::from(s));

        // Determine project type
        let is_mtv = ctx.has_option("mtv");
        let is_restful = ctx.has_option("restful") || !is_mtv; // RESTful is default

        let project_type = if is_mtv {
            "MTV (Model-Template-View)"
        } else {
            "RESTful API"
        };
        ctx.info(&format!(
            "Creating {} project '{}'...",
            project_type, project_name
        ));

        // Generate a random secret key
        let secret_key = format!("insecure-{}", generate_secret_key());

        // Prepare template context
        let mut context = TemplateContext::new();
        context.insert("project_name", &project_name);
        context.insert("secret_key", &secret_key);
        context.insert("camel_case_project_name", &to_camel_case(&project_name));
        context.insert("reinhardt_version", env!("CARGO_PKG_VERSION"));
        context.insert("is_mtv", if is_mtv { "true" } else { "false" });
        context.insert("is_restful", if is_restful { "true" } else { "false" });

        // Determine template directory
        let template_dir = if let Some(template_path) = ctx.option("template") {
            PathBuf::from(template_path)
        } else {
            // Use built-in template based on project type
            if is_mtv {
                get_project_template_dir("mtv")?
            } else {
                get_project_template_dir("restful")?
            }
        };

        // Create project using TemplateCommand
        let template_cmd = TemplateCommand::new();
        template_cmd.handle(
            &project_name,
            target.as_deref(),
            &template_dir,
            context,
            ctx,
        )?;

        ctx.success(&format!(
            "{} project '{}' created successfully! Next steps:",
            project_type, project_name
        ));
        ctx.info(&format!("  cd {}", project_name));
        ctx.info("  cargo run");

        Ok(())
    }
}

/// Create a Reinhardt app directory structure
///
/// Translation of Django's startapp command
pub struct StartAppCommand;

#[async_trait]
impl BaseCommand for StartAppCommand {
    fn name(&self) -> &str {
        "startapp"
    }

    fn description(&self) -> &str {
        "Creates a Reinhardt app directory structure for the given app name in the current directory or optionally in the given directory."
    }

    fn arguments(&self) -> Vec<CommandArgument> {
        vec![
            CommandArgument::required("name", "Name of the application"),
            CommandArgument::optional("directory", "Optional destination directory"),
        ]
    }

    fn options(&self) -> Vec<CommandOption> {
        vec![
            CommandOption::option(None, "template", "The path to load the template from"),
            CommandOption::option(
                Some('e'),
                "extension",
                "The file extension(s) to render (default: \"rs\")",
            )
            .with_default("rs"),
            CommandOption::flag(
                None,
                "mtv",
                "Create a MTV-style app (Model-Template-View, Django-style)",
            ),
            CommandOption::flag(None, "restful", "Create a RESTful API app (default)"),
            CommandOption::flag(
                None,
                "workspace",
                "Create app as a separate workspace crate instead of a module",
            ),
        ]
    }

    async fn execute(&self, ctx: &CommandContext) -> CommandResult<()> {
        let app_name = ctx
            .arg(0)
            .ok_or_else(|| {
                CommandError::InvalidArguments("You must provide an application name.".to_string())
            })?
            .clone();

        let target = ctx.arg(1).map(|s| PathBuf::from(s));

        // Determine app type and structure
        let is_mtv = ctx.has_option("mtv");
        let is_restful = ctx.has_option("restful") || !is_mtv; // RESTful is default
        let is_workspace = ctx.has_option("workspace");

        let app_type = if is_mtv {
            "MTV (Model-Template-View)"
        } else {
            "RESTful API"
        };
        let structure_type = if is_workspace {
            "workspace crate"
        } else {
            "module"
        };
        ctx.info(&format!(
            "Creating {} app '{}' as a {}...",
            app_type, app_name, structure_type
        ));

        if is_workspace {
            // Create as workspace crate
            create_workspace_app(&app_name, target.as_deref(), is_mtv, ctx).await?;

            ctx.success(&format!(
                "{} app '{}' created successfully as a workspace crate in apps/{}!",
                app_type, app_name, app_name
            ));
            ctx.info("The app has been added to the workspace members in Cargo.toml");
            ctx.info(
                "Don't forget to add it as a dependency and to INSTALLED_APPS in your settings.rs",
            );
        } else {
            // Create as module (default)
            // Create apps directory if it doesn't exist
            let apps_dir = PathBuf::from("apps");
            if !apps_dir.exists() {
                std::fs::create_dir_all(&apps_dir).map_err(|e| {
                    CommandError::ExecutionError(format!("Failed to create apps directory: {}", e))
                })?;
                ctx.verbose("Created apps/ directory");
            }

            // Set target to apps/{app_name} if no custom target is specified
            let app_target = if target.is_some() {
                target
            } else {
                Some(apps_dir.join(&app_name))
            };

            // Prepare template context
            let mut context = TemplateContext::new();
            context.insert("app_name", &app_name);
            context.insert("camel_case_app_name", &to_camel_case(&app_name));
            context.insert("is_mtv", if is_mtv { "true" } else { "false" });
            context.insert("is_restful", if is_restful { "true" } else { "false" });

            // Determine template directory
            let template_dir = if let Some(template_path) = ctx.option("template") {
                PathBuf::from(template_path)
            } else {
                // Use built-in template based on app type
                if is_mtv {
                    get_app_template_dir("mtv")?
                } else {
                    get_app_template_dir("restful")?
                }
            };

            // Create app using TemplateCommand
            let template_cmd = TemplateCommand::new();
            template_cmd.handle(
                &app_name,
                app_target.as_deref(),
                &template_dir,
                context,
                ctx,
            )?;

            // Update or create apps.rs to export the new app
            update_apps_export(&app_name)?;

            ctx.success(&format!(
                "{} app '{}' created successfully in apps/{}!",
                app_type, app_name, app_name
            ));
            ctx.info("The app has been added to apps.rs");
            ctx.info("Don't forget to add it to INSTALLED_APPS in your settings.rs");
        }

        Ok(())
    }
}

/// Get the path to the built-in project template directory
fn get_project_template_dir(template_type: &str) -> CommandResult<PathBuf> {
    // template_type: "mvc" or "restful"
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let template_dir = PathBuf::from(manifest_dir)
        .join("templates")
        .join(format!("project_{}_template", template_type));

    if !template_dir.exists() {
        return Err(CommandError::ExecutionError(format!(
            "Project template directory not found at {}. Falling back to default template.",
            template_dir.display()
        )));
    }

    Ok(template_dir)
}

/// Get the path to the built-in app template directory
fn get_app_template_dir(template_type: &str) -> CommandResult<PathBuf> {
    // template_type: "mvc" or "restful"
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let template_dir = PathBuf::from(manifest_dir)
        .join("templates")
        .join(format!("app_{}_template", template_type));

    if !template_dir.exists() {
        return Err(CommandError::ExecutionError(format!(
            "App template directory not found at {}. Falling back to default template.",
            template_dir.display()
        )));
    }

    Ok(template_dir)
}

/// Create a workspace-based app
async fn create_workspace_app(
    app_name: &str,
    target: Option<&Path>,
    is_mtv: bool,
    ctx: &CommandContext,
) -> CommandResult<()> {
    // Create apps directory if it doesn't exist
    let apps_dir = PathBuf::from("apps");
    if !apps_dir.exists() {
        std::fs::create_dir_all(&apps_dir).map_err(|e| {
            CommandError::ExecutionError(format!("Failed to create apps directory: {}", e))
        })?;
        ctx.verbose("Created apps/ directory");
    }

    // Set target to apps/{app_name} if no custom target is specified
    let app_target = if let Some(t) = target {
        t.to_path_buf()
    } else {
        apps_dir.join(app_name)
    };

    // Prepare template context
    let mut context = TemplateContext::new();
    context.insert("app_name", app_name);
    context.insert("camel_case_app_name", &to_camel_case(app_name));
    context.insert("is_mtv", if is_mtv { "true" } else { "false" });
    context.insert("is_restful", if !is_mtv { "true" } else { "false" });

    // Determine template directory for workspace apps
    let template_dir = if is_mtv {
        get_app_workspace_template_dir("mtv")?
    } else {
        get_app_workspace_template_dir("restful")?
    };

    // Create app using TemplateCommand
    let template_cmd = TemplateCommand::new();
    template_cmd.handle(app_name, Some(&app_target), &template_dir, context, ctx)?;

    // Update workspace Cargo.toml
    update_workspace_members(app_name)?;

    Ok(())
}

/// Get the path to the built-in workspace app template directory
fn get_app_workspace_template_dir(template_type: &str) -> CommandResult<PathBuf> {
    // template_type: "mvc" or "restful"
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let template_dir = PathBuf::from(manifest_dir)
        .join("templates")
        .join(format!("app_{}_workspace_template", template_type));

    if !template_dir.exists() {
        return Err(CommandError::ExecutionError(format!(
            "Workspace app template directory not found at {}.",
            template_dir.display()
        )));
    }

    Ok(template_dir)
}

/// Update workspace Cargo.toml to add new app as a member
fn update_workspace_members(app_name: &str) -> CommandResult<()> {
    use std::fs;

    let cargo_toml_path = PathBuf::from("Cargo.toml");

    if !cargo_toml_path.exists() {
        return Err(CommandError::ExecutionError(
            "Cargo.toml not found in current directory. Make sure you're in the project root."
                .to_string(),
        ));
    }

    let content = fs::read_to_string(&cargo_toml_path)
        .map_err(|e| CommandError::ExecutionError(format!("Failed to read Cargo.toml: {}", e)))?;

    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let member_line = format!("    \"apps/{}\",", app_name);

    // Find [workspace] section and members array
    let mut in_workspace_section = false;
    let mut in_members_array = false;
    let mut insert_index = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed == "[workspace]" {
            in_workspace_section = true;
            continue;
        }

        if in_workspace_section {
            if trimmed.starts_with('[') && trimmed != "[workspace]" {
                // Entered a different section
                break;
            }

            if trimmed.starts_with("members") {
                in_members_array = true;
                continue;
            }

            if in_members_array {
                if trimmed == "]" {
                    // Found end of members array, insert before this line
                    insert_index = Some(i);
                    break;
                }
            }
        }
    }

    if let Some(idx) = insert_index {
        // Check if member already exists
        let member_exists = lines
            .iter()
            .any(|line| line.contains(&format!("apps/{}", app_name)));

        if !member_exists {
            lines.insert(idx, member_line);
        }
    } else {
        // No workspace section found, add it
        return Err(CommandError::ExecutionError(
            "No [workspace] section with members array found in Cargo.toml. Please add one manually or use a workspace template.".to_string()
        ));
    }

    // Write back
    let new_content = lines.join("\n") + "\n";
    fs::write(&cargo_toml_path, new_content)
        .map_err(|e| CommandError::ExecutionError(format!("Failed to write Cargo.toml: {}", e)))?;

    Ok(())
}

/// Update or create apps.rs to export the new app
fn update_apps_export(app_name: &str) -> CommandResult<()> {
    use std::fs;

    let apps_file = PathBuf::from("src/apps.rs");
    let camel_case_name = to_camel_case(app_name);

    // Read existing content if file exists
    let mut lines = if apps_file.exists() {
        fs::read_to_string(&apps_file)
            .map_err(|e| CommandError::ExecutionError(format!("Failed to read apps.rs: {}", e)))?
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    } else {
        vec![
            "//! Apps module - exports all applications".to_string(),
            String::new(),
        ]
    };

    // Check if this app is already exported
    let pub_mod_line = format!("pub mod {};", app_name);
    let pub_use_line = format!("pub use {}::{}Config;", app_name, camel_case_name);

    if !lines.iter().any(|line| line.contains(&pub_mod_line)) {
        // Add pub mod declaration
        lines.push(pub_mod_line);
        // Add pub use declaration
        lines.push(pub_use_line);
    }

    // Write back to file
    let content = lines.join("\n") + "\n";
    fs::write(&apps_file, content)
        .map_err(|e| CommandError::ExecutionError(format!("Failed to write apps.rs: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startproject_command_name() {
        let cmd = StartProjectCommand;
        assert_eq!(cmd.name(), "startproject");
    }

    #[test]
    fn test_startapp_command_name() {
        let cmd = StartAppCommand;
        assert_eq!(cmd.name(), "startapp");
    }
}
