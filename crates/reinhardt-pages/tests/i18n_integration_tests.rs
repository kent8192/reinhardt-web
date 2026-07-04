//! Integration tests for first-class reactive i18n in pages.

#![cfg(all(not(target_arch = "wasm32"), feature = "i18n"))]

use reinhardt_i18n::{MessageCatalog, TranslationContext};
use reinhardt_pages::hydration::HydrationContext;
use reinhardt_pages::i18n::{
	I18nContext, provide_i18n_context, provide_i18n_from_hydration_context, tr,
	write_i18n_ssr_state,
};
use reinhardt_pages::prelude::*;
use reinhardt_pages::ssr::{SsrOptions, SsrRenderer};
use serial_test::serial;

fn sample_i18n_context() -> I18nContext {
	let mut translations = TranslationContext::new("ja", "en-US");

	let mut ja = MessageCatalog::new("ja");
	ja.add_translation("Hello", "こんにちは");
	ja.add_translation("{count} items", "{count} 件");
	translations.add_catalog("ja", ja).unwrap();

	let mut fr = MessageCatalog::new("fr");
	fr.add_translation("Hello", "Bonjour");
	fr.add_translation("{count} items", "{count} articles");
	translations.add_catalog("fr", fr).unwrap();

	I18nContext::new(translations)
}

#[test]
#[serial]
fn t_macro_renders_translated_text_during_ssr_and_serializes_catalogs() {
	let context = sample_i18n_context();
	let options = SsrOptions::new().i18n_context(context);
	let mut renderer = SsrRenderer::with_options(options);

	let view = page!(|| {
		div {
			p { { t!("Hello") } }
		}
	})();

	let html = renderer.render_page_with_view_head(view);

	assert!(
		html.contains("<p>こんにちは</p>"),
		"SSR should render translated text, got: {html}",
	);
	assert!(
		html.contains("\"pages.i18n\""),
		"SSR state should include the resolved i18n catalog, got: {html}",
	);
}

#[test]
#[serial]
fn t_macro_interpolates_named_values() {
	let context = sample_i18n_context();
	let _guard = provide_i18n_context(context);

	let translated = t!("{count} items", count = 3).render_string();

	assert_eq!(translated, "3 件");
}

#[test]
#[serial]
fn locale_switch_rerenders_t_macro_output() {
	let context = sample_i18n_context();
	let _guard = provide_i18n_context(context.clone());

	let view = page!(|| {
		span { { t!("Hello") } }
	})();

	let ja_html = view.render_to_string();
	context.set_locale("fr").unwrap();
	let fr_html = view.render_to_string();

	assert_eq!(ja_html, "<span>こんにちは</span>");
	assert_eq!(fr_html, "<span>Bonjour</span>");
}

#[test]
#[serial]
fn hydration_context_restores_ssr_resolved_i18n_catalogs() {
	let context = sample_i18n_context();
	let mut state = SsrState::new();
	write_i18n_ssr_state(&mut state, &context);

	let hydration = HydrationContext::from_state(state);
	let _guard = provide_i18n_from_hydration_context(&hydration)
		.unwrap()
		.expect("SSR state should contain i18n metadata");

	assert_eq!(tr("Hello").render_string(), "こんにちは");
}
