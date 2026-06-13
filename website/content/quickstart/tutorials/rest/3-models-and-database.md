+++
title = "Part 3: Models and the Database"
description = "Define the Snippet model, generate the initial migration, and apply it to the database."
weight = 30
+++

# Part 3: Models and the Database

Part 2 gave you working HTTP routes, but the data was hard-coded in `views.rs`. Now let's give the API a real table.

In this chapter you will define the `Snippet` model, generate the initial migration, apply it, and use the model builder that the `#[model]` macro creates.

## Define the Model

Open `src/apps/snippets/models.rs` and replace the placeholder with the model from the reference example:

```rust
use chrono::{DateTime, Utc};
use reinhardt::core::serde::{Deserialize, Serialize};
use reinhardt::prelude::*;

/// Snippet model representing a code snippet
#[model(app_label = "snippets", table_name = "snippets")]
#[derive(Serialize, Deserialize)]
pub struct Snippet {
	#[field(primary_key = true)]
	pub id: i64,

	#[field(max_length = 100)]
	pub title: String,

	#[field(max_length = 10000)]
	pub code: String,

	#[field(max_length = 50)]
	pub language: String,

	#[field(auto_now_add = true)]
	pub created_at: DateTime<Utc>,
}
```

Keep the attribute order exactly like this: `#[model(...)]` comes before `#[derive(...)]`. Reinhardt's model macro reads the struct, generates schema metadata, and then leaves the derived serialization implementations in place.

The `app_label = "snippets"` value must match the label registered by `installed_apps!` in `src/config/apps.rs`. The `table_name = "snippets"` value is the database table name the migration will create.

The fields are ordinary Rust fields with database metadata:

- `id` is the primary key.
- `title`, `code`, and `language` become required `VARCHAR` columns with the declared lengths.
- `created_at` is filled when a new model is built because it uses `auto_now_add = true`.

## Add a Normal Model Method

The reference example also gives `Snippet` a method that turns the code into highlighted HTML:

```rust
impl Snippet {
	/// Get a highlighted version of the code using syntect
	///
	/// Returns HTML-formatted code with syntax highlighting based on the snippet's language.
	/// Falls back to plain text if the language is not recognized.
	pub fn highlighted(&self) -> String {
		use syntect::highlighting::ThemeSet;
		use syntect::html::highlighted_html_for_string;
		use syntect::parsing::SyntaxSet;

		let ss = SyntaxSet::load_defaults_newlines();
		let ts = ThemeSet::load_defaults();

		let syntax = ss
			.find_syntax_by_name(&self.language)
			.or_else(|| ss.find_syntax_by_extension(&self.language))
			.unwrap_or_else(|| ss.find_syntax_plain_text());

		let theme = &ts.themes["base16-ocean.dark"];

		highlighted_html_for_string(&self.code, &ss, syntax, theme)
			.unwrap_or_else(|_| self.code.clone())
	}
}
```

This method is not special to Reinhardt. It is just Rust code on your model. In Part 5, the response serializer will call it so API clients receive both the original code and a highlighted version.

## Build Model Values

The `#[model]` macro creates a typestate builder. You will use it in Part 4 when the `POST /api/snippets/` handler inserts rows:

```rust
let snippet = Snippet::build()
	.title("Hello Reinhardt")
	.code("fn main() { println!(\"Hello, Reinhardt!\"); }")
	.language("rust")
	.finish();
```

You do not set `id` or `created_at` here. `id` defaults to `0`, which the ORM treats as "let the database allocate the auto-increment value" during insert. `created_at` is stamped by the builder because the field is marked `auto_now_add = true`.

## Generate the Migration

Run:

```bash
cargo make makemigrations
```

This wraps `cargo run --bin manage makemigrations`. The command scans registered `#[model]` types, groups them by `app_label`, compares them with existing migrations, and writes a migration when the schema changed.

For the snippets app, the generated file should be:

```text
migrations/snippets/0001_initial.rs
```

The reference migration creates the `snippets` table:

```rust
use reinhardt::db::migrations::FieldType;
use reinhardt::db::migrations::prelude::*;

pub(super) fn migration() -> Migration {
	Migration {
		app_label: "snippets".to_string(),
		name: "0001_initial".to_string(),
		operations: vec![Operation::CreateTable {
			name: "snippets".to_string(),
			columns: vec![
				ColumnDefinition {
					name: "code".to_string(),
					type_definition: FieldType::VarChar(10000u32),
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
				ColumnDefinition {
					name: "created_at".to_string(),
					type_definition: FieldType::TimestampTz,
					not_null: true,
					unique: false,
					primary_key: false,
					auto_increment: false,
					default: None,
				},
				ColumnDefinition {
					name: "id".to_string(),
					type_definition: FieldType::BigInteger,
					not_null: true,
					unique: false,
					primary_key: true,
					auto_increment: true,
					default: None,
				},
			],
			constraints: vec![],
			without_rowid: None,
			interleave_in_parent: None,
			partition: None,
		}],
		dependencies: vec![],
		atomic: true,
		replaces: vec![],
		initial: Some(true),
		state_only: false,
		database_only: false,
		swappable_dependencies: vec![],
		optional_dependencies: vec![],
	}
}
```

The excerpt above trims the repeated `language` and `title` column definitions, but your generated file should include all five model fields: `code`, `created_at`, `id`, `language`, and `title`.

## Apply the Migration

Run:

```bash
cargo make migrate
```

The reference example's make task starts the local PostgreSQL and Redis containers before running the management command, then applies every pending migration. You should see the snippets migration in the output:

```text
Applying snippets.0001_initial ... OK
```

If you changed `settings/local.toml` to point at your own PostgreSQL instance, make sure the credentials match `[core.databases.default]` before running `migrate`.

## Check That the Project Still Compiles

Run:

```bash
cargo check --all-features
```

This catches two common mistakes quickly: putting `#[derive(...)]` before `#[model(...)]`, and forgetting to enable the crates used by the model method (`chrono` and `syntect` in this example).

## What You Built

You now have:

- A `Snippet` model owned by the `snippets` app
- Schema metadata generated by `#[model]` and `#[field]`
- An initial `migrations/snippets/0001_initial.rs` file
- A migrated database table ready for CRUD handlers
- A builder API for constructing new `Snippet` values

The HTTP handlers still return static JSON. In [Part 4: Dependency Injection](../4-dependency-injection/), you will inject a `DatabaseConnection` into those handlers and replace the temporary responses with real ORM queries.
