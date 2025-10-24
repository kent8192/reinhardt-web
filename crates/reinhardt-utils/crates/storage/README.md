# reinhardt-storage

File storage backends

## Overview

Abstraction layer for file storage with support for local filesystem, cloud storage (S3, Azure, GCS), and custom storage backends. Handles file uploads, downloads, and URL generation.

## Features

### Implemented ✓

#### Core Storage Abstraction

- **Storage Trait**: Async trait defining standard storage operations
  - File save, read, delete operations
  - File existence checking and metadata retrieval
  - Directory listing functionality
  - URL generation for file access
  - File timestamp operations (accessed, created, modified times)

#### File and Metadata Handling

- **FileMetadata**: Comprehensive file metadata structure
  - Path, size, content type tracking
  - Creation and modification timestamps
  - Optional checksum support (SHA-256)
  - Builder pattern methods (`with_content_type`, `with_checksum`)
- **StoredFile**: File representation with metadata and content

#### Error Handling

- **StorageError**: Comprehensive error types
  - File not found errors
  - I/O error propagation
  - Invalid path detection
  - Storage full conditions
  - Permission denied errors
  - File already exists errors

#### Local Filesystem Storage

- **LocalStorage**: Production-ready local filesystem backend
  - Automatic directory creation
  - Path traversal attack prevention
  - SHA-256 checksum computation
  - File timestamp retrieval (accessed, created, modified)
  - URL generation with configurable base URL
  - Comprehensive directory listing

#### In-Memory Storage

- **InMemoryStorage**: Testing and development storage backend
  - Thread-safe in-memory file storage using Arc<RwLock>
  - Timestamp tracking (accessed, created, modified)
  - Directory-style path organization
  - Configurable file and directory permission modes
  - Django-style deconstruction for serialization
  - Clone support for easy testing

#### Prelude Module

- Re-exports of commonly used types for convenient importing

### Planned

Currently all planned features are implemented.
