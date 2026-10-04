use super::*;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
fn optional<T: ToTokens>(value: &Option<T>) -> TokenStream {
	match value {
		Some(value) => quote! { Some(#value) },
		None => quote! { None },
	}
}
impl ToTokens for QualifiedName {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let name = &self.name;
		let mut builder = quote! { QualifiedName::new(#name) };
		if let Some(schema) = &self.schema {
			builder.extend(quote! { .with_schema(#schema) });
		}
		tokens.extend(builder);
	}
}
impl ToTokens for SequenceKey {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let app = &self.app_label;
		let logical = &self.logical_name;
		tokens.extend(quote! { SequenceKey::new(#app, #logical) });
	}
}
impl ToTokens for SequenceDataType {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(match self {
			Self::SmallInteger => quote! { SequenceDataType::SmallInteger },
			Self::Integer => quote! { SequenceDataType::Integer },
			Self::BigInteger => quote! { SequenceDataType::BigInteger },
		});
	}
}
impl ToTokens for SequenceBound {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(match self {
			Self::Default => quote! { SequenceBound::Default },
			Self::Value(value) => quote! { SequenceBound::Value(#value) },
		});
	}
}
impl ToTokens for SequenceOptions {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let mut builder = quote! { SequenceOptions::new() };
		if let Some(value) = self.data_type {
			builder.extend(quote! { .with_data_type(#value) });
		}
		if let Some(value) = self.increment {
			builder.extend(quote! { .with_increment(#value) });
		}
		if let Some(value) = self.min_value {
			builder.extend(quote! { .with_min_value(#value) });
		}
		if let Some(value) = self.max_value {
			builder.extend(quote! { .with_max_value(#value) });
		}
		if let Some(value) = self.start {
			builder.extend(quote! { .with_start(#value) });
		}
		if let Some(value) = self.cache {
			builder.extend(quote! { .with_cache(#value) });
		}
		if let Some(value) = self.cycle {
			builder.extend(quote! { .with_cycle(#value) });
		}
		tokens.extend(builder);
	}
}
impl ToTokens for SequenceOwner {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let table = &self.table;
		let column = &self.column;
		tokens.extend(quote! { SequenceOwner::new(#table, #column) });
	}
}
impl ToTokens for SequenceDefinition {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let key = &self.key;
		let name = &self.name;
		let options = &self.options;
		let owner = optional(&self.owned_by);
		tokens.extend(
			quote! { SequenceDefinition::new(#key, #name).with_options(#options).with_owned_by(#owner) },
		);
	}
}
impl ToTokens for SequenceDefault {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let key = &self.key;
		let name = &self.name;
		tokens.extend(quote! { SequenceDefault::new(#key, #name) });
	}
}
impl ToTokens for IdentityGeneration {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(match self {
			Self::Always => quote! { IdentityGeneration::Always },
			Self::ByDefault => quote! { IdentityGeneration::ByDefault },
		});
	}
}
impl ToTokens for IdentityDefinition {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let generation = &self.generation;
		let options = &self.options;
		let mut builder = quote! { IdentityDefinition::new(#generation).with_options(#options) };
		if let Some(name) = &self.sequence_name {
			builder.extend(quote! { .with_sequence_name(#name) });
		}
		tokens.extend(builder);
	}
}
impl ToTokens for IdentityOperation {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		let table = &self.table;
		let column = &self.column;
		let field_type = match self.field_type {
			FieldType::SmallInteger => quote! { FieldType::SmallInteger },
			FieldType::Integer => quote! { FieldType::Integer },
			FieldType::BigInteger => quote! { FieldType::BigInteger },
			_ => panic!("identity operation requires an integer column"),
		};
		let old = optional(&self.old);
		let new = optional(&self.new);
		let mut builder =
			quote! { IdentityOperation::new(#table, #column, #field_type, #old, #new) };
		if let Some(old_type) = &self.old_field_type {
			let old_type = match old_type {
				FieldType::SmallInteger => quote! { FieldType::SmallInteger },
				FieldType::Integer => quote! { FieldType::Integer },
				FieldType::BigInteger => quote! { FieldType::BigInteger },
				_ => panic!("identity operation requires an integer column"),
			};
			builder.extend(quote! { .with_old_field_type(#old_type) });
		}
		tokens.extend(builder);
	}
}
impl ToTokens for SequenceOperation {
	fn to_tokens(&self, tokens: &mut TokenStream) {
		tokens.extend(match self {
        Self::Create { definition } => quote! { SequenceOperation::Create { definition: #definition } },
        Self::Drop { definition } => quote! { SequenceOperation::Drop { definition: #definition } },
        Self::Alter { old, new } => quote! { SequenceOperation::Alter { old: #old, new: #new } },
        Self::Rename { key, old, new } => quote! { SequenceOperation::Rename { key: #key, old: #old, new: #new } },
        Self::Ownership { key, name, old, new } => { let old = optional(old); let new = optional(new); quote! { SequenceOperation::Ownership { key: #key, name: #name, old: #old, new: #new } } },
        Self::RenameDeclaration { old, new } => quote! { SequenceOperation::RenameDeclaration { old: #old, new: #new } },
        Self::Restart { name, value, reverse_value } => { let reverse_value = optional(reverse_value); quote! { SequenceOperation::Restart { name: #name, value: #value, reverse_value: #reverse_value } } },
    });
	}
}
