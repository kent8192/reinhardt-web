//! REST framework re-exports.

#[cfg(any(
	feature = "api",
	feature = "api-only",
	feature = "rest",
	feature = "standard",
	feature = "full"
))]
pub use reinhardt_rest::serializers::{Deserializer, JsonSerializer, Serializer};

pub use reinhardt_rest::pagination::{
	CursorPagination, LimitOffsetPagination, PageNumberPagination, PaginatedResponse, Paginator,
};

#[cfg(any(
	feature = "api",
	feature = "api-only",
	feature = "rest",
	feature = "standard",
	feature = "full"
))]
pub use reinhardt_rest::filters::{FieldOrderingExt, MultiTermSearch};
pub use reinhardt_rest::filters::{FilterBackend, FilterError, FilterResult};

pub use reinhardt_rest::throttling::{
	AnonRateThrottle, ScopedRateThrottle, Throttle, UserRateThrottle,
};

#[cfg(any(
	feature = "api-only",
	feature = "compressed-parsers",
	feature = "rest",
	feature = "standard",
	feature = "full"
))]
pub use reinhardt_rest::parsers::{
	FileUploadParser, FormParser, JSONParser, MediaType, MultiPartParser, ParseError, ParseResult,
	Parser,
};

pub use reinhardt_rest::versioning::{
	AcceptHeaderVersioning, BaseVersioning, HostNameVersioning, NamespaceVersioning,
	QueryParameterVersioning, RequestVersionExt, URLPathVersioning, VersioningError,
	VersioningMiddleware,
};

pub use reinhardt_rest::metadata::{
	ActionMetadata, BaseMetadata, ChoiceInfo, FieldInfo, FieldInfoBuilder, FieldType,
	MetadataOptions, MetadataResponse, SimpleMetadata,
};

#[cfg(feature = "openapi")]
pub use reinhardt_rest::negotiation::MediaType;
pub use reinhardt_rest::negotiation::*;

#[cfg(any(
	feature = "api-only",
	feature = "compressed-parsers",
	feature = "rest",
	feature = "standard",
	feature = "full"
))]
pub use reinhardt_rest::parsers;
#[cfg(any(
	feature = "api",
	feature = "api-only",
	feature = "rest",
	feature = "standard",
	feature = "full"
))]
pub use reinhardt_rest::serializers;
pub use reinhardt_rest::{filters, metadata, negotiation, pagination, throttling, versioning};

#[cfg(any(
	feature = "browsable-api",
	feature = "full",
	feature = "reinhardt-browsable-api"
))]
pub use reinhardt_rest::browsable_api;

#[cfg(feature = "openapi")]
pub use reinhardt_rest::openapi::*;

#[cfg(feature = "openapi-router")]
pub use reinhardt_openapi::OpenApiRouter;
