//! Authentication and authorization re-exports.

pub use reinhardt_auth::{
	AllowAny, AuthBackend, AuthIdentity, AuthInfo, BaseUser, CurrentUser, FullUser, IsAdminUser,
	IsAuthenticated, PasswordHasher, Permission, PermissionsMixin, validate_auth_extractors,
};

#[cfg(feature = "argon2-hasher")]
#[cfg_attr(docsrs, doc(cfg(all(feature = "auth", feature = "argon2-hasher"))))]
pub use reinhardt_auth::Argon2Hasher;

#[cfg(feature = "auth-jwt")]
pub use reinhardt_auth::{Claims, JwtAuth, JwtError};

// User and group management
pub use reinhardt_auth::{
	CreateGroupData, CreateUserData, Group, GroupManagementError, GroupManagementResult,
	GroupManager, ObjectPermission, ObjectPermissionChecker, ObjectPermissionManager,
	UpdateUserData, UserManagementError, UserManagementResult, UserManager,
};
