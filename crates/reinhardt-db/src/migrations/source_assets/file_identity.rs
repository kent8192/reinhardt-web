//! File identities that do not retain open handles between asset reads.

use cap_std::fs::{File, Metadata};
use std::io;
use std::path::Path;

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub(super) struct FileIdentity {
	device: u64,
	inode: u64,
}

impl FileIdentity {
	pub(super) fn from_path(path: &Path) -> io::Result<Self> {
		let file = File::from_std(std::fs::File::open(path)?);
		Self::from_metadata(&file.metadata()?)
	}

	pub(super) fn from_metadata(metadata: &Metadata) -> io::Result<Self> {
		#[cfg(any(unix, windows))]
		{
			#[cfg(windows)]
			use cap_fs_ext::MetadataExt;
			#[cfg(unix)]
			use cap_std::fs::MetadataExt;

			// Windows uses the volume serial number and file index for this pair.
			Ok(Self {
				device: metadata.dev(),
				inode: metadata.ino(),
			})
		}
		#[cfg(not(any(unix, windows)))]
		{
			let _ = metadata;
			Err(io::Error::new(
				io::ErrorKind::Unsupported,
				"migration SQL asset file identities are unsupported on this target",
			))
		}
	}
}
