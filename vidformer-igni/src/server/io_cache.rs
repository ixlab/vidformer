use log::*;
use redis::{Commands, Connection};
use std::io::{Read, Result, Seek, SeekFrom};

pub struct RedisIoCache<R> {
    inner: R,
    conn: Connection,
    chunk_size: usize,
    position: u64,
    prefix: String,
}

impl<R: Read + Seek> RedisIoCache<R> {
    pub fn new(inner: R, redis_url: &str, prefix: &str, chunk_size: usize) -> Self {
        let client = redis::Client::open(redis_url).expect("Bad Redis URL");
        let conn = client
            .get_connection()
            .expect("Cannot get Redis connection");

        RedisIoCache {
            inner,
            conn,
            chunk_size,
            position: 0,
            prefix: prefix.to_string(),
        }
    }

    fn chunk_key(&self, chunk_index: u64) -> String {
        format!("{}:chunk:{}", self.prefix, chunk_index)
    }

    fn read_one_chunk_or_eof(&mut self) -> Result<Vec<u8>> {
        let mut buf = vec![0_u8; self.chunk_size];
        let mut total_read = 0;

        // Loop until we've filled `CHUNK_SIZE` bytes or we hit EOF.
        while total_read < self.chunk_size {
            let read_now = self.inner.read(&mut buf[total_read..])?;
            if read_now == 0 {
                break;
            }
            total_read += read_now;
        }

        assert!(total_read <= self.chunk_size);
        buf.truncate(total_read);
        Ok(buf)
    }
}

impl<R: Read + Seek> Read for RedisIoCache<R> {
    fn read(&mut self, out_buf: &mut [u8]) -> Result<usize> {
        if out_buf.is_empty() {
            return Ok(0);
        }

        let chunk_index = self.position / self.chunk_size as u64;
        let offset_in_chunk = (self.position % self.chunk_size as u64) as usize;

        let bytes_left_in_chunk = self.chunk_size - offset_in_chunk;
        let to_read = std::cmp::min(bytes_left_in_chunk, out_buf.len());

        let key = self.chunk_key(chunk_index);
        let start = offset_in_chunk as isize;
        let end = (offset_in_chunk + to_read - 1) as isize; // redis GETRANGE is inclusive for some reason

        let partial_data: std::result::Result<Vec<u8>, redis::RedisError> = redis::cmd("GETRANGE")
            .arg(&key)
            .arg(start)
            .arg(end)
            .query(&mut self.conn);

        match partial_data {
            Ok(partial_data) => {
                if !partial_data.is_empty() {
                    trace!(
                        "IO cache hit - GETRANGE {} [{}-{}] returned {} bytes (requested {})",
                        key,
                        start,
                        end,
                        partial_data.len(),
                        to_read
                    );
                    out_buf[..partial_data.len()].copy_from_slice(&partial_data);
                    self.position += partial_data.len() as u64;
                    return Ok(partial_data.len());
                }
            }
            Err(err) => {
                // An actual error, ignore and move on to the underlying reader
                warn!("io cache getrange error: {:?}", err);
            }
        }

        let chunk_start_offset = chunk_index * self.chunk_size as u64;
        self.inner.seek(SeekFrom::Start(chunk_start_offset))?;

        let chunk_buf = self.read_one_chunk_or_eof()?;
        if !chunk_buf.is_empty() {
            match self.conn.set(&key, &chunk_buf) {
                Ok(()) => {} // cache set success
                Err(err) => {
                    warn!("io cache set error: {:?}", err);
                }
            }
        }

        let to_copy = std::cmp::min(to_read, chunk_buf.len() - offset_in_chunk);
        out_buf[..to_copy].copy_from_slice(&chunk_buf[offset_in_chunk..offset_in_chunk + to_copy]);
        self.position += to_copy as u64;
        Ok(to_copy)
    }
}

impl<R: Read + Seek> Seek for RedisIoCache<R> {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        let (base, offset) = match pos {
            SeekFrom::Start(off) => {
                self.position = off;
                return Ok(self.position);
            }
            SeekFrom::End(off) => {
                // Move to the end of the underlying file + off
                let end = self.inner.seek(SeekFrom::End(0))?;
                (end as i64, off)
            }
            SeekFrom::Current(off) => (self.position as i64, off),
        };

        let new_pos = if offset.is_negative() {
            base.checked_sub(offset.wrapping_abs() as u64 as i64)
                .ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "Negative seek")
                })?
        } else {
            base.checked_add(offset as u64 as i64).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "Seek overflow")
            })?
        };

        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid seek to negative position",
            ));
        }

        self.position = new_pos as u64;
        Ok(self.position)
    }
}

pub(crate) struct IgniIoWrapper {
    pub(crate) url: String,
    pub(crate) chunk_size: usize,
}

impl vidformer::io::IoWrapper for IgniIoWrapper {
    fn wrap(
        &self,
        r: Box<dyn vidformer::io::ReadSeek>,
        io_namespace: &str,
    ) -> Box<dyn vidformer::io::ReadSeek> {
        Box::new(std::io::BufReader::with_capacity(
            256 * 1024,
            RedisIoCache::new(r, &self.url, io_namespace, self.chunk_size),
        ))
    }
}
