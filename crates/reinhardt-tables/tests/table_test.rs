use reinhardt_tables::column::BaseColumn;
use reinhardt_tables::table::{SimpleTable, SortOrder};
use reinhardt_tables::{Column, Table};
use rstest::*;

#[derive(Debug, Clone)]
struct TestUser {
	id: i32,
	name: String,
	active: bool,
}

#[fixture]
fn sample_users() -> Vec<TestUser> {
	vec![
		TestUser {
			id: 1,
			name: "Alice".to_string(),
			active: true,
		},
		TestUser {
			id: 2,
			name: "Bob".to_string(),
			active: false,
		},
		TestUser {
			id: 3,
			name: "Charlie".to_string(),
			active: true,
		},
	]
}

#[rstest]
fn test_create_empty_table() {
	let table: SimpleTable<TestUser> = SimpleTable::new();
	assert_eq!(table.rows().len(), 0);
	assert_eq!(table.total_rows(), 0);
}

#[rstest]
fn test_create_table_with_rows(sample_users: Vec<TestUser>) {
	let table = SimpleTable::with_rows(sample_users);
	assert_eq!(table.rows().len(), 3);
	assert_eq!(table.total_rows(), 3);
}

#[rstest]
fn test_table_with_columns(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add columns
	let id_column: Box<dyn Column<Row = TestUser>> =
		Box::new(BaseColumn::new("id", "ID", |user: &TestUser| {
			user.id.to_string()
		}));
	let name_column: Box<dyn Column<Row = TestUser>> =
		Box::new(BaseColumn::new("name", "Name", |user: &TestUser| {
			user.name.clone()
		}));

	table.add_column(id_column);
	table.add_column(name_column);

	assert_eq!(table.columns().len(), 2);
	assert_eq!(table.columns()[0].name(), "id");
	assert_eq!(table.columns()[1].name(), "name");
}

#[rstest]
fn test_table_sort_config(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add name column
	table.add_column(Box::new(BaseColumn::new(
		"name",
		"Name",
		|user: &TestUser| user.name.clone(),
	)));

	assert!(table.sort_config().is_none());

	table.sort_by("name", SortOrder::Ascending).unwrap();

	let sort_config = table.sort_config().unwrap();
	assert_eq!(sort_config.field, "name");
	assert_eq!(sort_config.order, SortOrder::Ascending);
}

#[rstest]
fn test_table_pagination(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	assert!(table.pagination_config().is_none());

	table.paginate(1, 2).unwrap();

	let pagination = table.pagination_config().unwrap();
	assert_eq!(pagination.page, 1);
	assert_eq!(pagination.per_page, 2);
	assert_eq!(table.total_pages(), 2); // 3 rows, 2 per page = 2 pages
}

#[rstest]
fn test_table_pagination_invalid_page() {
	let mut table: SimpleTable<TestUser> = SimpleTable::new();

	let result = table.paginate(0, 10);
	assert!(result.is_err());
}

#[rstest]
fn test_table_visible_rows(sample_users: Vec<TestUser>) {
	let table = SimpleTable::with_rows(sample_users);
	let visible = table.visible_rows();

	assert_eq!(visible.len(), 3);
	assert_eq!(visible[0].name, "Alice");
	assert_eq!(visible[1].name, "Bob");
	assert_eq!(visible[2].name, "Charlie");
}

#[rstest]
fn test_table_filters(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add active column
	table.add_column(Box::new(BaseColumn::new(
		"active",
		"Active",
		|user: &TestUser| user.active.to_string(),
	)));

	assert!(table.filters().is_empty());

	let mut filters = std::collections::HashMap::new();
	filters.insert("active".to_string(), "true".to_string());

	table.filter(filters).unwrap();

	assert_eq!(table.filters().len(), 1);
	assert_eq!(table.filters().get("active"), Some(&"true".to_string()));
}

#[rstest]
fn test_table_sorting_ascending(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add name column
	table.add_column(Box::new(BaseColumn::new(
		"name",
		"Name",
		|user: &TestUser| user.name.clone(),
	)));

	// Sort by name ascending
	table.sort_by("name", SortOrder::Ascending).unwrap();

	let visible = table.visible_rows();
	assert_eq!(visible.len(), 3);
	assert_eq!(visible[0].name, "Alice");
	assert_eq!(visible[1].name, "Bob");
	assert_eq!(visible[2].name, "Charlie");
}

#[rstest]
fn test_table_sorting_descending(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add name column
	table.add_column(Box::new(BaseColumn::new(
		"name",
		"Name",
		|user: &TestUser| user.name.clone(),
	)));

	// Sort by name descending
	table.sort_by("name", SortOrder::Descending).unwrap();

	let visible = table.visible_rows();
	assert_eq!(visible.len(), 3);
	assert_eq!(visible[0].name, "Charlie");
	assert_eq!(visible[1].name, "Bob");
	assert_eq!(visible[2].name, "Alice");
}

