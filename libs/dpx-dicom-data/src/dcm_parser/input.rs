use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use bytes::Bytes;

/// Fully-buffered input for the sans-io parser: the whole stream is available as
/// one [`Bytes`] buffer. A memory-mapped file yields it zero-copy (`mapped =
/// true`, so the data set keeps it as its master buffer); other readers are read
/// into memory.
///
// ponytail: a non-mmap reader is buffered whole. Incremental, memory-bounded
// push-streaming for huge non-mmap inputs is a later layer over the same core.
pub(crate) struct Source {
    pub(crate) data: Bytes,
    pub(crate) mapped: bool,
}

impl Source {
    /// Memory-maps a file. Errors (rather than falling back) if mapping fails.
    pub(crate) fn mmap(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        // SAFETY: the file is opened read-only. As is standard for mmap, external
        // truncation while mapped could raise SIGBUS; that risk is accepted.
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        Ok(Self { data: Bytes::from_owner(mmap), mapped: true })
    }

    /// Reads a reader fully into memory.
    pub(crate) fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(Self { data: Bytes::from(buf), mapped: false })
    }

    pub(crate) fn from_bytes(data: Bytes, mapped: bool) -> Self {
        Self { data, mapped }
    }
}
