use super::log_buffer::{LogEntry, LogSource};
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;

/// Log file writer for TUI mode
pub struct LogFileWriter {
	backend_writer: BufWriter<File>,
	frontend_writer: BufWriter<File>,
	backend_path: PathBuf,
	frontend_path: PathBuf,
}

impl LogFileWriter {
	/// Create a new LogFileWriter
	/// Files are created in `/tmp/reinhardt-runall/{timestamp}/`
	pub async fn new() -> std::io::Result<Self> {
		// Generate timestamp for directory name
		let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
		let dir = PathBuf::from(format!("/tmp/reinhardt-runall/{}", timestamp));
		tokio::fs::create_dir_all(&dir).await?;

		// Log file paths
		let backend_path = dir.join("backend.log");
		let frontend_path = dir.join("frontend.log");

		// Open files in append mode
		let backend_file = OpenOptions::new()
			.create(true)
			.append(true)
			.open(&backend_path)
			.await?;
		let frontend_file = OpenOptions::new()
			.create(true)
			.append(true)
			.open(&frontend_path)
			.await?;

		Ok(Self {
			backend_writer: BufWriter::new(backend_file),
			frontend_writer: BufWriter::new(frontend_file),
			backend_path,
			frontend_path,
		})
	}

	/// Write a log entry to the appropriate file
	pub async fn write(&mut self, entry: &LogEntry) -> std::io::Result<()> {
		let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M:%S");
		let line = format!("[{}] {}\n", timestamp, entry.message);

		match entry.source {
			LogSource::Backend => {
				self.backend_writer.write_all(line.as_bytes()).await?;
				self.backend_writer.flush().await?;
			}
			LogSource::Frontend => {
				self.frontend_writer.write_all(line.as_bytes()).await?;
				self.frontend_writer.flush().await?;
			}
		}

		Ok(())
	}

	/// Get the backend log file path
	pub fn backend_path(&self) -> &PathBuf {
		&self.backend_path
	}

	/// Get the frontend log file path
	pub fn frontend_path(&self) -> &PathBuf {
		&self.frontend_path
	}
}

/// Spawn a task to write logs to files
pub fn spawn_log_writer_task(
	mut log_rx: mpsc::UnboundedReceiver<LogEntry>,
) -> tokio::task::JoinHandle<()> {
	tokio::spawn(async move {
		// Initialize log file writer
		let mut writer = match LogFileWriter::new().await {
			Ok(w) => {
				eprintln!("Log files created:");
				eprintln!("  Backend:  {}", w.backend_path().display());
				eprintln!("  Frontend: {}", w.frontend_path().display());
				w
			}
			Err(e) => {
				eprintln!("Failed to create log files: {}", e);
				return;
			}
		};

		// Process log entries
		while let Some(entry) = log_rx.recv().await {
			if let Err(e) = writer.write(&entry).await {
				eprintln!("Failed to write log: {}", e);
			}
		}
	})
}
