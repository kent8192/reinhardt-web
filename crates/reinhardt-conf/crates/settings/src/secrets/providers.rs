//! Secret provider implementations
//!
//! Various backends for storing and retrieving secrets:
//! - HashiCorp Vault
//! - AWS Secrets Manager
//! - Azure Key Vault
//! - Environment variables
//! - Memory (for testing)

pub mod env;
pub mod memory;

#[cfg(feature = "vault")]
pub mod hashicorp;

#[cfg(feature = "aws-secrets")]
pub mod aws;

#[cfg(feature = "azure-keyvault")]
pub mod azure;
