use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::cursor::{Cursor, Seek};

pub struct CacheFile<C: Cursor> {
    data: Arc<Mutex<Vec<C::Item>>>,
    head: Arc<Mutex<Option<C>>>,
}

pub struct CacheCursor<'a, C: Cursor> {
    file: &'a CacheFile<C>,
    pos: usize,
}

impl<C: Cursor> CacheFile<C>
where
    C::Item: Clone,
{
    pub fn head<'a>(&'a self) -> anyhow::Result<Option<CacheCursor<'a, C>>> {
        if self.ensure_len(1)? {
            Ok(Some(CacheCursor { file: self, pos: 0 }))
        } else {
            Ok(None)
        }
    }
}

impl<'a, C: Cursor> Clone for CacheCursor<'a, C> {
    fn clone(&self) -> Self {
        Self {
            file: self.file,
            pos: self.pos,
        }
    }
}

impl<'a, C: Cursor> PartialEq for CacheCursor<'a, C> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.pos == other.pos
    }
}

impl<'a, C: Cursor> Eq for CacheCursor<'a, C> {}

impl<'a, C: Cursor> PartialOrd for CacheCursor<'a, C> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if std::ptr::eq(self.file, other.file) {
            self.pos.partial_cmp(&other.pos)
        } else {
            None
        }
    }
}

impl<'a, C: Cursor> Cursor for CacheCursor<'a, C>
where
    C::Item: Clone,
{
    type Item = C::Item;

    fn data(&self) -> anyhow::Result<Self::Item> {
        match self.file.ensure_len(self.pos + 1) {
            Ok(true) => Ok(self
                .file
                .data
                .lock()
                .expect("Failed to get guard")
                .get(self.pos)
                .unwrap()
                .clone()),
            Ok(false) => Err(anyhow::anyhow!("Failed to get data at cursor: found <eof>")),
            Err(e) => Err(e),
        }
    }

    fn seek(&self, op: Seek) -> anyhow::Result<Option<Self>> {
        let new_pos = match op {
            Seek::Left(x) if x <= self.pos => self.pos - x,
            Seek::Left(_) => return Ok(None),
            Seek::Right(x) => self.pos.checked_add(x).ok_or_else(|| {
                anyhow::anyhow!(
                    "Cannot apply {op:?} to cursor - Operation would result in overflow"
                )
            })?,
        };

        match self.file.ensure_len(new_pos + 1) {
            Ok(true) => Ok(Some(Self {
                file: self.file,
                pos: new_pos,
            })),
            Ok(false) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl<F: Cursor> CacheFile<F> {
    fn ensure_len(&self, len: usize) -> anyhow::Result<bool> {
        let mut data = self.data.lock().expect("Failed to get guard");

        let mut maybe_head = self.head.lock().expect("Failed to get guard");

        while data.len() < len
            && let Some(head) = maybe_head.clone()
        {
            data.push(head.data()?);
            *maybe_head = head.seek(Seek::Right(1))?;
        }

        return Ok(data.len() >= len);
    }
}