#[rstest]
fn test_table_filtering_active_users(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add active column
	table.add_column(Box::new(BaseColumn::new(
		"active",
		"Active",
		|user: &TestUser| user.active.to_string(),
	)));

	// Filter by active = true
	let mut filters = std::collections::HashMap::new();
	filters.insert("active".to_string(), "true".to_string());
	table.filter(filters).unwrap();

	let visible = table.visible_rows();
	assert_eq!(visible.len(), 2);
	assert_eq!(visible[0].name, "Alice");
	assert_eq!(visible[1].name, "Charlie");
	assert_eq!(table.filtered_rows_count(), 2);
}

#[rstest]
fn test_table_filtering_by_name(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add name column
	table.add_column(Box::new(BaseColumn::new(
		"name",
		"Name",
		|user: &TestUser| user.name.clone(),
	)));

	// Filter by name containing "li"
	let mut filters = std::collections::HashMap::new();
	filters.insert("name".to_string(), "li".to_string());
	table.filter(filters).unwrap();

	let visible = table.visible_rows();
	assert_eq!(visible.len(), 2);
	assert_eq!(visible[0].name, "Alice");
	assert_eq!(visible[1].name, "Charlie");
	assert_eq!(table.filtered_rows_count(), 2);
}

#[rstest]
fn test_table_sort_and_filter(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add columns
	table.add_column(Box::new(BaseColumn::new(
		"name",
		"Name",
		|user: &TestUser| user.name.clone(),
	)));
	table.add_column(Box::new(BaseColumn::new(
		"active",
		"Active",
		|user: &TestUser| user.active.to_string(),
	)));

	// Filter by active = true
	let mut filters = std::collections::HashMap::new();
	filters.insert("active".to_string(), "true".to_string());
	table.filter(filters).unwrap();

	// Sort by name descending
	table.sort_by("name", SortOrder::Descending).unwrap();

	let visible = table.visible_rows();
	assert_eq!(visible.len(), 2);
	assert_eq!(visible[0].name, "Charlie");
	assert_eq!(visible[1].name, "Alice");
}

#[rstest]
fn test_table_pagination_with_filter(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add name column
	table.add_column(Box::new(BaseColumn::new(
		"name",
		"Name",
		|user: &TestUser| user.name.clone(),
	)));

	// Filter by name containing "li" (matches Alice and Charlie)
	let mut filters = std::collections::HashMap::new();
	filters.insert("name".to_string(), "li".to_string());
	table.filter(filters).unwrap();

	// Paginate: 1 per page
	table.paginate(1, 1).unwrap();

	let visible = table.visible_rows();
	assert_eq!(visible.len(), 1);
	assert_eq!(visible[0].name, "Alice");
	assert_eq!(table.filtered_rows_count(), 2);
	assert_eq!(table.total_pages(), 2);

	// Page 2
	table.paginate(2, 1).unwrap();
	let visible = table.visible_rows();
	assert_eq!(visible.len(), 1);
	assert_eq!(visible[0].name, "Charlie");
}

#[rstest]
fn test_table_sort_filter_paginate(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add columns
	table.add_column(Box::new(BaseColumn::new(
		"name",
		"Name",
		|user: &TestUser| user.name.clone(),
	)));
	table.add_column(Box::new(BaseColumn::new(
		"active",
		"Active",
		|user: &TestUser| user.active.to_string(),
	)));

	// Filter by active = true (Alice, Charlie)
	let mut filters = std::collections::HashMap::new();
	filters.insert("active".to_string(), "true".to_string());
	table.filter(filters).unwrap();

	// Sort by name descending
	table.sort_by("name", SortOrder::Descending).unwrap();

	// Paginate: 1 per page
	table.paginate(1, 1).unwrap();

	let visible = table.visible_rows();
	assert_eq!(visible.len(), 1);
	assert_eq!(visible[0].name, "Charlie"); // First after sort

	// Page 2
	table.paginate(2, 1).unwrap();
	let visible = table.visible_rows();
	assert_eq!(visible.len(), 1);
	assert_eq!(visible[0].name, "Alice"); // Second after sort
}

#[rstest]
fn test_table_sort_nonexistent_column() {
	let mut table: SimpleTable<TestUser> = SimpleTable::new();
	let result = table.sort_by("nonexistent", SortOrder::Ascending);
	assert!(result.is_err());
}

#[rstest]
fn test_table_sort_non_sortable_column(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add non-sortable column
	table.add_column(Box::new(
		BaseColumn::new("name", "Name", |user: &TestUser| user.name.clone()).sortable(false),
	));

	let result = table.sort_by("name", SortOrder::Ascending);
	assert!(result.is_err());
}

#[rstest]
fn test_table_filter_nonexistent_column(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	let mut filters = std::collections::HashMap::new();
	filters.insert("nonexistent".to_string(), "value".to_string());

	let result = table.filter(filters);
	assert!(result.is_err());
}

#[rstest]
fn test_table_filter_non_filterable_column(sample_users: Vec<TestUser>) {
	let mut table = SimpleTable::with_rows(sample_users);

	// Add non-filterable column
	table.add_column(Box::new(
		BaseColumn::new("name", "Name", |user: &TestUser| user.name.clone()).filterable(false),
	));

	let mut filters = std::collections::HashMap::new();
	filters.insert("name".to_string(), "Alice".to_string());

	let result = table.filter(filters);
	assert!(result.is_err());
}
