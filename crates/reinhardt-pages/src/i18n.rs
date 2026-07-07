//! Reactive page translation support.
//!
//! This module connects `reinhardt-i18n` catalogs to page rendering, SSR state,
//! and client hydration.

use std::borrow::Cow;
#[cfg(wasm)]
use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;

use reinhardt_i18n::{I18nError, MessageCatalog, TranslationContext};
use serde::{Deserialize, Serialize};

use crate::component::{IntoPage, Page};
use crate::hydration::HydrationContext;
use crate::reactive::{Context, ContextGuard, Signal, get_context};
use crate::ssr::SsrState;

/// Metadata key used for i18n catalogs in the SSR state script.
pub const SSR_I18N_METADATA_KEY: &str = "pages.i18n";

thread_local! {
	static I18N_CONTEXT: Context<I18nContext> = Context::new();
}

#[cfg(wasm)]
thread_local! {
	static HYDRATED_I18N_GUARDS: RefCell<Vec<ContextGuard<I18nContext>>> =
		const { RefCell::new(Vec::new()) };
}

/// Errors raised while reading or writing i18n state.
#[derive(Debug, thiserror::Error)]
pub enum I18nStateError {
	/// The serialized SSR state could not be decoded.
	#[error(transparent)]
	Decode(#[from] serde_json::Error),
	/// The decoded translation context is not valid.
	#[error(transparent)]
	I18n(#[from] I18nError),
}

/// Reactive translation context for pages.
#[derive(Clone)]
pub struct I18nContext {
	locale: Signal<String>,
	translations: Arc<TranslationContext>,
}

impl fmt::Debug for I18nContext {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("I18nContext")
			.field("locale", &self.locale.get_untracked())
			.field("fallback_locale", &self.translations.get_fallback_locale())
			.finish_non_exhaustive()
	}
}

impl I18nContext {
	/// Creates a reactive page i18n context from a translation context.
	pub fn new(translations: TranslationContext) -> Self {
		let locale = translations.get_locale().to_string();
		Self {
			locale: Signal::new(locale),
			translations: Arc::new(translations),
		}
	}

	/// Creates a context with empty catalogs for the given locale and fallback.
	pub fn empty(locale: impl Into<String>, fallback_locale: impl Into<String>) -> Self {
		Self::new(TranslationContext::new(locale, fallback_locale))
	}

	/// Returns the current locale and tracks it reactively.
	pub fn locale(&self) -> String {
		normalize_locale(&self.locale.get()).to_string()
	}

	/// Switches the active locale.
	///
	/// # Errors
	///
	/// Returns an i18n error when the locale is invalid.
	pub fn set_locale(&self, locale: impl Into<String>) -> Result<(), I18nError> {
		let locale = locale.into();
		TranslationContext::validate_locale_tag(&locale)?;
		self.locale.set(normalize_locale(&locale).to_string());
		Ok(())
	}

	/// Returns the translation context with the current page locale applied.
	pub fn translation_context(&self) -> TranslationContext {
		let mut translations = self.translations.as_ref().clone();
		translations
			.set_locale(self.locale_untracked())
			.expect("stored page locale should be valid");
		translations
	}

	pub(crate) fn locale_untracked(&self) -> String {
		normalize_locale(&self.locale.get_untracked()).to_string()
	}

	/// Translates a simple message.
	pub fn translate(&self, message: &str) -> String {
		self.translations
			.translate_for_locale(&self.locale(), message)
	}

	/// Translates a plural message.
	pub fn translate_plural(&self, singular: &str, plural: &str, count: usize) -> String {
		self.translations
			.translate_plural_for_locale(&self.locale(), singular, plural, count)
	}

	/// Translates a contextual message.
	pub fn translate_context(&self, context: &str, message: &str) -> String {
		self.translations
			.translate_context_for_locale(&self.locale(), context, message)
	}

