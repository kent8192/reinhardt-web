//! Mobile code generation visitor.
//!
//! Implements IRVisitor to generate mobile-specific code from reinhardt-manouche IR.

use proc_macro2::TokenStream;
use quote::quote;
use reinhardt_manouche::codegen::IRVisitor;
use reinhardt_manouche::ir::{
	AttributeIR, ComponentCallIR, ComponentIR, ConditionalIR, ElementIR, EventIR, ExprIR, FieldIR,
	FormIR, HeadElementIR, HeadIR, LoopIR, NodeIR, PropIR, TextIR, WatchIR,
};

use crate::MobileConfig;

/// Mobile code generation visitor.
///
/// Generates mobile-specific code from reinhardt-manouche IR.
pub struct MobileVisitor {
	/// Configuration for mobile generation
	#[allow(dead_code)] // Will be used in future implementation
	config: MobileConfig,
	/// Generated HTML content
	html_buffer: String,
	/// Generated JS content
	js_buffer: String,
}

impl MobileVisitor {
	/// Creates a new MobileVisitor with the given configuration.
	pub fn new(config: MobileConfig) -> Self {
		Self {
			config,
			html_buffer: String::new(),
			js_buffer: String::new(),
		}
	}

	/// Returns the generated HTML content.
	pub fn html(&self) -> &str {
		&self.html_buffer
	}

	/// Returns the generated JavaScript content.
	pub fn js(&self) -> &str {
		&self.js_buffer
	}

	/// Resets the visitor buffers.
	pub fn reset(&mut self) {
		self.html_buffer.clear();
		self.js_buffer.clear();
	}
}

impl IRVisitor for MobileVisitor {
	type Output = TokenStream;

	fn visit_component(&mut self, ir: &ComponentIR) -> Self::Output {
		// Generate component wrapper with props
		let _props = ir.props.iter().map(|p| {
			let name = &p.name;
			quote! { #name }
		});

		let body = ir.body.iter().map(|node| self.visit_node(node));

		quote! {
			{
				#(#body)*
			}
		}
	}

	fn visit_prop(&mut self, ir: &PropIR) -> Self::Output {
		let name = &ir.name;
		let ty = &ir.ty;
		quote! {
			#name: #ty
		}
	}

	fn visit_element(&mut self, ir: &ElementIR) -> Self::Output {
		let tag = &ir.tag;
		// Collect all values first to avoid multiple mutable borrows
		let attrs: Vec<_> = ir
			.attributes
			.iter()
			.map(|a| self.visit_attribute(a))
			.collect();
		let events: Vec<_> = ir.events.iter().map(|e| self.visit_event(e)).collect();
		let children: Vec<_> = ir.children.iter().map(|c| self.visit_node(c)).collect();

		quote! {
			MobileElement::new(#tag)
				#(.attr(#attrs))*
				#(.on(#events))*
				#(.child(#children))*
		}
	}

	fn visit_text(&mut self, ir: &TextIR) -> Self::Output {
		let content = &ir.content;
		quote! { MobileText::new(#content) }
	}

	fn visit_expression(&mut self, ir: &ExprIR) -> Self::Output {
		let expr = &ir.expr;
		quote! { MobileExpr::new(#expr) }
	}

	fn visit_conditional(&mut self, ir: &ConditionalIR) -> Self::Output {
		let condition = &ir.condition;
		let then_body = ir.then_body.iter().map(|n| self.visit_node(n));

		quote! {
			if #condition {
				#(#then_body)*
			}
		}
	}

	fn visit_loop(&mut self, ir: &LoopIR) -> Self::Output {
		let iterator = &ir.iterator;
		let body = ir.body.iter().map(|n| self.visit_node(n));

		quote! {
			for item in #iterator {
				#(#body)*
			}
		}
	}

	fn visit_fragment(&mut self, ir: &[NodeIR]) -> Self::Output {
		let nodes = ir.iter().map(|n| self.visit_node(n));
		quote! { #(#nodes)* }
	}

	fn visit_component_call(&mut self, ir: &ComponentCallIR) -> Self::Output {
		let name = syn::parse_str::<syn::Ident>(&ir.name).unwrap_or_else(|_| {
			syn::Ident::new("UnknownComponent", proc_macro2::Span::call_site())
		});
		let args = ir.args.iter().map(|(k, v)| {
			let key = syn::parse_str::<syn::Ident>(k)
				.unwrap_or_else(|_| syn::Ident::new("arg", proc_macro2::Span::call_site()));
			quote! { #key: #v }
		});

		quote! { #name { #(#args),* } }
	}

	fn visit_watch(&mut self, ir: &WatchIR) -> Self::Output {
		let body = ir.body.iter().map(|n| self.visit_node(n));
		quote! {
			watch! {
				#(#body)*
			}
		}
	}

	fn visit_attribute(&mut self, ir: &AttributeIR) -> Self::Output {
		let name = &ir.name;
		// TODO: Handle AttrValueIR variants properly
		quote! { (#name, "value") }
	}

	fn visit_event(&mut self, ir: &EventIR) -> Self::Output {
		let name = &ir.name;
		let handler = &ir.handler;
		quote! { (#name, #handler) }
	}

	fn visit_form(&mut self, ir: &FormIR) -> Self::Output {
		let name = &ir.name;
		let fields = ir.fields.iter().map(|f| self.visit_field(f));

		quote! {
			MobileForm::new(#name)
				#(.field(#fields))*
		}
	}

	fn visit_field(&mut self, ir: &FieldIR) -> Self::Output {
		let name = &ir.name;
		quote! { MobileField::new(#name) }
	}

	fn visit_head(&mut self, ir: &HeadIR) -> Self::Output {
		let elements = ir.elements.iter().map(|e| self.visit_head_element(e));
		quote! { #(#elements)* }
	}

	fn visit_head_element(&mut self, _ir: &HeadElementIR) -> Self::Output {
		// TODO: Handle different head element types
		quote! { () }
	}
}
