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
#[serial(i18n)]
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
#[serial(i18n)]
fn t_macro_interpolates_named_values() {
	let context = sample_i18n_context();
	let _guard = provide_i18n_context(context);

	let view = PageElement::new("span")
		.child(t!("{count} items", count = 3))
		.into_page();

	assert_eq!(view.render_to_string(), "<span>3 件</span>");
}

#[test]
#[serial(i18n)]
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
#[serial(i18n)]
fn hydration_context_restores_ssr_resolved_i18n_catalogs() {
	let context = sample_i18n_context();
	let mut state = SsrState::new();
	write_i18n_ssr_state(&mut state, &context);

	let hydration = HydrationContext::from_state(state);
	let _guard = provide_i18n_from_hydration_context(&hydration)
		.unwrap()
		.expect("SSR state should contain i18n metadata");

	let view = PageElement::new("span").child(tr("Hello")).into_page();

	assert_eq!(view.render_to_string(), "<span>こんにちは</span>");
}

#[test]
#[serial(i18n)]
fn hydration_i18n_preserves_catalog_registration_locale() {
	let mut translations = TranslationContext::new("fr-CA", "en-US");
	let mut fr = MessageCatalog::new("fr");
	fr.add_translation("Hello", "Bonjour");
	translations.add_catalog("fr-CA", fr).unwrap();
	let context = I18nContext::new(translations);

	let mut state = SsrState::new();
	write_i18n_ssr_state(&mut state, &context);
	let hydration = HydrationContext::from_state(state);
	let _guard = provide_i18n_from_hydration_context(&hydration)
		.unwrap()
		.expect("SSR state should contain i18n metadata");

	let view = PageElement::new("span").child(tr("Hello")).into_page();

	assert_eq!(view.render_to_string(), "<span>Bonjour</span>");
}

#[test]
#[serial(i18n)]
fn translated_text_builder_defers_rendering_to_page_render() {
	let translated = tr("Hello");
	let view = PageElement::new("p").child(translated).into_page();
	let context = sample_i18n_context();
	let _guard = provide_i18n_context(context.clone());

	let ja_html = view.render_to_string();
	context.set_locale("fr").unwrap();
	let fr_html = view.render_to_string();

	assert_eq!(ja_html, "<p>こんにちは</p>");
	assert_eq!(fr_html, "<p>Bonjour</p>");
}

#[test]
#[serial(i18n)]
fn renderer_state_contains_i18n_metadata_after_low_level_render() {
	let context = sample_i18n_context();
	let mut renderer = SsrRenderer::with_options(SsrOptions::new().i18n_context(context));
	let view = page!(|| {
		p { { t!("Hello") } }
	})();

	let html = renderer.render_view(&view);

	assert_eq!(html, "<p>こんにちは</p>");
	assert!(
		renderer.state().get_metadata("pages.i18n").is_some(),
		"low-level renderer state should include i18n metadata",
	);
}

#[test]
#[serial(i18n)]
fn renderer_html_lang_tracks_current_i18n_locale() {
	let context = sample_i18n_context();
	let mut renderer = SsrRenderer::with_options(SsrOptions::new().i18n_context(context.clone()));
	context.set_locale("fr").unwrap();

	let view = page!(|| {
		p { { t!("Hello") } }
	})();
	let html = renderer.render_page_with_view_head(view);

	assert!(
		html.contains("<html lang=\"fr\">"),
		"html lang should track the current i18n locale, got: {html}",
	);
	assert!(
		html.contains("<p>Bonjour</p>"),
		"SSR content should use the current i18n locale, got: {html}",
	);
}