	/// Translates a contextual plural message.
	pub fn translate_context_plural(
		&self,
		context: &str,
		singular: &str,
		plural: &str,
		count: usize,
	) -> String {
		self.translations.translate_context_plural_for_locale(
			&self.locale(),
			context,
			singular,
			plural,
			count,
		)
	}
}

/// Provides an i18n context for the current page render scope.
///
/// The returned guard removes the context when dropped.
pub fn provide_i18n_context(context: I18nContext) -> ContextGuard<I18nContext> {
	I18N_CONTEXT.with(|i18n_context| ContextGuard::new(i18n_context, context))
}

/// Returns the current i18n context when one is available.
pub fn use_i18n_context() -> Option<I18nContext> {
	I18N_CONTEXT.with(get_context)
}

/// Runs a closure with an i18n context installed.
pub fn with_i18n_context<R>(context: &I18nContext, f: impl FnOnce() -> R) -> R {
	let _guard = provide_i18n_context(context.clone());
	f()
}

/// Returns the current locale.
pub fn locale() -> String {
	use_i18n_context()
		.map(|context| context.locale())
		.unwrap_or_else(reinhardt_i18n::get_locale)
}

/// Switches the current page locale.
///
/// # Errors
///
/// Returns an i18n error when the locale is invalid.
pub fn set_locale(locale: impl Into<String>) -> Result<(), I18nError> {
	let locale = locale.into();
	if let Some(context) = use_i18n_context() {
		context.set_locale(locale)
	} else {
		reinhardt_i18n::activate(&locale)
	}
}

/// Lazily translated page text.
#[derive(Clone, Debug)]
pub struct TranslatedText {
	kind: TranslationKind,
	args: Vec<TranslationArg>,
}

impl TranslatedText {
	/// Adds a named interpolation argument.
	pub fn arg(mut self, name: impl Into<Cow<'static, str>>, value: impl ToString) -> Self {
		self.args.push(TranslationArg {
			name: name.into(),
			value: value.to_string(),
		});
		self
	}

	/// Renders this translation to a string.
	pub fn render_string(&self) -> String {
		let rendered = match (&self.kind, use_i18n_context()) {
			(TranslationKind::Simple { message }, Some(context)) => context.translate(message),
			(
				TranslationKind::Plural {
					singular,
					plural,
					count,
				},
				Some(context),
			) => context.translate_plural(singular, plural, *count),
			(TranslationKind::Context { context, message }, Some(i18n)) => {
				i18n.translate_context(context, message)
			}
			(
				TranslationKind::ContextPlural {
					context,
					singular,
					plural,
					count,
				},
				Some(i18n),
			) => i18n.translate_context_plural(context, singular, plural, *count),
			(TranslationKind::Simple { message }, None) => reinhardt_i18n::gettext(message),
			(
				TranslationKind::Plural {
					singular,
					plural,
					count,
				},
				None,
			) => reinhardt_i18n::ngettext(singular, plural, *count),
			(TranslationKind::Context { context, message }, None) => {
				reinhardt_i18n::pgettext(context, message)
			}
			(
				TranslationKind::ContextPlural {
					context,
					singular,
					plural,
					count,
				},
				None,
			) => reinhardt_i18n::npgettext(context, singular, plural, *count),
		};

		interpolate_named(rendered, &self.args)
	}
}

impl fmt::Display for TranslatedText {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.render_string())
	}
}

impl IntoPage for TranslatedText {
	fn into_page(self) -> Page {
		Page::reactive(move || Page::text(self.render_string()))
	}
}

#[derive(Clone, Debug)]
enum TranslationKind {
	Simple {
		message: Cow<'static, str>,
	},
	Plural {
		singular: Cow<'static, str>,
		plural: Cow<'static, str>,
		count: usize,
	},
	Context {
		context: Cow<'static, str>,
		message: Cow<'static, str>,
	},
	ContextPlural {
		context: Cow<'static, str>,
		singular: Cow<'static, str>,
		plural: Cow<'static, str>,
		count: usize,
	},
}

