use std::sync::Arc;

use reinhardt::pages::i18n::{
	I18nContext, I18nError, LazyString, MessageCatalog, TranslationContext, TranslationGuard,
	activate, activate_with_catalog, deactivate, get_active_translation, get_language, get_locale,
	gettext, gettext_lazy, ngettext, ngettext_lazy, npgettext, pgettext, set_active_translation,
	set_active_translation_permanent,
};

fn translated_context() -> TranslationContext {
	let mut catalog = MessageCatalog::new("ja");
	catalog.add_translation("Hello", "こんにちは");

	let mut translations = TranslationContext::new("ja", "en-US");
	translations
		.add_catalog("ja", catalog)
		.expect("the Japanese catalog should be valid");
	translations
}

#[test]
fn pages_i18n_facade_constructs_catalog_backed_context() {
	let context = I18nContext::new(translated_context());

	assert_eq!(context.translate("Hello"), "こんにちは");
}

#[test]
fn pages_i18n_facade_restores_scoped_global_translation() {
	assert!(get_active_translation().is_none());

	{
		let _guard: TranslationGuard = set_active_translation(Arc::new(translated_context()));
		assert_eq!(gettext("Hello"), "こんにちは");
	}

	assert!(get_active_translation().is_none());
}

mod prelude_exports {
	use reinhardt::pages::prelude::*;

	#[test]
	fn pages_i18n_prelude_exports_primary_types() {
		fn accepts_primary_types(
			_catalog: MessageCatalog,
			_context: TranslationContext,
			_error: Option<I18nError>,
			_lazy: Option<LazyString>,
			_guard: Option<TranslationGuard>,
		) {
		}

		accepts_primary_types(
			MessageCatalog::new("en-US"),
			TranslationContext::new("en-US", "en-US"),
			None,
			None,
			None,
		);
	}
}

mod crate_root_exports {
	use reinhardt::pages::{
		I18nError, LazyString, MessageCatalog, TranslationContext, TranslationGuard,
	};

	#[test]
	fn pages_crate_root_exports_primary_types() {
		fn accepts_primary_types(
			_catalog: MessageCatalog,
			_context: TranslationContext,
			_error: Option<I18nError>,
			_lazy: Option<LazyString>,
			_guard: Option<TranslationGuard>,
		) {
		}

		accepts_primary_types(
			MessageCatalog::new("en-US"),
			TranslationContext::new("en-US", "en-US"),
			None,
			None,
			None,
		);
	}
}

#[test]
fn target_neutral_exports_compile() {
	let _activate: fn(&str) -> Result<(), I18nError> = activate;
	let _activate_with_catalog: fn(&str, MessageCatalog) -> Result<(), I18nError> =
		activate_with_catalog;
	let _deactivate: fn() = deactivate;
	let _get_locale: fn() -> String = get_locale;
	let _get_language: fn() -> String = get_language;
	let _gettext: fn(&str) -> String = gettext;
	let _ngettext: fn(&str, &str, usize) -> String = ngettext;
	let _pgettext: fn(&str, &str) -> String = pgettext;
	let _npgettext: fn(&str, &str, &str, usize) -> String = npgettext;
	let _gettext_lazy: fn(&str) -> LazyString = gettext_lazy;
	let _ngettext_lazy: fn(&str, &str, usize) -> LazyString = ngettext_lazy;
	let _set_permanent: fn(Arc<TranslationContext>) = set_active_translation_permanent;
}
