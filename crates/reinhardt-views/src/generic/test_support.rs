use reinhardt_db::orm::{
	CustomManager, FieldSelector, Filter, FilterOperator, FilterValue, Manager, Model, QuerySet,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ManagedArticle {
	pub(crate) id: Option<i64>,
	pub(crate) title: String,
	pub(crate) is_archived: bool,
	pub(crate) tenant_id: i64,
}

#[derive(Clone)]
pub(crate) struct ManagedArticleFields;

impl FieldSelector for ManagedArticleFields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

impl Model for ManagedArticle {
	type PrimaryKey = i64;
	type Fields = ManagedArticleFields;
	type Objects = VisibleArticleManager;

	fn table_name() -> &'static str {
		"managed_articles"
	}

	fn primary_key(&self) -> Option<Self::PrimaryKey> {
		self.id
	}

	fn set_primary_key(&mut self, value: Self::PrimaryKey) {
		self.id = Some(value);
	}

	fn new_fields() -> Self::Fields {
		ManagedArticleFields
	}
}

#[derive(Default)]
pub(crate) struct VisibleArticleManager;

impl CustomManager for VisibleArticleManager {
	type Model = ManagedArticle;

	fn new() -> Self {
		Self
	}

	fn all(&self) -> QuerySet<ManagedArticle> {
		Manager::<ManagedArticle>::new().all().filter(Filter::new(
			"is_archived",
			FilterOperator::Eq,
			FilterValue::Boolean(false),
		))
	}
}

pub(crate) fn explicit_queryset() -> QuerySet<ManagedArticle> {
	QuerySet::<ManagedArticle>::new().filter(Filter::new(
		"tenant_id",
		FilterOperator::Eq,
		FilterValue::Integer(42),
	))
}

pub(crate) fn assert_default_manager_queryset(queryset: QuerySet<ManagedArticle>) {
	let filters = queryset.filters();
	assert_eq!(filters.len(), 1);
	assert_eq!(filters[0].field, "is_archived");

	assert!(
		matches!(filters[0].operator, FilterOperator::Eq),
		"expected Eq operator, got: {:?}",
		filters[0].operator
	);
	assert!(
		matches!(filters[0].value, FilterValue::Boolean(false)),
		"expected Boolean(false) value, got: {:?}",
		filters[0].value
	);
}

pub(crate) fn assert_explicit_queryset(queryset: QuerySet<ManagedArticle>) {
	let filters = queryset.filters();
	assert_eq!(filters.len(), 1);
	assert_eq!(filters[0].field, "tenant_id");

	assert!(
		matches!(filters[0].operator, FilterOperator::Eq),
		"expected Eq operator, got: {:?}",
		filters[0].operator
	);
	assert!(
		matches!(filters[0].value, FilterValue::Integer(42)),
		"expected Integer(42) value, got: {:?}",
		filters[0].value
	);
}
