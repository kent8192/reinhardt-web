#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn generated_builders_do_not_require_downstream_bon_dependency() {
	let crate_dir = TempDir::new().expect("create downstream fixture");
	let target_dir = TempDir::new().expect("create downstream target dir");
	let reinhardt_pages_dir = env!("CARGO_MANIFEST_DIR").replace('\\', "\\\\");

	fs::write(
		crate_dir.path().join("Cargo.toml"),
		format!(
			r#"[package]
name = "downstream-builder-reexport-fixture"
version = "0.0.0"
edition = "2024"

[dependencies]
reinhardt-pages = {{ path = "{reinhardt_pages_dir}" }}
"#
		),
	)
	.expect("write downstream manifest");
	fs::create_dir(crate_dir.path().join("src")).expect("create downstream src dir");
	fs::write(
		crate_dir.path().join("src/main.rs"),
		r#"use reinhardt_pages::router::request::{FromRequest, RouteContext};
use reinhardt_pages::{Page, Path, Query, component, page, page_props};

#[page_props]
struct SearchPageProps {
	#[from_request(path)]
	id: i64,
	#[from_request(query)]
	tab: String,
}

#[component("/users/{id}/", "user-detail")]
fn user_page(Path(id): Path<i64>, Query(tab): Query<String>) -> Page {
	page!(|id: i64, tab: String| {
		div { {
			format!("{id}:{tab}")
		} }
	})(id, tab)
}

fn main() {
	let _ = SearchPageProps::builder()
		.id(7)
		.tab("profile".to_string())
		.build();
	let _extractor: fn(&RouteContext) -> Result<SearchPageProps, _> =
		SearchPageProps::from_request;
	let _ = UserPageProps::builder()
		.id(7)
		.tab("profile".to_string())
		.build();
}
"#,
	)
	.expect("write downstream source");

	let output = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
		.arg("check")
		.arg("--manifest-path")
		.arg(crate_dir.path().join("Cargo.toml"))
		.arg("--target-dir")
		.arg(target_dir.path())
		.arg("--offline")
		.output()
		.expect("run downstream cargo check");

	assert!(
		output.status.success(),
		"downstream fixture should compile without a direct bon dependency\nstdout:\n{}\nstderr:\n{}",
		String::from_utf8_lossy(&output.stdout),
		String::from_utf8_lossy(&output.stderr)
	);
}
