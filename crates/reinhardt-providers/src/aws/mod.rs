//! AWS provider utilities.

pub mod credentials;
pub mod s3;

pub use credentials::{AwsCredentials, AwsCredentialsSource, AwsSigningConfig};
pub use s3::{ObjectMetadata, S3Client, S3ClientConfig};
