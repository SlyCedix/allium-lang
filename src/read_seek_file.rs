use std::{
    io::{ErrorKind, Read, Seek, SeekFrom},
    sync::{Arc, Mutex},
};

use crate::cursor::{self, Cursor};

/// Adapts an object implementing [`Read`] and [`Seek`] as a [`File`] without caching.
///
/// Errors produced by calls into the inner object will result
pub struct ReadSeekFile<R: Read + Seek> {
    inner: Arc<Mutex<R>>,
}

pub struct ReadSeekCursor<'a, R: Read + Seek> {
    file: &'a ReadSeekFile<R>,
    pos: usize,
}

impl<R: Read + Seek> From<R> for ReadSeekFile<R> {
    fn from(value: R) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
        }
    }
}

impl<'a, R: Read + Seek + 'a> ReadSeekFile<R> {
    pub fn start(&'a self) -> anyhow::Result<Option<impl Cursor<Item = u8>>> {
        let mut inner = self.inner.lock().expect("Failed to acquire lock");
        match inner.seek(SeekFrom::Start(0)) {
            Ok(_) => Ok(Some(ReadSeekCursor { file: self, pos: 0 })),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}

impl<'a, R: Read + Seek + 'a> Clone for ReadSeekCursor<'a, R> {
    fn clone(&self) -> Self {
        Self {
            file: self.file,
            pos: self.pos,
        }
    }
}

impl<'a, R: Read + Seek + 'a> Cursor for ReadSeekCursor<'a, R> {
    type Item = u8;

    fn data(&self) -> anyhow::Result<Self::Item> {
        let mut inner = self.file.inner.lock().expect("Failed to acquire lock");
        inner.seek(SeekFrom::Start(self.pos as u64))?;
        let mut data = [0u8];
        inner.read_exact(&mut data)?;
        Ok(data[0])
    }

    fn seek(&self, op: cursor::Seek) -> anyhow::Result<Option<Self>> {
        let new_pos = match op {
            cursor::Seek::Left(x) if x <= self.pos => self.pos - x,
            cursor::Seek::Left(_) => return Ok(None),
            cursor::Seek::Right(x) => self.pos.checked_add(x).ok_or_else(|| {
                anyhow::anyhow!("Failed to apply {op:?} - Opretion would result in overflow")
            })?,
        };

        match self
            .file
            .inner
            .lock()
            .expect("Failed to acquire lock")
            .seek(SeekFrom::Start(new_pos as u64))
        {
            Ok(_) => Ok(Some(Self {
                file: self.file,
                pos: new_pos,
            })),
            Err(e) if e.kind() == ErrorKind::UnexpectedEof => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
