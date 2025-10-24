# reinhardt-mail

Email sending and templating

## Overview

Email framework for sending emails with support for HTML and plain text, attachments, inline images, and template-based emails. Supports multiple email backends including SMTP and development console backend.

## Features

### Implemented ✓

#### Core Message Building

- **EmailMessage**: Flexible email message builder with builder pattern
  - Subject, body, from address, and recipient list configuration
  - HTML content support with plain text fallback
  - Mutable message modification methods
- **EmailMessageBuilder**: Type-safe builder for constructing email messages

#### Email Backends

- **ConsoleBackend**: Development backend that prints emails to console
- **FileBackend**: Backend for saving emails to files
- **MemoryBackend**: In-memory storage backend for testing
  - Thread-safe message storage using Arc<Mutex<>>
  - Message counting and retrieval capabilities
- **SmtpBackend**: SMTP protocol backend for sending emails
- **EmailBackend trait**: Extensible trait for implementing custom backends

#### Utility Functions

- **send_mail**: Simple convenience function for sending basic emails
- **send_mail_with_backend**: Send emails using a specific backend
- **send_mass_mail**: Bulk email sending for efficient multi-message delivery
- **mail_admins**: Send emails to administrators with subject prefix support
- **mail_managers**: Send emails to managers with subject prefix support

#### Settings Integration

- **backend_from_settings**: Backend initialization from configuration
- Integration with reinhardt-settings for email configuration
  - Admin and manager email lists
  - Subject prefix configuration
  - Server email and from email settings

#### Error Handling

- **EmailError**: Comprehensive error types for email operations
  - InvalidAddress: Email address validation errors
  - MissingField: Required field validation
  - BackendError: Backend-specific errors
  - SmtpError: SMTP protocol errors
  - IoError: File system and I/O errors
  - TemplateError: Template rendering errors
  - AttachmentError: File attachment errors
  - InvalidHeader: Email header validation errors
  - HeaderInjection: Security validation for header injection attacks

### Planned

- Template system integration for dynamic email content
- File attachment support (Alternative and Attachment structs are defined but not implemented)
- Inline image support for HTML emails
- Advanced SMTP features (TLS/SSL, authentication methods)
- Email validation and sanitization
- Async email sending capabilities
- Connection pooling for bulk email operations
