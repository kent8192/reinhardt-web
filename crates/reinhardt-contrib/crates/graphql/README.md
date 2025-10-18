# reinhardt-graphql

GraphQL integration

## Overview

GraphQL API support with schema generation from models, query and mutation resolvers, and integration with the authentication and permission system. Provides a flexible alternative to REST APIs.

## Features

### Implemented ✓

#### Core Type System
- **GraphQL Type Markers**: `GraphQLType` and `GraphQLField` traits for type-safe GraphQL type definitions
- **Error Handling**: Custom `GraphQLError` enum with Schema, Resolver, and NotFound variants
- **Base Resolver Trait**: Async `Resolver` trait with generic output types for flexible resolver implementation

#### Schema & Data Types
- **User Type**: Complete GraphQL object implementation with id, name, email, and active fields
- **User Storage**: Thread-safe in-memory storage using `Arc<RwLock<HashMap>>` for user data
  - `new()`: Create new storage instance
  - `add_user()`: Add or update user in storage
  - `get_user()`: Retrieve user by ID
  - `list_users()`: List all stored users
- **Input Types**: `CreateUserInput` for user creation mutations
- **Schema Builder**: `create_schema()` function to build GraphQL schema with data context

#### Query Operations
- **User Queries**:
  - `user(id: ID)`: Retrieve single user by ID
  - `users()`: List all users
  - `hello(name: Option<String>)`: Simple greeting query for testing
- **Context Integration**: Queries access UserStorage through GraphQL context

#### Mutation Operations
- **User Mutations**:
  - `createUser(input: CreateUserInput)`: Create new user with auto-generated UUID
  - `updateUserStatus(id: ID, active: bool)`: Update user active status
- **State Management**: Mutations persist changes to UserStorage

#### Subscription System
- **Event Types**: `UserEvent` enum supporting Created, Updated, and Deleted events
- **Event Broadcasting**: `EventBroadcaster` with tokio broadcast channel (capacity: 100)
  - `new()`: Create new broadcaster instance
  - `broadcast()`: Send events to all subscribers
  - `subscribe()`: Subscribe to event stream
- **Subscription Root**: `SubscriptionRoot` with filtered subscription streams
  - `userCreated()`: Stream of user creation events
  - `userUpdated()`: Stream of user update events
  - `userDeleted()`: Stream of user deletion events (returns ID only)
- **Async Streams**: Real-time event filtering using async-stream

#### Integration
- **async-graphql Integration**: Built on async-graphql framework for production-ready GraphQL server
- **Type Safety**: Full Rust type system integration with compile-time guarantees
- **Async/Await**: Complete async support with tokio runtime
- **Documentation**: Comprehensive doc comments with examples for all public APIs

### Planned

Currently all planned features are implemented.

