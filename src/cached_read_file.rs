use std::{
    io::Read,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

use crate::file::*;

/// Represents any stream of bytes as a random access collection of characters.
///
/// Reads and caches data lazily in chunks of 4KiB (or less)
pub struct CachedReadFile<'a, R: Read + 'a> {
    inner: Arc<Mutex<R>>,
    data: Arc<Mutex<Vec<u8>>>,
    _marker: PhantomData<&'a u8>,
}

pub struct CachedReadCursor<'a, R: Read + 'a> {
    file: &'a CachedReadFile<'a, R>,
    pos: usize,
}

pub struct CachedReadSpan<'a, R: Read + 'a> {
    file: &'a CachedReadFile<'a, R>,
    pos: usize,
    end: usize,
}

impl<'a, R: Read + 'a> Clone for CachedReadCursor<'a, R> {
    fn clone(&self) -> Self {
        Self {
            file: self.file,
            pos: self.pos,
        }
    }
}

impl<'a, R: Read + 'a> Clone for CachedReadSpan<'a, R> {
    fn clone(&self) -> Self {
        Self {
            file: self.file,
            pos: self.pos,
            end: self.end,
        }
    }
}

impl<'a, R: Read + 'a> PartialEq for CachedReadCursor<'a, R> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.pos == other.pos
    }
}

impl<'a, R: Read + 'a> Eq for CachedReadCursor<'a, R> {}

impl<'a, R: Read + 'a> PartialOrd for CachedReadCursor<'a, R> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if !std::ptr::eq(self.file, other.file) {
            None
        } else {
            self.pos.partial_cmp(&other.pos)
        }
    }
}

impl<'a, R: Read + 'a> PartialEq for CachedReadSpan<'a, R> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.pos == other.pos && self.end == other.end
    }
}

impl<'a, R: Read + 'a> Eq for CachedReadSpan<'a, R> {}

impl<'a, R: Read + 'a> From<R> for CachedReadFile<'a, R> {
    fn from(value: R) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
            data: Arc::new(Mutex::new(Vec::new())),
            _marker: PhantomData,
        }
    }
}

impl<'a, R: Read + 'a> File<'a> for CachedReadFile<'a, R> {
    type Item = u8;
    type Cursor = CachedReadCursor<'a, R>;

    fn start(&'a self) -> anyhow::Result<Option<Self::Cursor>> {
        if self.ensure_len(1)? {
            Ok(Some(CachedReadCursor { file: self, pos: 0 }))
        } else {
            Ok(None)
        }
    }
}

impl<'a, R: Read> Cursor<'a> for CachedReadCursor<'a, R> {
    type Item = u8;
    type Span = CachedReadSpan<'a, R>;

    fn data(&self) -> anyhow::Result<Self::Item> {
        anyhow::ensure!(
            self.file.ensure_len(self.pos + 1)?,
            "{self:?} refers to invalid memory in {:?}",
            self.file
        );
        Ok(self.file.get(self.pos).unwrap())
    }

    fn next(&self) -> anyhow::Result<Option<Self>> {
        if self.file.ensure_len(self.pos + 2)? {
            Ok(Some(CachedReadCursor {
                file: self.file,
                pos: self.pos + 1,
            }))
        } else {
            Ok(None)
        }
    }

    fn span_to(&self, other: &Self) -> anyhow::Result<Self::Span> {
        if self.pos <= other.pos {
            if self.file.ensure_len(other.pos + 1)? {
                Ok(CachedReadSpan {
                    file: self.file,
                    pos: self.pos,
                    end: other.pos + 1,
                })
            } else {
                Err(anyhow::anyhow!(
                    "Cannot create span from {self:?} to {other:?}. Reached <eof>."
                ))
            }
        } else {
            Err(anyhow::anyhow!(
                "Cannot create span from {self:?} to {other:?}. Length would be negative"
            ))
        }
    }
}

struct SpanIterator<'a, R: Read> {
    file: &'a CachedReadFile<'a, R>,
    pos: usize,
    end: usize,
}

impl<'a, R: Read> Iterator for SpanIterator<'a, R> {
    type Item = anyhow::Result<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.end {
            match self.file.ensure_len(self.pos + 1) {
                Ok(true) => {
                    let res = self.file.get(self.pos).unwrap();
                    self.pos += 1;
                    Some(Ok(res))
                }
                Ok(false) => Some(Err(anyhow::anyhow!("Reached <eof> before end of span"))),
                Err(e) => Some(Err(e)),
            }
        } else {
            None
        }
    }
}

impl<'a, R: Read> Span<'a> for CachedReadSpan<'a, R> {
    type Item = u8;

    fn data(&self) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Self::Item>>> {
        Ok(SpanIterator {
            file: self.file,
            pos: self.pos,
            end: self.end,
        })
    }

    fn len(&self) -> anyhow::Result<usize> {
        Ok(self.end - self.pos)
    }
}

