# Native MySQL fixture readiness

## Observed failure

On 2026-10-04, the MySQL test in
[runtime_sql_generated_backend.rs](../crates/reinhardt-db/tests/runtime_sql_generated_backend.rs)
failed at container startup with `WaitContainer(StartupTimeout)`, before any
database assertions ran, on baseline `ae39903071` with the default module
readiness conditions. The default timeout and a 120-second timeout both
failed. The environment was macOS arm64 with an arm64 OrbStack Docker daemon,
Rust 1.96.0, `testcontainers-modules` 0.15.0, `testcontainers` 0.27.3, and
`sqlx` 0.8.6. The MySQL module uses `mysql:8.1`, `MYSQL_DATABASE=test`, and
`MYSQL_ALLOW_EMPTY_PASSWORD=yes`.

Reproduce the affected fixture with
`cargo test -p reinhardt-db --features mysql,sqlite --test runtime_sql_generated_backend mysql_generated_pool_execution_keeps_decimal_and_full_unsigned_range -- --exact --nocapture`.

A separate container using the same image and environment reached SQL port
3306 readiness in approximately 13 seconds and answered `SELECT 1`. Its logs
contained both strings required by the module's default readiness conditions:
`X Plugin ready for connections. Bind-address` and
`/usr/sbin/mysqld: ready for connections.`. The initial setup server uses port
0; the final server explicitly reports port 3306. This evidence establishes
that increasing the timeout alone is insufficient. The reason the default
TestContainers log waits time out has not been isolated.

## Local workaround and removal

The fixture retains the same image, environment, and RAII container ownership,
but waits for `port: 3306  MySQL Community Server` on stderr. This confirms the
final SQL server rather than its temporary setup server or X Plugin. It follows
the readiness condition already used by the repository's
[MySQL write-intent fixture](../crates/reinhardt-db/tests/mysql_write_intent.rs).
The native database connection and all generated-execution assertions still
run after container startup.

Remove the explicit readiness override when the default module conditions
pass the same native MySQL test reliably in this environment after a dependency
or environment fix. The ideal implementation is `Mysql::default().start()`;
a closed issue or a dependency version change alone is insufficient evidence.

With the override, the native MySQL scenario passed both before and after the
baseline advanced to `973ec5b99b`. The full three-backend test target then passed
on the latter baseline, including ordinary model-array CRUD, JSON hydration,
argument ordering, and rejection of direct unsupported SQL array arguments.

## Investigation and reporting

The affected definitions are in
[testcontainers-modules 0.15.0](https://docs.rs/testcontainers-modules/0.15.0/src/testcontainers_modules/mysql/mod.rs.html)
and the wait implementation in `testcontainers` 0.27.3. Both package manifests
identify the TestContainers repositories. Searches of open and closed issues
in `testcontainers/testcontainers-rs-modules-community` for `mysql` and
`testcontainers/testcontainers-rs` for `StartupTimeout` found no matching
record on 2026-10-04. The modules repository's contribution guide was reviewed.
No upstream defect is asserted, and no external report has been submitted;
external publication policy has not been verified. This internal record follows
[Upstream Issue Reporting](../instructions/UPSTREAM_ISSUE_REPORTING.md).
