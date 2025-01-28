use std::boxed::Box;
use std::io::{Read, Seek};

pub trait ReadSeek: Read + Seek + Send + Sync {}
impl<T: Read + Seek + Send + Sync> ReadSeek for T {}

/// A trait for wrapping I/O Readers.
pub trait IoWrapper: Send + Sync {
    fn wrap(&self, r: Box<dyn ReadSeek>, fuid: &str) -> Box<dyn ReadSeek>;
}
