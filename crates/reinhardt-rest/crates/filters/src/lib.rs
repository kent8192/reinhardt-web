//! Type-safe filtering backends for Reinhardt framework
//!
//! Provides compile-time type-safe filtering using reinhardt-orm's Field<M, T> system.
//!
//! ## Planned Features
//! TODO: Custom filter backends for specialized use cases
//! TODO: Query result caching
//! TODO: Intelligent index usage
//! TODO: Query plan optimization hints
//! TODO: Relevance scoring
//! TODO: Synonym handling
//! TODO: Search result highlighting

// Core filter trait
pub mod filter;

// Type-safe filtering system
pub mod field_extensions;
pub mod fulltext;
pub mod fuzzy;
pub mod geo;
pub mod multi_term;
pub mod ordering_field;
pub mod query_filter;
pub mod range;
pub mod searchable;

// Core exports
pub use filter::{FilterBackend, FilterError, FilterResult};

// Type-safe exports
pub use field_extensions::FieldOrderingExt;
pub use fulltext::{FullTextSearchFilter, FullTextSearchMode};
pub use fuzzy::{FuzzyAlgorithm, FuzzySearchFilter};
pub use geo::{BoundingBoxFilter, DistanceFilter, DistanceUnit, NearbyFilter, PolygonFilter};
pub use multi_term::{MultiTermSearch, Operator, SearchTerm, TermType};
pub use ordering_field::{OrderDirection, OrderingField};
pub use query_filter::QueryFilter;
pub use range::{DateRangeFilter, NumericRangeFilter, RangeFilter};
pub use searchable::SearchableModel;
