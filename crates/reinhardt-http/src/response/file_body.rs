//! Owned file ranges for bounded-memory HTTP transport.

use bytes::Bytes;
use std::fs::File;
use std::io;
#[cfg(not(unix))]
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

/// An owned file range read in bounded chunks by native transports (experimental, P0).
///
/// Clones retain the same open file, with independent logical read positions.
/// Unix transports use offset reads; other platforms protect each seek/read pair
/// shared by body clones with a short lock. Transports perform blocking reads off the
/// async executor. The final owner closes the file; no pathname is reopened.
#[derive(Debug, Clone)]
pub struct FileResponseBody {
	file: Arc<Mutex<File>>,
	offset: u64,
	length: u64,
}

impl PartialEq for FileResponseBody {
	fn eq(&self, other: &Self) -> bool {
		Arc::ptr_eq(&self.file, &other.file)
			&& self.offset == other.offset
			&& self.length == other.length
	}
}

impl Eq for FileResponseBody {}

impl FileResponseBody {
	/// Own a regular file and a range fully contained in its current length.
	pub fn new(file: File, offset: u64, length: u64) -> io::Result<Self> {
		let metadata = file.metadata()?;
		if !metadata.is_file()
			|| offset
				.checked_add(length)
				.is_none_or(|end| end > metadata.len())
		{
			return Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"response range must fit within a regular file",
			));
		}
		Ok(Self {
			file: Arc::new(Mutex::new(file)),
			offset,
			length,
		})
	}

	/// Number of bytes in this representation or partial representation.
	pub fn len(&self) -> u64 {
		self.length
	}

	/// Whether the selected range is empty.
	pub fn is_empty(&self) -> bool {
		self.length == 0
	}

	/// Read at most `max_bytes` from a position relative to this range.
	///
	/// An empty chunk denotes exactly the end of the range. A truncated file is an
	/// error, never a silently successful short response. This method performs
	/// blocking I/O; async transports should use a blocking-task executor.
	pub fn read_chunk(&self, relative_offset: u64, max_bytes: usize) -> io::Result<Bytes> {
		if relative_offset > self.length || max_bytes == 0 {
			return Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"invalid response chunk position or size",
			));
		}
		let size = (self.length - relative_offset).min(max_bytes as u64) as usize;
		if size == 0 {
			return Ok(Bytes::new());
		}
		let mut buffer = vec![0; size];
		let file = self
			.file
			.lock()
			.map_err(|_| io::Error::other("response file lock poisoned"))?;
		#[cfg(unix)]
		std::os::unix::fs::FileExt::read_exact_at(
			&*file,
			&mut buffer,
			self.offset + relative_offset,
		)?;
		#[cfg(not(unix))]
		{
			let mut file = file;
			file.seek(SeekFrom::Start(self.offset + relative_offset))?;
			file.read_exact(&mut buffer)?;
		}
		Ok(Bytes::from(buffer))
	}
}
