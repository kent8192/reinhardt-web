use crate::EmailResult;
use crate::message::EmailMessage;

/// Trait for email backends
pub trait EmailBackend: Send + Sync {
    fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize>;
}

pub fn backend_from_settings() {}

pub struct ConsoleBackend;

impl EmailBackend for ConsoleBackend {
    fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize> {
        println!("Console backend: {} messages", messages.len());
        Ok(messages.len())
    }
}

pub struct FileBackend;

impl EmailBackend for FileBackend {
    fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize> {
        Ok(messages.len())
    }
}

pub struct MemoryBackend {
    messages: std::sync::Arc<std::sync::Mutex<Vec<EmailMessage>>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            messages: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    pub fn count(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    pub fn get_messages(&self) -> Vec<EmailMessage> {
        self.messages.lock().unwrap().clone()
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl EmailBackend for MemoryBackend {
    fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize> {
        let mut stored = self.messages.lock().unwrap();
        stored.extend_from_slice(messages);
        Ok(messages.len())
    }
}

pub struct SmtpBackend;

impl EmailBackend for SmtpBackend {
    fn send_messages(&self, messages: &[EmailMessage]) -> EmailResult<usize> {
        Ok(messages.len())
    }
}
