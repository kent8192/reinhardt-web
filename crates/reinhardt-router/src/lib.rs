//! Shared router trait surface for the Reinhardt framework.
//!
//! This crate exists to break the circular dependency between
//! `reinhardt-urls` (which owns the concrete router implementations) and
//! `reinhardt-rest` (which needs to read namespace / path information
//! out of a router to drive its versioning strategies).
//!
//! Both crates depend on `reinhardt-router` instead of each other,
//! and concrete router types implement [`VersionedRouter`] so that
//! `reinhardt-rest::versioning` can operate generically without knowing
//! about URL pattern internals.
//!
//! See <https://github.com/kent8192/reinhardt-web/issues/4321>.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod router;
pub mod version;

pub use router::VersionedRouter;
pub use version::RouteVersionInfo;
