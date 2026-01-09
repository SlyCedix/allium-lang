use std::{
    io::Read,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::source::{Cursor, unicode::UTF8Byte};

/// Represents any stream of bytes as a random access collection of characters.
///
/// Reads and caches data lazily in chunks of 4KiB (or less)
pub struct File<R: Read> {
    /// The actual object we read bytes from. We use dyn here so that `File` can generically be
    /// built from any u8 stream, for example `stdin`
    inner: Arc<Mutex<R>>,

    /// Storage of character data we've pulled from the
    data: Arc<Mutex<Vec<u8>>>,
    lines: Arc<Mutex<Vec<usize>>>,
}

impl<R: Read> std::fmt::Debug for File<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("File")
            .field("len", &self.len())
            .finish_non_exhaustive()
    }
}

impl<R: Read> File<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner: Arc::new(Mutex::new(inner)),
            data: Arc::new(Mutex::new(Vec::new())),
            lines: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Attempt to open the os file at the specified path and wrap it in a `File`
    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<File<std::fs::File>> {
        Ok(File::new(std::fs::File::open(path)?))
    }

    /// get a cursor associated with the first character of the file, skipping the UTF-8 BOM
    /// character if it exists
    pub fn start<'a>(&'a self) -> anyhow::Result<Cursor<'a, R>> {
        anyhow::ensure!(self.ensure_len(1)?, "{self:?} is empty");
        let c = Cursor::new(self, 0);
        let b = self.char_at(0)?;

        // skip UTF-8 Byte Order Mark(BOM) only at beginning of file
        if b.1 == '\u{FEFF}' { c.next() } else { Ok(c) }
    }

    /// get the byte length and data associated with a given index
    ///
    /// see `Cursor::deref` for publically exposed version of this function
    pub(in crate::source) fn char_at(
        &self,
        idx: usize,
    ) -> anyhow::Result<(usize, char)> {
        anyhow::ensure!(
            self.ensure_len(idx + 1)?,
            "{idx:?} refers to memory not available in {self:?}"
        );

        let mut pos = idx;
        let mut head = self.at(pos).unwrap();

        let (length, mut val) = match UTF8Byte::from(head) {
            UTF8Byte::OneByte(v) => (1, v as u32),
            UTF8Byte::TwoByte(v) => (2, v as u32),
            UTF8Byte::ThreeByte(v) => (3, v as u32),
            UTF8Byte::FourByte(v) => (4, v as u32),
            _ => {
                return Err(anyhow::anyhow!(
                    "{idx:?} did not refer to valid utf-8 character start byte"
                ));
            }
        };

        anyhow::ensure!(
            self.ensure_len(pos + length)?,
            "{idx:?} refers to a valid utf-8 character start byte, but file reached <eof>"
        );

        for _ in 1..length {
            pos += 1;
            head = self.at(pos).unwrap();

            match UTF8Byte::from(head) {
                UTF8Byte::Continuation(v) => {
                    val <<= 6;
                    val |= v as u32;
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "{idx:?} refers to valid utf-8 chracter start byte, but encounted non-continuation byte while reading rest"
                    ));
                }
            };
        }

        char::from_u32(val).ok_or_else(|| {
            anyhow::anyhow!(
                "{idx:?} referred to a valid code-point, but it was a surrogate value ({val:#04X})"
            )
        }).map(|v| (length, v))
    }
}

/// Functions implemented here acquire mutexes and thus should be self contained.
///
/// Do not depend on any other `File::` functions to ensure that mutexes do not panic when
/// attempting to reacquire the lock
impl<R: Read> File<R> {
    /// get the length, in bytes, currently loaded into the internal buffer.
    /// 
    /// the the `Read` may still contain more bytes
    pub fn len(&self) -> usize {
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

    fn at(&self, idx: usize) -> Option<u8> {
        self.data.lock().unwrap().get(idx).copied()
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use crate::source::File;

    struct Readable<I: Iterator<Item = u8>> {
        inner: I,
    }

    impl<I: Iterator<Item = u8>> Readable<I> {
        fn new(s: I) -> Self {
            Self { inner: s }
        }
    }

    impl<I: Iterator<Item = u8>> Read for Readable<I> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let mut len = 0usize;
            for k in buf {
                match self.inner.next() {
                    Some(b) => *k = b,
                    None => break,
                }
                len += 1;
            }

            Ok(len)
        }
    }

    #[test]
    fn file_handles_valid_utf8() {
        let utf8_str = "⅏℁℀ⅽ℣⅏ⅶⅢ⅚ℹℜℙℐℴ⅄ⅽ℧ⅾ℧ℋⅣ℣ⅹ⅑ℽℽↀℨℋⅡℜⅱℋ℮ↆ℠ↃⅅↆⅮℇ℺Ⅱℿℰℯ⅚ℨⅥⅬℯⅿℐ℘℻⅊℔ⅪℚↇⅹℋℨⅣℹ℘↉⅒ⅸⅠK℺Kⅅ℈Ⅽℐⅴ™℟™ℶℾⅾ⅊⅛ℊℳⅺ℃ℱↀℬⅣⅽ℻⅟℞ↄ℩Ⅿⅸ℔⅜Ⅿℇ℗Ⅰ↊⅊ℳↃ℆ℭ℧ℵⅹℽↆÅↈℜℏⅼ℈ↁℊⅇ℘℃⅕Ⅎↁⅿ⅓℠ⅸℼↇⅻ℆Ⅷ℠℡ⅫⅬℊ⅃⅒ⅿↈℭℹℊ⅀ℤⅺ℧ℽ⅏Ⅹ℟№Ⅸⅷℭℐ℘ⅺ⅏Ⅱ⅀⅖ℌ⅘ⅳ⅔ℱ⅗⅍ℷ℻↋ℍ℁⅀Ⅷℛℯ⅓Ⅶℵℱℊↅ⅍ℇⅤ⅗⅑";
        let file = File::new(Readable::new(utf8_str.bytes()));
        let mut cursor = file.start();

        for c in utf8_str.chars() {
            // unwrap here so that we can just fall off on the last char
            let curr = cursor.unwrap();
            let char_at = curr.char().unwrap();

            assert!(c == char_at, "{c:?} != {char_at:?}");
            cursor = curr.next();
        }

        assert!(cursor.is_err(), "Chars ended, but not at end of file");
    }

    #[test]
    fn file_handles_all_utf8() {
        let range = '\0'..'\u{10FFFF}';
        let string = range.collect::<String>();
        let bytes = string.bytes();
        let file = File::new(Readable::new(bytes));
        let mut cursor = file.start();

        for c in string.chars() {
            // unwrap here so that we can just fall off on the last char
            let curr = cursor.unwrap();
            let char_at = curr.char().unwrap();

            assert!(c == char_at, "{c:?} != {char_at:?}");
            cursor = curr.next();
        }

        assert!(cursor.is_err(), "Chars ended, but not at end of file");
    }

    #[test]
    fn file_errors_invalid_utf8() {
        let bytes = 0xF0..0xFFu8;
        let file = File::new(Readable::new(bytes));

        assert!(file.start().is_err())
    }

    #[test]
    fn file_skips_utf8_bom() {
        let string = "\u{FEFF}Hello world";
        let file = File::new(Readable::new(string.bytes()));
        let mut cursor = file.start();

        for c in string.chars().skip(1) {
            let curr = cursor.unwrap();
            let char_at = curr.char().unwrap();

            assert!(c == char_at, "{c:?} != {char_at:?}");
            cursor = curr.next();
        }
        assert!(cursor.is_err());
    }
}
