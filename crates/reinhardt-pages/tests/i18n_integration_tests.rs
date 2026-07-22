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
	ja.add_translation("Hello {name}", "こんにちは {name}");
	ja.add_translation("{count} items", "{count} 件");
	translations.add_catalog("ja", ja).unwrap();

	let mut fr = MessageCatalog::new("fr");
	fr.add_translation("Hello", "Bonjour");
	fr.add_translation("Hello {name}", "Bonjour {name}");
	fr.add_translation("{count} items", "{count} articles");
	translations.add_catalog("fr", fr).unwrap();

	I18nContext::new(translations)
}

#[tokio::test]
#[serial(i18n)]
async fn t_macro_renders_translated_text_during_ssr_and_serializes_catalogs() {
	let context = sample_i18n_context();
	let options = SsrOptions::new().i18n_context(context);
	let mut renderer = SsrRenderer::with_options(options);

	let view = page!(|| {
		div {
			p { { t!("Hello") } }
		}
	})();

	let html = renderer
		.render_page_with_view_head(view)
		.await
		.collect_string()
		.await;

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
fn t_macro_borrows_named_interpolation_values() {
	let name = String::from("Alice");

	let text = t!("Hello {name}", name = name);

	assert_eq!(name, "Alice");
	assert_eq!(text.render_string(), "Hello Alice");
}

#[test]
fn qualified_t_macro_inside_page_uses_regular_expression_codegen() {
	mod external {
		macro_rules! t {
			() => {
				"external"
			};
		}

		pub(crate) use t;
	}

	let view = page!(|| {
		span { { external::t!() } }
	})();

	assert_eq!(view.render_to_string(), "<span>external</span>");
}

#[test]
#[serial(i18n)]
fn t_macro_interpolation_borrows_non_copy_page_captures() {
	let context = sample_i18n_context();
	let _guard = provide_i18n_context(context);
	let view = page!(|name: String| {
		p { { t!("Hello {name}", name = name) } }
	})("Ada".to_string());

	assert_eq!(view.render_to_string(), "<p>こんにちは Ada</p>");
	assert_eq!(view.render_to_string(), "<p>こんにちは Ada</p>");
}

#[test]
#[serial(i18n)]
fn t_macro_named_interpolation_uses_implicit_page_capture_value() {
	let context = sample_i18n_context();
	let _guard = provide_i18n_context(context);
	let project_id = 42_i64;

	let view = page!({
		p { { t!("Project {id}", id = project_id) } }
	});

	assert_eq!(view.render_to_string(), "<p>Project 42</p>");
}

#[test]
#[serial(i18n)]
fn t_macro_named_interpolation_uses_strict_page_parameter_value() {
	let context = sample_i18n_context();
	let _guard = provide_i18n_context(context);

	let view = page!(|project_id: i64| {
		p { { t!("Project {id}", id = project_id) } }
	})(42);

	assert_eq!(view.render_to_string(), "<p>Project 42</p>");
}

#[test]
#[serial(i18n)]
fn page_macro_does_not_special_case_local_t_macro() {
	macro_rules! t {
		($message:literal) => {
			String::from($message)
		};
	}

	let view = page!(|| {
		p { { t!("local text") } }
	})();

	assert_eq!(view.render_to_string(), "<p>local text</p>");
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

#[tokio::test]
#[serial(i18n)]
async fn renderer_state_contains_i18n_metadata_after_low_level_render() {
	let context = sample_i18n_context();
	let mut renderer = SsrRenderer::with_options(SsrOptions::new().i18n_context(context));
	let view = page!(|| {
		p { { t!("Hello") } }
	})();

	let html = renderer.render_view_with_state(&view).await;

	assert_eq!(html, "<p>こんにちは</p>");
	assert!(
		renderer.state().get_metadata("pages.i18n").is_some(),
		"low-level renderer state should include i18n metadata",
	);
}

#[tokio::test]
#[serial(i18n)]
async fn render_view_accepts_mutable_renderer_reference() {
	let context = sample_i18n_context();
	let mut renderer = SsrRenderer::with_options(SsrOptions::new().i18n_context(context));
	let view = page!(|| {
		p { { t!("Hello") } }
	})();
	let shared_renderer = &mut renderer;

	let html = shared_renderer.render_view(&view).await;

	assert_eq!(html, "<p>こんにちは</p>");
}

#[tokio::test]
#[serial(i18n)]
async fn renderer_html_lang_tracks_current_i18n_locale() {
	let context = sample_i18n_context();
	let mut renderer = SsrRenderer::with_options(SsrOptions::new().i18n_context(context.clone()));
	context.set_locale("fr").unwrap();

	let view = page!(|| {
		p { { t!("Hello") } }
	})();
	let html = renderer
		.render_page_with_view_head(view)
		.await
		.collect_string()
		.await;

	assert!(
		html.contains("<html lang=\"fr\">"),
		"html lang should track the current i18n locale, got: {html}",
	);
	assert!(
		html.contains("<p>Bonjour</p>"),
		"SSR content should use the current i18n locale, got: {html}",
	);
}

#[test]
#[serial(i18n)]
fn empty_page_locale_resets_to_default_locale_for_public_reads_and_ssr() {
	let context = sample_i18n_context();
	context.set_locale("").unwrap();

	let mut state = SsrState::new();
	write_i18n_ssr_state(&mut state, &context);
	let metadata = state
		.get_metadata("pages.i18n")
		.expect("SSR state should include i18n metadata");

	assert_eq!(context.locale(), "en-US");
	assert_eq!(
		metadata
			.get("current_locale")
			.and_then(|value| value.as_str()),
		Some("en-US"),
	);
}

#[test]
#[serial(i18n)]
fn invalid_locale_write_is_rejected_without_changing_context() {
	let context = sample_i18n_context();

	assert!(context.set_locale("fr").is_ok());
	assert!(context.set_locale("en/US").is_err());

	let mut state = SsrState::new();
	write_i18n_ssr_state(&mut state, &context);
	let metadata = state
		.get_metadata("pages.i18n")
		.expect("SSR state should include i18n metadata");

	assert_eq!(context.locale(), "fr");
	assert_eq!(context.translation_context().get_locale(), "fr");
	assert_eq!(
		metadata
			.get("current_locale")
			.and_then(|value| value.as_str()),
		Some("fr"),
	);
}

#[test]
#[serial(i18n)]
fn translation_context_accessor_tracks_current_page_locale() {
	let context = sample_i18n_context();
	context.set_locale("fr").unwrap();

	let translations = context.translation_context();

	assert_eq!(translations.get_locale(), "fr");
	assert_eq!(translations.translate("Hello"), "Bonjour");
}