impl<'a, R: Read> std::fmt::Debug for CachedReadFile<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

impl<'a, R: Read> std::fmt::Debug for CachedReadCursor<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedByteCursor")
            .field("file", &self.file)
            .field("pos", &self.pos)
            .finish()
    }
}

impl<'a, R: Read> std::fmt::Debug for CachedReadSpan<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CachedByteSpan")
            .field("file", &self.file)
            .field("pos", &self.pos)
            .field("end", &self.end)
            .finish()
    }
}

/// Functions implemented here acquire mutexes and thus should be self contained.
///
/// Do not depend on any other `File::` functions to ensure that mutexes do not panic when
/// attempting to reacquire the lock
impl<'a, R: Read> CachedReadFile<'a, R> {
    /// get the length, in bytes, currently loaded into the internal buffer.
    ///
    /// the the `Read` may still contain more bytes
    fn len(&self) -> usize {
        self.data.lock().unwrap().len()
    }

    /// returns a bool indicating whether the available length is at least the value specified by
    /// `len`, attempting to expand the internal buffer in 4kB chunks until `len` is reached
    fn ensure_len(&self, len: usize) -> anyhow::Result<bool> {
        let mut inner = self.inner.lock().unwrap();

        let mut data = self.data.lock().unwrap();

        while len > data.len() {
            let mut bytes = [0u8; 4096];
            let bytes_read = inner.read(&mut bytes)?;
            if bytes_read == 0 {
                break;
            }
            data.extend_from_slice(&bytes[..bytes_read]);
        }

        Ok(data.len() >= len)
    }

    fn get(&self, idx: usize) -> Option<u8> {
        self.data.lock().unwrap().get(idx).copied()
    }
}

#[cfg(test)]
mod test {
    use crate::{
        cached_read_file::CachedReadFile,
        file::{Cursor, File},
    };

    #[test]
    fn output_is_correct() {
        let range = 0..=0xFFu8;
        let memory = range.collect::<Vec<u8>>();
        let read = std::io::Cursor::new(memory);
        let file = CachedReadFile::from(read);
        let mut head = file
            .start()
            .expect("Error getting first value")
            .expect("Found <eof>");

        for c in 0..0xFFu8 {
            assert!(c == head.data().expect("File is missing data"));

            head = head
                .next()
                .expect("Error getting value")
                .expect("Found <eof>");
        }

        assert!(
            head.next().expect("Error getting last value").is_none(),
            "Did not find <eof>"
        )
    }

    #[test]
    fn retraverses() {
        let range = 0..=0x10u8;
        let memory = range.collect::<Vec<u8>>();
        let read = std::io::Cursor::new(memory);
        let file = CachedReadFile::from(read);
        let mut head = file
            .start()
            .expect("Error getting start value")
            .expect("Found <eof>");

        for c in 0..0x10u8 {
            assert!(c == head.data().expect("File is missing data"));

            head = head
                .next()
                .expect("Error getting value")
                .expect("Found <eof>");
        }

        let last = head;

        head = file
            .start()
            .expect("Error getting start value a second time")
            .expect("Found <eof> at second start");

        for c in 0..0x10u8 {
            assert!(c == head.data().expect("File is missing data"));

            head = head
                .next()
                .expect("Error getting value")
                .expect("Found <eof>");
        }

        assert!(last == head, "Ends were not equal");

        assert!(
            head.next()
                .expect("Error getting first last value")
                .is_none(),
            "Did not find <eof> at first end"
        );

        assert!(
            last.next()
                .expect("Error getting first last value")
                .is_none(),
            "Did not find <eof> at second end"
        );
    }

    #[test]
    fn equality() {
        let range = 0..=0x10u8;
        let memory = range.collect::<Vec<u8>>();
        let read = std::io::Cursor::new(memory);
        let file = CachedReadFile::from(read);
        let mut head = file
            .start()
            .expect("Error getting start value")
            .expect("Found <eof>");

        let mut cursors = vec![head.clone()];
        let mut spans = vec![head.span_to(&head).expect("Failed to create span to self")];

        for c in 0..0x10u8 {
            assert!(c == head.data().expect("File is missing data"));

            head = head
                .next()
                .expect("Error getting value")
                .expect("Found <eof>");

            assert!(head == head);
            assert!(cursors.iter().all(|past| past != &head));
            cursors.push(head.clone());

            for span in cursors.iter().map(|past| past.span_to(&head).unwrap()) {
                spans.push(span);
            }
        }

        for span in spans.iter() {
            assert!(
                spans.iter().filter(|other| &span == other).count() == 1,
                "More than one span was equivalent"
            );
        }
    }
}
