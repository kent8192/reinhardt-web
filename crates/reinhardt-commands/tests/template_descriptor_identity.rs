#![cfg(feature = "pages")]

use std::path::Path;

use reinhardt_commands::template_manifest::{collect_template_source, source_id_for_path};
use reinhardt_pages::{component::Page, hmr::TemplateDescriptor, page};

fn compiled_template() -> Page {
	page!(|| {
		section {
			class: "template-identity",
			"Compiled descriptor identity"
		}
	})()
}

#[test]
fn compiled_descriptor_key_matches_the_collected_source_callsite() {
	// Arrange
	let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
		.ancestors()
		.nth(2)
		.expect("commands crate has a workspace root");
	let source_path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("tests")
		.join("template_descriptor_identity.rs");
	let source_id = source_id_for_path(workspace_root, &source_path);
	let source = include_str!("template_descriptor_identity.rs");
	let collected = collect_template_source(&source_id, source).expect("collect test source");
	let compiled = compiled_template();
	let (metadata, _) = compiled
		.into_dev_template_parts()
		.expect("page macro emits development metadata");
	let runtime = metadata
		.downcast_ref::<TemplateDescriptor>()
		.expect("page macro stores a template descriptor");

	// Act
	let collected_descriptor = collected
		.templates
		.iter()
		.find(|template| template.descriptor.static_tree == runtime.static_tree)
		.expect("collector finds the compiled template");

	// Assert
	assert_eq!(runtime.key, collected_descriptor.key);
	assert_eq!(runtime.abi_hash, collected_descriptor.descriptor.abi_hash);
}
