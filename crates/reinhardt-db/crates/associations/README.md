# reinhardt-associations

Model associations and relationships

## Overview

Provides SQLAlchemy-style association proxies for simplifying access to related objects through associations. This crate enables elegant and type-safe access to attributes of related objects without manual traversal.

## Features

### Implemented ✓

#### Association Proxy (`AssociationProxy<S, A, T>`)

- **Single object attribute access**: Access attributes of related objects through foreign key and one-to-one relationships
- **Type-safe proxies**: Compile-time type checking for association chains
- **Generic implementation**: Works with any source type, associated type, and target attribute type
- **Key methods**:
  - `new()`: Create a new association proxy with custom getter functions
  - `get()`: Retrieve the target attribute through the association

#### Association Collection (`AssociationCollection<S, C, T>`)

- **Collection attribute access**: Access attributes of items in collections through one-to-many and many-to-many relationships
- **Batch operations**: Retrieve all target attributes from a collection at once
- **Collection utilities**: Count and check emptiness of collections
- **Key methods**:
  - `new()`: Create a new association collection proxy with custom getter functions
  - `get_all()`: Get all target attributes from the collection
  - `count()`: Count the number of items in the collection
  - `is_empty()`: Check if the collection is empty

#### Prelude Module

- Re-exports commonly used types for convenient importing