use std::marker::PhantomData;

use crate::file::{Cursor, File, Span};

/// Represents all possible meanings for a given utf-8 byte and extracts the meaningful bits
///
/// > Do not construct this type directly, instead use `UTF8Byte::From(u8)` to ensure that
/// > variant accurately represents the data attached
enum UTF8Byte {
    OneByte(u8),
    TwoByte(u8),
    ThreeByte(u8),
    FourByte(u8),
    Continuation(u8),
    Invalid(u8),
}

impl From<u8> for UTF8Byte {
    fn from(value: u8) -> Self {
        match value {
            0b0000_0000..=0b0111_1111 => Self::OneByte(value),
            0b1000_0000..=0b1011_1111 => Self::Continuation(value & 0b0011_1111),
            0b1100_0000..=0b1101_1111 => Self::TwoByte(value & 0b0001_1111),
            0b1110_0000..=0b1110_1111 => Self::ThreeByte(value & 0b0000_1111),
            0b1111_0000..=0b1111_0111 => Self::FourByte(value & 0b0000_0111),
            _ => Self::Invalid(value),
        }
    }
}

impl From<UTF8Byte> for u8 {
    fn from(value: UTF8Byte) -> Self {
        match value {
            UTF8Byte::OneByte(v) => v & 0b0111_1111,
            UTF8Byte::TwoByte(v) => (v & 0b0001_1111) | 0b1100_0000,
            UTF8Byte::ThreeByte(v) => (v & 0b0000_1111) | 0b1110_0000,
            UTF8Byte::FourByte(v) => (v & 0b0000_0111) | 0b1000_0000,
            UTF8Byte::Continuation(v) => (v & 0b0011_1111) | 0b1000_0000,
            UTF8Byte::Invalid(v) => v,
        }
    }
}

/// Converts a [`File<'a, u8>`] into a [`File<'a, char>`] by parsing the bytes as utf-8
pub struct UTF8File<'a, F: File<'a, Item = u8> + 'a> {
    // byte file
    inner: F,

    _marker: PhantomData<&'a char>,
}

pub struct UTF8Cursor<'a, F: File<'a, Item = u8> + 'a> {
    file: &'a UTF8File<'a, F>,
    inner: F::Cursor,
}

pub struct UTF8Span<'a, F: File<'a, Item = u8> + 'a> {
    file: &'a UTF8File<'a, F>,
    start: UTF8Cursor<'a, F>,
    end: UTF8Cursor<'a, F>,
}

impl<'a, F: File<'a, Item = u8>> PartialEq for UTF8Cursor<'a, F> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.inner == other.inner
    }
}

impl<'a, F: File<'a, Item = u8>> Eq for UTF8Cursor<'a, F> {}

impl<'a, F: File<'a, Item = u8>> PartialOrd for UTF8Cursor<'a, F> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if std::ptr::eq(self.file, other.file) {
            self.inner.partial_cmp(&other.inner)
        } else {
            None
        }
    }
}

impl<'a, F: File<'a, Item = u8>> PartialEq for UTF8Span<'a, F> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.start == other.start && self.end == other.end
    }
}

impl<'a, F: File<'a, Item = u8>> Eq for UTF8Span<'a, F> {}

impl<'a, F: File<'a, Item = u8>> Clone for UTF8Cursor<'a, F> {
    fn clone(&self) -> Self {
        Self {
            file: self.file,
            inner: self.inner.clone(),
        }
    }
}

impl<'a, F: File<'a, Item = u8>> Clone for UTF8Span<'a, F> {
    fn clone(&self) -> Self {
        Self {
            file: self.file,
            start: self.start.clone(),
            end: self.end.clone(),
        }
    }
}

impl<'a, F: File<'a, Item = u8>> From<F> for UTF8File<'a, F> {
    fn from(value: F) -> Self {
        UTF8File {
            inner: value,
            _marker: PhantomData,
        }
    }
}

impl<'a, F: File<'a, Item = u8>> std::fmt::Debug for UTF8File<'a, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File").finish_non_exhaustive()
    }
}

impl<'a, F: File<'a, Item = u8>> File<'a> for UTF8File<'a, F> {
    type Item = char;
    type Cursor = UTF8Cursor<'a, F>;

    fn start(&'a self) -> anyhow::Result<Option<Self::Cursor>> {
        if let Some(inner) = self.inner.start()? {
            let start = Self::Cursor { file: self, inner };

            // skip utf-8 Byte Order Mark if it exists
            match self.deref(&start)? {
                (Some(n), '\u{FEFF}') => Ok(Some(Self::Cursor {
                    file: self,
                    inner: n,
                })),
                (None, '\u{FEFF}') => Ok(None),
                (_, _) => Ok(Some(start)),
            }
        } else {
            Ok(None)
        }
    }
}

impl<'a, F: File<'a, Item = u8>> Cursor<'a> for UTF8Cursor<'a, F> {
    type Item = char;
    type Span = UTF8Span<'a, F>;

    fn data(&self) -> anyhow::Result<Self::Item> {
        self.file.deref(self).map(|(_, c)| c)
    }

    fn next(&self) -> anyhow::Result<Option<Self>> {
        self.file.deref(self).map(|(n, _)| {
            n.map(|inner| Self {
                file: self.file,
                inner,
            })
        })
    }

    fn span_to(&self, other: &Self) -> anyhow::Result<Self::Span> {
        anyhow::ensure!(
            std::ptr::eq(self.file, other.file),
            "Failed to create UTF8Span: Cursors refer to two different files"
        );

        anyhow::ensure!(
            self <= other,
            "Failed to create UTF8Span: Length would be negative"
        );

        Ok(Self::Span {
            file: self.file,
            start: self.clone(),
            end: other.clone(),
        })
    }
}

