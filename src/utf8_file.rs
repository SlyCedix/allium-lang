use std::marker::PhantomData;

use crate::cursor::{Cursor, Seek};

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
            UTF8Byte::FourByte(v) => (v & 0b0000_0111) | 0b1111_0000,
            UTF8Byte::Continuation(v) => (v & 0b0011_1111) | 0b1000_0000,
            UTF8Byte::Invalid(v) => v,
        }
    }
}

pub struct UTF8Cursor<C> {
    inner: C,
}

impl<C: Clone> Clone for UTF8Cursor<C> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<C: Cursor<Item = u8>> UTF8Cursor<C> {
    pub fn convert(inner: C) -> anyhow::Result<Option<impl Cursor<Item = char>>> {
        if let (next, '\u{FEFF}') = Self::deref(&inner)? {
            Ok(next)
        } else {
            Ok(Some(Self { inner }))
        }
    }

    fn deref(inner: &C) -> anyhow::Result<(Option<Self>, char)> {
        let mut head = inner.clone();

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
        let c = char::from_u32(val).ok_or_else(|| {
            anyhow::anyhow!(
                "Cursor referred to a valid code-point, but it was a surrogate value ({val:#04X})"
            )
        })?;

        Ok((head.next()?.map(|inner| Self { inner }), c))
    }
}

impl<C: Cursor<Item = u8>> Cursor for UTF8Cursor<C> {
    type Item = char;

    fn data(&self) -> anyhow::Result<Self::Item> {
        UTF8Cursor::deref(&self.inner).map(|(_, c)| c)
    }

    fn seek(&self, op: Seek) -> anyhow::Result<Option<Self>> {
        if let Seek::Right(mut x) = op {
            let mut head = self.clone();
            while x > 0 {
                head = match UTF8Cursor::deref(&head.inner)? {
                    (None, _) => return Ok(None),
                    (Some(h), _) => h,
                };
                x -= 1;
            }
            Ok(Some(head))
        } else {
            Err(anyhow::anyhow!(
                "Seek failed: Seek::Left is unsuported by this file"
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use crate::{
        cursor::{Cursor, Seek},
        memory_file::MemoryFile,
        utf8_file::UTF8Cursor,
    };

    #[test]
    fn file_handles_all_valid_utf8() {
        let range = '\0'..'\u{10FFFF}';
        let string = range.collect::<String>();
        let bytes = string.bytes();
        let memory = bytes.collect::<Vec<u8>>();
        let byte_file = MemoryFile::new(memory.as_slice());
        let byte_cursor = byte_file.head().unwrap().unwrap();
        let mut cursor = UTF8Cursor::convert(byte_cursor).unwrap().unwrap();

        for c in string.chars() {
            let data = cursor.data().expect("Failed to get data at cursor");
            assert!(c == data, "{c:?} !== {data:?}");

            cursor = match cursor
                .seek(Seek::Right(1))
                .expect("Failed to get next cursor in file")
            {
                Some(c) => c,
                None => break,
            }
        }

        assert!(
            cursor.seek(Seek::Right(1)).unwrap().is_none(),
            "Chars ended, but not at end of file"
        );
    }

    #[test]
    fn file_errors_invalid_utf8() {
        let bytes = 0xF0..0xFFu8;
        let memory = bytes.collect::<Vec<u8>>();
        let byte_file = MemoryFile::new(memory.as_slice());
        let byte_cursor = byte_file.head().unwrap().unwrap();
        let cursor = UTF8Cursor::convert(byte_cursor);

        assert!(cursor.is_err())
    }

    #[test]
    fn file_skips_utf8_bom() {
        let string = "\u{FEFF}Hello world";
        let bytes = string.bytes();
        let memory = bytes.collect::<Vec<u8>>();
        let byte_file = MemoryFile::new(memory.as_slice());
        let byte_cursor = byte_file.head().unwrap().unwrap();
        let mut cursor = UTF8Cursor::convert(byte_cursor).unwrap().unwrap();

        for c in string.chars().skip(1) {
            let data = cursor.data().expect("Failed to get data at cursor");
            assert!(c == data, "{c:?} !== {data:?}");

            cursor = match cursor
                .seek(Seek::Right(1))
                .expect("Failed to get next cursor in file")
            {
                Some(c) => c,
                None => break,
            }
        }

        assert!(
            cursor.seek(Seek::Right(1)).unwrap().is_none(),
            "Chars ended, but not at end of file"
        );
    }
}
