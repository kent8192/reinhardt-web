# reinhardt-taggit

Simple tagging system for Reinhardt framework, inspired by django-taggit.

## Features

- **Tag Model**: Core tag entities with normalized names and slugs
- **Tagged Item Model**: Polymorphic many-to-many relationships
- **TaggableManager**: High-level API for managing tags on model instances
- **#[taggable] Macro**: Zero-boilerplate tagging with attribute macro
- **Query API**: Filter objects by tags, tag clouds, and statistics
- **Tag Normalization**: Configurable tag name normalization strategies

## Quick Start

```rust
use reinhardt_taggit::prelude::*;

#[model(table_name = "foods")]
#[taggable]
pub struct Food {
	#[field(primary_key)]
	pub id: i64,

	#[field]
	pub name: String,
}

// Add tags to a food instance
let mut food = Food::new("Apple");
food.tags().add(&["red", "fruit"]).await?;

// Query foods by tags
let red_foods = Food::query()
	.filter_by_tags(&["red"])
	.all()
	.await?;
```

## Status

🚧 **Under Development** - This crate is in active development and not yet ready for production use.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
