# reinhardt-router

Shared router trait surface for the Reinhardt framework.

This crate exists to break the circular dependency between
[`reinhardt-urls`](https://crates.io/crates/reinhardt-urls) (which owns
the concrete router implementations) and
[`reinhardt-rest`](https://crates.io/crates/reinhardt-rest) (which needs
to read namespace / path information out of a router to drive its
versioning strategies).

Both crates depend on `reinhardt-router` instead of each other, and
concrete router types implement the `VersionedRouter` trait so that
`reinhardt-rest::versioning` can operate generically without knowing
about URL pattern internals.

See <https://github.com/kent8192/reinhardt-web/issues/4321> for the
background.

## License

BSD-3-Clause.
