//! Audit log backends

pub mod file;
pub mod memory;

pub use file::FileSecretAuditBackend;
pub use memory::MemorySecretAuditBackend;
