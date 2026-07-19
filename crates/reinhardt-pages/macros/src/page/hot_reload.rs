//! Development-only page descriptor and dynamic slot code generation.

use proc_macro2::TokenStream;
use quote::quote;

use reinhardt_manouche::core::TypedPageMacro;
use reinhardt_manouche::hot_reload::{
	CompilerDynamicSlotSignature, CompilerSlotKind, CompilerStaticTemplateNode,
	ManoucheHotReloadTemplate,
};

/// Generates the target-neutral descriptor attached to one compiled page.
pub(super) fn generate_template_descriptor(
	macro_ast: &TypedPageMacro,
	pages_crate: &TokenStream,
) -> TokenStream {
	let template = match reinhardt_manouche::hot_reload::lower_page_macro(macro_ast) {
		Ok(template) => template,
		Err(error) => {
			let message = error.to_string();
			return quote! { compile_error!(#message); };
		}
	};
	let mut next_nested_index = 1;
	generate_template(
		&template,
		pages_crate,
		quote!(0_u32),
		&mut next_nested_index,
	)
}

/// Wraps a generated dynamic page subtree in its development slot marker.
pub(super) fn wrap_dynamic_slot(
	slot_id: u32,
	view: TokenStream,
	_pages_crate: &TokenStream,
) -> TokenStream {
	quote! { (#view).with_dev_slot(#slot_id) }
}

pub(super) fn wrap_dynamic_slots(
	view: TokenStream,
	slot_ids: &[u32],
	pages_crate: &TokenStream,
) -> TokenStream {
	// A dynamic element or keyed loop can contribute more than one ABI entry,
	// but a single outer range owns the live DOM. The final preorder slot is
	// the outer marker emitted by the lowering and keeps every inner binding
	// and its RAII owner together during a static patch.
	slot_ids.last().map_or(view.clone(), |slot_id| {
		wrap_dynamic_slot(*slot_id, view, pages_crate)
	})
}

fn generate_template(
	template: &ManoucheHotReloadTemplate,
	pages_crate: &TokenStream,
	nested_template_index: TokenStream,
	next_nested_index: &mut u32,
) -> TokenStream {
	let nested = template
		.nested
		.iter()
		.map(|nested_template| {
			let index = *next_nested_index;
			*next_nested_index = next_nested_index.saturating_add(1);
			generate_template(
				nested_template,
				pages_crate,
				quote!(#index),
				next_nested_index,
			)
		})
		.collect::<Vec<_>>();
	let slots = template
		.slots
		.iter()
		.map(|slot| generate_slot(slot, pages_crate))
		.collect::<Vec<_>>();
	let static_tree = generate_static_tree(&template.static_tree, pages_crate);
	let abi_bytes = template
		.abi_hash
		.0
		.iter()
		.map(|byte| quote!(#byte))
		.collect::<Vec<_>>();
	quote! {
		#pages_crate::hmr::TemplateDescriptor {
			key: #pages_crate::hmr::TemplateKey {
				source_id: #pages_crate::hmr::SourceId(file!().to_owned()),
				line: line!(),
				column: column!(),
				nested_template_index: #nested_template_index,
			},
			abi_hash: #pages_crate::hmr::DynamicAbiHash([#(#abi_bytes),*]),
			static_tree: #static_tree,
			slots: vec![#(#slots),*],
			nested: vec![#(#nested),*],
		}
	}
}

fn generate_slot(slot: &CompilerDynamicSlotSignature, pages_crate: &TokenStream) -> TokenStream {
	let slot_id = slot.slot_id.0;
	let semantic_kind = semantic_kind_name(&slot.semantic_kind);
	let canonical_tokens = slot
		.canonical_tokens
		.iter()
		.map(|token| quote!(#token.to_owned()))
		.collect::<Vec<_>>();
	quote! {
		#pages_crate::hmr::DynamicSlotDescriptor {
			slot_id: #pages_crate::hmr::DynamicSlotId(#slot_id),
			semantic_kind: #semantic_kind.to_owned(),
			canonical_tokens: vec![#(#canonical_tokens),*],
		}
	}
}

fn generate_static_tree(
	node: &CompilerStaticTemplateNode,
	pages_crate: &TokenStream,
) -> TokenStream {
	match node {
		CompilerStaticTemplateNode::Element {
			tag,
			static_attrs,
			children,
		} => {
			let attrs = static_attrs
				.iter()
				.map(|(name, value)| quote!((#name.to_owned(), #value.to_owned())));
			let children = children
				.iter()
				.map(|child| generate_static_tree(child, pages_crate));
			quote! {
				#pages_crate::hmr::StaticTemplateNode::Element {
					tag: #tag.to_owned(),
					static_attrs: vec![#(#attrs),*],
					children: vec![#(#children),*],
				}
			}
		}
		CompilerStaticTemplateNode::Text(text) => {
			quote! { #pages_crate::hmr::StaticTemplateNode::Text(#text.to_owned()) }
		}
		CompilerStaticTemplateNode::Slot(slot_id) => {
			let slot_id = slot_id.0;
			quote! { #pages_crate::hmr::StaticTemplateNode::Slot(#pages_crate::hmr::DynamicSlotId(#slot_id)) }
		}
	}
}

fn semantic_kind_name(kind: &CompilerSlotKind) -> String {
	match kind {
		CompilerSlotKind::Expression => "expression".to_owned(),
		CompilerSlotKind::DynamicAttribute { name } => format!("dynamic_attribute:{name}"),
		CompilerSlotKind::Event { name } => format!("event:{name}"),
		CompilerSlotKind::IfCondition => "if_condition".to_owned(),
		CompilerSlotKind::ForIteration => "for_iteration".to_owned(),
		CompilerSlotKind::ForKey => "for_key".to_owned(),
		CompilerSlotKind::ComponentInvocation => "component_invocation".to_owned(),
		CompilerSlotKind::HeadExpression => "head_expression".to_owned(),
	}
}