#[derive(Clone, Debug)]
struct TranslationArg {
	name: Cow<'static, str>,
	value: String,
}

/// Creates a lazily translated page string.
pub fn tr(message: impl Into<Cow<'static, str>>) -> TranslatedText {
	TranslatedText {
		kind: TranslationKind::Simple {
			message: message.into(),
		},
		args: Vec::new(),
	}
}

/// Creates a lazily translated plural page string.
pub fn tn(
	singular: impl Into<Cow<'static, str>>,
	plural: impl Into<Cow<'static, str>>,
	count: usize,
) -> TranslatedText {
	TranslatedText {
		kind: TranslationKind::Plural {
			singular: singular.into(),
			plural: plural.into(),
			count,
		},
		args: Vec::new(),
	}
}

/// Creates a lazily translated contextual page string.
pub fn tp(
	context: impl Into<Cow<'static, str>>,
	message: impl Into<Cow<'static, str>>,
) -> TranslatedText {
	TranslatedText {
		kind: TranslationKind::Context {
			context: context.into(),
			message: message.into(),
		},
		args: Vec::new(),
	}
}

/// Creates a lazily translated contextual plural page string.
pub fn tnp(
	context: impl Into<Cow<'static, str>>,
	singular: impl Into<Cow<'static, str>>,
	plural: impl Into<Cow<'static, str>>,
	count: usize,
) -> TranslatedText {
	TranslatedText {
		kind: TranslationKind::ContextPlural {
			context: context.into(),
			singular: singular.into(),
			plural: plural.into(),
			count,
		},
		args: Vec::new(),
	}
}

/// Writes the current page i18n context into SSR state.
pub fn write_i18n_ssr_state(state: &mut SsrState, context: &I18nContext) {
	let snapshot = I18nSsrSnapshot::from_i18n_context(context);
	state.add_metadata(SSR_I18N_METADATA_KEY, snapshot);
}

/// Reads an i18n context from SSR state.
///
/// # Errors
///
/// Returns an error when the serialized metadata is malformed or contains an
/// invalid locale.
pub fn i18n_context_from_ssr_state(
	state: &SsrState,
) -> Result<Option<I18nContext>, I18nStateError> {
	state
		.get_metadata(SSR_I18N_METADATA_KEY)
		.map(|value| {
			let snapshot = serde_json::from_value::<I18nSsrSnapshot>(value.clone())?;
			let translations = snapshot.into_translation_context()?;
			Ok(I18nContext::new(translations))
		})
		.transpose()
}

/// Provides the i18n context found in a hydration context.
///
/// The returned guard must be kept alive while the hydrated component renders.
///
/// # Errors
///
/// Returns an error when SSR i18n metadata is malformed.
pub fn provide_i18n_from_hydration_context(
	context: &HydrationContext,
) -> Result<Option<ContextGuard<I18nContext>>, I18nStateError> {
	context
		.get_metadata(SSR_I18N_METADATA_KEY)
		.map(|value| {
			let snapshot = serde_json::from_value::<I18nSsrSnapshot>(value.clone())?;
			let translations = snapshot.into_translation_context()?;
			Ok(provide_i18n_context(I18nContext::new(translations)))
		})
		.transpose()
}

#[cfg(wasm)]
pub(crate) fn retain_hydrated_i18n_context(guard: ContextGuard<I18nContext>) {
	HYDRATED_I18N_GUARDS.with(|guards| guards.borrow_mut().push(guard));
}

fn interpolate_named(mut rendered: String, args: &[TranslationArg]) -> String {
	for arg in args {
		let placeholder = format!("{{{}}}", arg.name);
		rendered = rendered.replace(&placeholder, &arg.value);
	}
	rendered
}