struct UTF8SpanIterator<'a, F: File<'a, Item = u8>> {
    file: &'a UTF8File<'a, F>,
    curr: UTF8Cursor<'a, F>,
    end: UTF8Cursor<'a, F>,
}

impl<'a, F: File<'a, Item = u8>> Iterator for UTF8SpanIterator<'a, F> {
    type Item = anyhow::Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr < self.end {
            let data = self.curr.data();
            if let Err(e) = data {
                return Some(Err(e));
            }
            let data = data.unwrap();
            self.curr = match self.curr.next() {
                Ok(Some(c)) => c,
                Ok(None) => {
                    return Some(Err(anyhow::anyhow!(
                        "UTF8SpanIterator: Encountered <eof> while iterating span"
                    )));
                }
                Err(e) => return Some(Err(e)),
            };
            Some(Ok(data))
        } else {
            None
        }
    }
}

impl<'a, F: File<'a, Item = u8>> Span<'a> for UTF8Span<'a, F> {
    type Item = char;

    fn data(&self) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Self::Item>>> {
        Ok(UTF8SpanIterator {
            file: self.file,
            curr: self.start.clone(),
            end: self.end.clone(),
        })
    }

    fn len(&self) -> anyhow::Result<usize> {
        let mut l = 0usize;

        for r in self.data()? {
            _ = r?;
            l += 1;
        }

        Ok(l)
    }
}

impl<'a, F: File<'a, Item = u8>> UTF8File<'a, F> {
    fn deref(&self, cursor: &UTF8Cursor<'a, F>) -> anyhow::Result<(Option<F::Cursor>, char)> {
        anyhow::ensure!(
            std::ptr::eq(self, cursor.file),
            "Cursor does not refer to self"
        );

        let mut head = cursor.inner.clone();

        let (length, mut val) = match UTF8Byte::from(head.data()?) {
            UTF8Byte::OneByte(v) => (1, v as u32),
            UTF8Byte::TwoByte(v) => (2, v as u32),
            UTF8Byte::ThreeByte(v) => (3, v as u32),
            UTF8Byte::FourByte(v) => (4, v as u32),
            _ => {
                return Err(anyhow::anyhow!(
                    "Cursor does not refer to a valid utf-8 start byte"
                ));
            }
        };

        for _ in 1..length {
            head = match head.next()? {
                Some(c) => c,
                None => return Err(anyhow::anyhow!("Reached <eof> while parsing utf-8 char")),
            };

            if let UTF8Byte::Continuation(v) = UTF8Byte::from(head.data()?) {
                val <<= 6;
                val |= v as u32;
            } else {
                return Err(anyhow::anyhow!(
                    "Cursor referred to a valid utf-8 start byte, but proceeding byte was not a continuation"
                ));
            }
        }

        let next = head.next()?;

        char::from_u32(val).ok_or_else(|| {
            anyhow::anyhow!("Cursor referred to a valid code-point, but it was a surrogate value ({val:#04X})")
        })
            .map(|x| (next, x))
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use crate::{
        CachedReadFile,
        file::{Cursor, File},
        utf8_file::UTF8File,
    };

    #[test]
    fn file_handles_all_valid_utf8() {
        let range = '\0'..'\u{10FFFF}';
        let string = range.collect::<String>();
        let bytes = string.bytes();
        let memory = bytes.collect::<Vec<u8>>();
        let read = std::io::Cursor::new(memory);
        let byte_file = CachedReadFile::from(read);
        let utf8_file = UTF8File::from(byte_file);

        let mut cursor = utf8_file
            .start()
            .expect("Failed to get first cursor in utf8 file")
            .expect("Found <eof> at start of utf8 file");

        for c in string.chars() {
            let data = cursor.data().expect("Failed to get data at cursor");
            assert!(c == data, "{c:?} !== {data:?}");

            cursor = match cursor.next().expect("Failed to get next cursor in file") {
                Some(c) => c,
                None => break,
            }
        }

        assert!(
            cursor.next().unwrap().is_none(),
            "Chars ended, but not at end of file"
        );
    }

    #[test]
    fn file_errors_invalid_utf8() {
        let bytes = 0xF0..0xFFu8;
        let memory = bytes.collect::<Vec<u8>>();
        let read = std::io::Cursor::new(memory);
        let byte_file = CachedReadFile::from(read);
        let utf8_file = UTF8File::from(byte_file);

        assert!(utf8_file.start().is_err())
    }

    #[test]
    fn file_skips_utf8_bom() {
        let string = "\u{FEFF}Hello world";
        let bytes = string.bytes();
        let memory = bytes.collect::<Vec<u8>>();
        let read = std::io::Cursor::new(memory);
        let byte_file = CachedReadFile::from(read);
        let utf8_file = UTF8File::from(byte_file);

        let mut cursor = utf8_file
            .start()
            .expect("Failed to get first cursor in utf8 file")
            .expect("Found <eof> at start of utf8 file");

        for c in string.chars().skip(1) {
            let data = cursor.data().expect("Failed to get data at cursor");
            assert!(c == data, "{c:?} !== {data:?}");

            cursor = match cursor.next().expect("Failed to get next cursor in file") {
                Some(c) => c,
                None => break,
            }
        }

        assert!(
            cursor.next().unwrap().is_none(),
            "Chars ended, but not at end of file"
        );
    }
}
