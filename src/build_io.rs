//! File operations retaining their operation, path and original I/O error.
use crate::{BuildError, IoOperation};
use std::{
	fs,
	path::{Path, PathBuf},
};

pub(super) fn read_to_string(path: &Path) -> Result<String, BuildError> {
	fs::read_to_string(path).map_err(|source| BuildError::io(IoOperation::Read, path, source))
}

pub(super) fn read(path: &Path) -> Result<Vec<u8>, BuildError> {
	fs::read(path).map_err(|source| BuildError::io(IoOperation::Read, path, source))
}

pub(super) fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<(), BuildError> {
	let path = path.as_ref();
	fs::write(path, contents).map_err(|source| BuildError::io(IoOperation::Write, path, source))
}

pub(super) fn create_dir_all(path: &Path) -> Result<(), BuildError> {
	fs::create_dir_all(path)
		.map_err(|source| BuildError::io(IoOperation::CreateDirectory, path, source))
}

pub(super) fn canonicalize(path: &Path) -> Result<PathBuf, BuildError> {
	path.canonicalize()
		.map_err(|source| BuildError::io(IoOperation::Canonicalize, path, source))
}

pub(super) fn read_dir(path: &Path) -> Result<fs::ReadDir, BuildError> {
	fs::read_dir(path).map_err(|source| BuildError::io(IoOperation::ReadDirectory, path, source))
}

pub(super) fn symlink_metadata(path: &Path) -> Result<fs::Metadata, BuildError> {
	fs::symlink_metadata(path).map_err(|source| BuildError::io(IoOperation::Metadata, path, source))
}
