# reinhardt-hybrid

Hybrid property and validation support

## Overview

Hybrid properties that work both as instance attributes and class-level query expressions. Allows defining computed properties that can be used in database queries, similar to SQLAlchemy's hybrid properties.

## Features

### Implemented ✓

#### HybridProperty

- **Instance-level getters**: Define getters that work on struct instances
  - `HybridProperty::new()` - Create a property with instance-level behavior
  - `get()` - Get the value for an instance
- **SQL expression support**: Generate SQL expressions for database queries
  - `with_expression()` - Add SQL expression generation capability
  - `expression()` - Get the SQL expression string
- **Type-safe**: Full type safety with generics `HybridProperty<T, R>`

#### HybridMethod

- **Instance-level methods**: Define methods that accept parameters
  - `HybridMethod::new()` - Create a method with instance-level behavior
  - `call()` - Call the method for an instance with arguments
- **SQL expression methods**: Generate parameterized SQL expressions
  - `with_expression()` - Add SQL expression generation capability
  - `expression()` - Get the SQL expression string with arguments
- **Type-safe**: Full type safety with generics `HybridMethod<T, A, R>`

#### SQL Expression Builders

- **SqlExpression struct**: Serializable SQL expression container
  - `new()` - Create a SQL expression from a string
  - `concat()` - Generate CONCAT expressions
  - `lower()` - Generate LOWER expressions for case-insensitive operations
  - `upper()` - Generate UPPER expressions for case-insensitive operations
  - `coalesce()` - Generate COALESCE expressions for NULL handling
- **Expression trait**: Convert types to SQL strings
  - Implemented for `SqlExpression`, `String`, and `&str`
  - `to_sql()` - Convert to SQL string representation

#### Comparator System

- **Comparator trait**: Customize SQL comparison operations
  - `new()` - Create a comparator with an expression
  - `eq()`, `ne()` - Equality and inequality comparisons
  - `lt()`, `le()`, `gt()`, `ge()` - Ordering comparisons
- **UpperCaseComparator**: Built-in case-insensitive comparator
  - Automatically applies UPPER() to both sides of comparisons

#### Property Override Support

- **HybridPropertyOverride trait**: Define overridable property behavior
  - `get_instance()` - Get instance-level value
  - `get_expression()` - Get SQL expression (optional)
  - `set_instance()` - Set instance-level value (optional)
- **OverridableProperty wrapper**: Composition-based property override
  - `new()` - Create an overridable property with custom implementation
  - `get()`, `set()` - Instance-level getters and setters
  - `expression()` - SQL expression support
  - Enables polymorphic behavior without traditional inheritance

#### Macro Support

- **hybrid_property! macro**: Convenience macro for defining hybrid properties