fn normalize_locale(locale: &str) -> &str {
	if locale.is_empty() || TranslationContext::validate_locale_tag(locale).is_err() {
		"en-US"
	} else {
		locale
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct I18nSsrSnapshot {
	current_locale: String,
	fallback_locale: String,
	catalogs: Vec<I18nCatalogSnapshot>,
}

impl I18nSsrSnapshot {
	fn from_i18n_context(context: &I18nContext) -> Self {
		Self::from_translation_context(context.translations.as_ref(), &context.locale_untracked())
	}

	fn from_translation_context(context: &TranslationContext, current_locale: &str) -> Self {
		Self {
			current_locale: current_locale.to_string(),
			fallback_locale: context.get_fallback_locale().to_string(),
			catalogs: context
				.catalogs()
				.map(|(locale, catalog)| I18nCatalogSnapshot::from_catalog(locale, catalog))
				.collect(),
		}
	}

	fn into_translation_context(self) -> Result<TranslationContext, I18nError> {
		let mut context = TranslationContext::new(self.current_locale, self.fallback_locale);
		for catalog in self.catalogs {
			let registration_locale = catalog.registration_locale();
			context.add_catalog(registration_locale, catalog.into_catalog())?;
		}
		Ok(context)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct I18nCatalogSnapshot {
	#[serde(default)]
	registration_locale: String,
	locale: String,
	messages: Vec<I18nMessageSnapshot>,
	plurals: Vec<I18nPluralSnapshot>,
	contexts: Vec<I18nContextMessageSnapshot>,
	context_plurals: Vec<I18nContextPluralSnapshot>,
}

impl I18nCatalogSnapshot {
	fn from_catalog(registration_locale: &str, catalog: &MessageCatalog) -> Self {
		Self {
			registration_locale: registration_locale.to_string(),
			locale: catalog.locale().to_string(),
			messages: catalog
				.translations()
				.map(|(message, translation)| I18nMessageSnapshot {
					message: message.to_string(),
					translation: translation.to_string(),
				})
				.collect(),
			plurals: catalog
				.plural_translations()
				.map(|(singular, forms)| I18nPluralSnapshot {
					singular: singular.to_string(),
					forms: forms.to_vec(),
				})
				.collect(),
			contexts: catalog
				.context_translations()
				.map(
					|(context, message, translation)| I18nContextMessageSnapshot {
						context: context.to_string(),
						message: message.to_string(),
						translation: translation.to_string(),
					},
				)
				.collect(),
			context_plurals: catalog
				.context_plural_translations()
				.map(|(context, singular, forms)| I18nContextPluralSnapshot {
					context: context.to_string(),
					singular: singular.to_string(),
					forms: forms.to_vec(),
				})
				.collect(),
		}
	}

	fn into_catalog(self) -> MessageCatalog {
		let mut catalog = MessageCatalog::new(&self.locale);
		for message in self.messages {
			catalog.add_translation(message.message, message.translation);
		}
		for plural in self.plurals {
			catalog.add_plural(plural.singular, plural.forms);
		}
		for context in self.contexts {
			catalog.add_context(context.context, context.message, context.translation);
		}
		for plural in self.context_plurals {
			catalog.add_context_plural_forms(plural.context, plural.singular, plural.forms);
		}
		catalog
	}

	fn registration_locale(&self) -> String {
		if self.registration_locale.is_empty() {
			self.locale.clone()
		} else {
			self.registration_locale.clone()
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct I18nMessageSnapshot {
	message: String,
	translation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct I18nPluralSnapshot {
	singular: String,
	forms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct I18nContextMessageSnapshot {
	context: String,
	message: String,
	translation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct I18nContextPluralSnapshot {
	context: String,
	singular: String,
	forms: Vec<String>,
}

/// Translates a message inside `page!` without explicitly threading resources.
#[macro_export]
macro_rules! t {
	($message:literal $(,)?) => {
		$crate::i18n::tr($message)
	};
	($message:literal, $($name:ident = $value:expr),+ $(,)?) => {{
		let mut __reinhardt_translation = $crate::i18n::tr($message);
		$(
			__reinhardt_translation =
				__reinhardt_translation.arg(stringify!($name), $value);
		)+
		__reinhardt_translation
	}};
}
