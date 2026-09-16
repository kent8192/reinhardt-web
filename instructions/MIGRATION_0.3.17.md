# Migration Guide: 0.3.x to 0.3.17

0.3.17 includes an intentional breaking security correction for GraphQL over
gRPC despite retaining the 0.3 release line. This is the scoped exception in
[ST-2](STABILITY_POLICY.md#st-2-scoped-security-exception-for-0317).
Applications using custom schemas with `GraphQLGrpcService` must migrate
before updating their deployment. Cargo requirements such as `"0.3.16"`
permit 0.3.17, so an ordinary dependency update can select this release.

## Why construction changes

Schema extensions can replace a request or transform its parsed document.
An operation check performed before those hooks can approve a query that
later executes as a mutation. The corrected transport surrounds the extension
chain with a guard and checks the final selected operation before resolvers run.
The dependency's public API does not allow adding that guard to a completed
schema, so schemas must install it during construction.

## Required migration

1. For schemas passed to `GraphQLGrpcService::new`, replace
   `Schema::build(query, mutation, subscription)` with
   `GraphQLGrpcService::schema_builder(query, mutation, subscription)`.
   Preserve existing `.data(...)`, `.extension(...)`, depth and complexity
   limits, and other builder calls before `.finish()`.
2. Replace `Schema::new(query, mutation, subscription)` with
   `GraphQLGrpcService::schema_builder(query, mutation, subscription).finish()`.
3. Continue passing the finished schema to `GraphQLGrpcService::new`.
   Unguarded schemas now cause a construction-time panic. Built-in
   `create_schema` helpers install the guard when `graphql-grpc` is enabled.
4. If HTTP and gRPC handlers share a schema, build it with the guarded builder
   and share that schema. Ordinary HTTP requests do not acquire a gRPC
   operation constraint.
5. Update subscription clients and tests to handle `INVALID_ARGUMENT` while
   consuming the returned stream. Invalid requests are rejected before any
   subscription resolver is created; the error is delivered through the stream.

The [guarded builder example](https://github.com/kent8192/reinhardt-web/blob/main/crates/reinhardt-graphql/src/grpc_service.rs#L67)
demonstrates the guarded builder. Query and Mutation RPCs also reject invalid
documents, invalid operation selections, and mismatched operation classes with
`INVALID_ARGUMENT` before execution.

## Verify the upgrade

Exercise application startup, each RPC operation class, custom request and
document transformations, and subscription stream errors before deploying.
Confirm rejected requests do not invoke resolvers with side effects. An
API-compatible `cargo-semver-checks` result alone does not verify these runtime
contracts.

This exception is limited to the GraphQL-over-gRPC behaviors introduced by
[the operation validation fix](https://github.com/kent8192/reinhardt-web/commit/b3245858d19a98f1a3acaf4348e8c5aca46c693d).
Other APIs and later releases retain the normal 0.3 compatibility policy.
