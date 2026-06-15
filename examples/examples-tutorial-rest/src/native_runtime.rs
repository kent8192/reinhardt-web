//! Target-specific runtime helpers for the REST tutorial example.

#[cfg(not(target_arch = "wasm32"))]
pub mod models {
	pub use crate::apps::snippets::models::*;
}

#[cfg(not(target_arch = "wasm32"))]
pub mod serializers {
	pub use crate::apps::snippets::serializers::*;
}

#[cfg(not(target_arch = "wasm32"))]
#[path = "apps/snippets/di.rs"]
pub mod di;

#[cfg(not(target_arch = "wasm32"))]
#[path = "apps/snippets/views.rs"]
mod views;

#[cfg(not(target_arch = "wasm32"))]
pub fn highlighted_code(language: &str, code: &str) -> String {
	use syntect::highlighting::ThemeSet;
	use syntect::html::highlighted_html_for_string;
	use syntect::parsing::SyntaxSet;

	let ss = SyntaxSet::load_defaults_newlines();
	let ts = ThemeSet::load_defaults();

	let syntax = ss
		.find_syntax_by_name(language)
		.or_else(|| ss.find_syntax_by_extension(language))
		.unwrap_or_else(|| ss.find_syntax_plain_text());

	let theme = &ts.themes["base16-ocean.dark"];

	highlighted_html_for_string(code, &ss, syntax, theme).unwrap_or_else(|_| code.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn highlighted_code(_language: &str, code: &str) -> String {
	code.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn snippets_url_patterns() -> reinhardt::ServerRouter {
	reinhardt::ServerRouter::new()
		// Function-based endpoints (Tutorial 1-5)
		// - GET    /snippets/        - views::list
		// - POST   /snippets/        - views::create
		// - GET    /snippets/config/ - views::config (registered before
		//   the `{id}` route below so this literal path is matched first)
		// - GET    /snippets/{id}/   - views::retrieve
		// - PUT    /snippets/{id}/   - views::update
		// - DELETE /snippets/{id}/   - views::delete
		.endpoint(views::list)
		.endpoint(views::create)
		.endpoint(views::config)
		.endpoint(views::retrieve)
		.endpoint(views::update)
		.endpoint(views::delete)
		// ViewSet endpoints (Tutorial 6, rc.23+ real CRUD)
		// - GET    /snippets-viewset/         - list (pagination/filter/order)
		// - POST   /snippets-viewset/         - create
		// - GET    /snippets-viewset/{id}/    - retrieve
		// - PUT    /snippets-viewset/{id}/    - update
		// - PATCH  /snippets-viewset/{id}/    - partial update
		// - DELETE /snippets-viewset/{id}/    - delete
		.viewset("/snippets-viewset", views::viewset())
}

#[cfg(target_arch = "wasm32")]
pub fn snippets_url_patterns() -> reinhardt::ServerRouter {
	reinhardt::ServerRouter::new()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn mount_api_routes(router: reinhardt::UnifiedRouter) -> reinhardt::UnifiedRouter {
	router.mount("/api/", crate::apps::snippets::urls::url_patterns())
}

#[cfg(target_arch = "wasm32")]
pub fn mount_api_routes(router: reinhardt::UnifiedRouter) -> reinhardt::UnifiedRouter {
	router
}

#[cfg(not(target_arch = "wasm32"))]
use reinhardt::conf::settings::builder::SettingsBuilder;
#[cfg(not(target_arch = "wasm32"))]
use reinhardt::conf::settings::profile::Profile;
#[cfg(not(target_arch = "wasm32"))]
use reinhardt::conf::settings::sources::{DefaultSource, LowPriorityEnvSource, TomlFileSource};
#[cfg(not(target_arch = "wasm32"))]
use reinhardt::core::serde::json;
#[cfg(not(target_arch = "wasm32"))]
use reinhardt::settings;
#[cfg(not(target_arch = "wasm32"))]
use std::env;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
#[settings(core: CoreSettings | contacts: ContactSettings)]
pub struct ProjectSettings;

#[cfg(target_arch = "wasm32")]
pub struct ProjectSettings;

#[cfg(not(target_arch = "wasm32"))]
fn profile_name() -> String {
	env::var("REINHARDT_ENV").unwrap_or_else(|_| {
		if env::var("CI").is_ok() {
			"ci".to_string()
		} else {
			"local".to_string()
		}
	})
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_settings_dir() -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("settings")
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get_settings() -> ProjectSettings {
	let profile_str = profile_name();
	let settings_dir = resolve_settings_dir();
	let base_dir = env::current_dir().expect("Failed to get current directory");

	SettingsBuilder::new()
		.profile(Profile::parse(&profile_str))
		.add_source(DefaultSource::new().with_value(
			"core.base_dir",
			json::Value::String(base_dir.to_string_lossy().to_string()),
		))
		.add_source(LowPriorityEnvSource::new().with_prefix("REINHARDT_"))
		.add_source(TomlFileSource::new(settings_dir.join("base.toml")))
		.add_source(TomlFileSource::new(
			settings_dir.join(format!("{}.toml", profile_str)),
		))
		.build_composed()
		.expect("Failed to build settings")
}

#[cfg(target_arch = "wasm32")]
pub fn get_settings() -> ProjectSettings {
	ProjectSettings
}
