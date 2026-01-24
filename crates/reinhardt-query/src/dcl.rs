//! Data Control Language (DCL) support for reinhardt-query
//!
//! This module provides type-safe builders for GRANT and REVOKE statements.
//!
//! # Examples
//!
//! ```
//! use reinhardt_query::dcl::{Privilege, ObjectType, Grantee};
//!
//! // Create privilege
//! let privilege = Privilege::Select;
//! assert_eq!(privilege.as_sql(), "SELECT");
//!
//! // Create object type
//! let object_type = ObjectType::Table;
//! assert_eq!(object_type.as_sql(), "TABLE");
//!
//! // Create grantee
//! let grantee = Grantee::role("app_user");
//! ```

mod alter_role;
mod alter_user;
mod create_role;
mod create_user;
mod drop_role;
mod drop_user;
mod grant;
mod grant_role;
mod grantee;
mod object;
mod privilege;
mod rename_user;
mod reset_role;
mod revoke;
mod revoke_role;
mod role_attributes;
mod role_specification;
mod set_default_role;
mod set_role;
mod user_options;

#[cfg(test)]
mod tests;

pub use alter_role::AlterRoleStatement;
pub use alter_user::AlterUserStatement;
pub use create_role::CreateRoleStatement;
pub use create_user::CreateUserStatement;
pub use drop_role::DropRoleStatement;
pub use drop_user::DropUserStatement;
pub use grant::GrantStatement;
pub use grant_role::GrantRoleStatement;
pub use grantee::Grantee;
pub use object::ObjectType;
pub use privilege::Privilege;
pub use rename_user::RenameUserStatement;
pub use reset_role::ResetRoleStatement;
pub use revoke::RevokeStatement;
pub use revoke_role::RevokeRoleStatement;
pub use role_attributes::RoleAttribute;
pub use role_specification::{DropBehavior, RoleSpecification};
pub use set_default_role::{DefaultRoleSpec, SetDefaultRoleStatement};
pub use set_role::{RoleTarget, SetRoleStatement};
pub use user_options::UserOption;
