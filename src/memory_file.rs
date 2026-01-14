use std::{cmp::Ordering, marker::PhantomData};

use crate::cursor::{Cursor, Seek};

/// Exposes a given slice as a [`File`]
pub struct MemoryFile<'a, T> {
    inner: &'a [T],
}

impl<'a, T> MemoryFile<'a, T> {
    pub fn new(data: &'a [T]) -> Self {
        Self { inner: data }
    }
}

struct MemoryCursor<'a, T> {
    file: &'a MemoryFile<'a, T>,
    pos: usize,
}

impl<'a, T> PartialEq for MemoryCursor<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.pos == other.pos
    }
}

impl<'a, T> Eq for MemoryCursor<'a, T> {}

impl<'a, T> PartialOrd for MemoryCursor<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if std::ptr::eq(self.file, other.file) {
            self.pos.partial_cmp(&other.pos)
        } else {
            None
        }
    }
}

impl<'a, T> Clone for MemoryCursor<'a, T> {
    fn clone(&self) -> Self {
        Self {
            file: self.file,
            pos: self.pos,
        }
    }
}

impl<'a, T: Clone> MemoryFile<'a, T> {
    pub fn head(&'a self) -> anyhow::Result<Option<impl Cursor<Item = T>>> {
        if self.inner.is_empty() {
            Ok(None)
        } else {
            Ok(Some(MemoryCursor { file: self, pos: 0 }))
        }
    }
}

impl<'a, T: Clone> Cursor for MemoryCursor<'a, T> {
    type Item = T;

    fn data(&self) -> anyhow::Result<Self::Item> {
        self.file.inner.get(self.pos).cloned().ok_or_else(|| {
            anyhow::anyhow!("Failed to get data associated with cursor at {}", self.pos)
        })
    }

    fn seek(&self, op: Seek) -> anyhow::Result<Option<Self>> {
        if let Seek::Left(x) = op {
            if x > self.pos {
                Ok(None)
            } else {
                Ok(Some(MemoryCursor {
                    file: self.file,
                    pos: self.pos - x,
                }))
            }
        } else if let Seek::Right(x) = op {
            let new_pos = self.pos.checked_add(x).ok_or_else(|| {
                anyhow::anyhow!(
                    "Failed to apply {op:?} to cursor at {}, operation would result in overflow",
                    self.pos
                )
            })?;

            if self.file.inner.len() > new_pos {
                Ok(Some(MemoryCursor {
                    file: self.file,
                    pos: new_pos,
                }))
            } else {
                Ok(None)
            }
        } else {
            panic!("Invalid seek operation: {op:?}")
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        cursor::{Cursor, Seek},
        memory_file::MemoryFile,
    };

    #[test]
    fn test_works() {
        let v = [0, 1, 2, 3, 4, 5];
        let f = MemoryFile::new(v.as_slice());
        let mut head = f.head().expect("Failed to get head of memoryfile");

        let mut i = 0;
        while let Some(c) = &head {
            let data = c.data().unwrap();
            assert!(i == data);
            i += 1;
            head = c.seek(Seek::Right(1)).expect("Failed to seek right");
        }
    }
}
