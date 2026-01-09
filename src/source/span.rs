use std::{fmt::Debug, io::Read};

use crate::source::File;

#[derive(Clone)]
pub struct Span<'a, R: Read> {
    file: &'a File<R>,
    start: usize,
    end: usize,
}

impl<'a, R: Read> PartialEq for Span<'a, R> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.start == other.start && self.end == other.end
    }
}

impl<'a, R: Read> Eq for Span<'a, R> {}

impl<'a, R: Read> Debug for Span<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Span")
            .field("file", &self.file)
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

impl<'a, R: Read> Span<'a, R> {
    pub(in crate::source) fn new(file: &'a File<R>, start: usize, end: usize) -> Self {
        assert!(end >= start);

        Self { file, start, end }
    }

    pub fn byte_len(&self) -> usize {
        self.end - self.start
    }

    pub fn char_len(&self) -> anyhow::Result<usize> {
        let mut pos = self.start;
        let mut count = 0;

        loop {
            let (len, _) = self.file.char_at(pos)?;
            pos += len;
            count += 1;
            if pos >= self.end {
                return Ok(count);
            }
        }
    }

    pub fn chars(&self) -> impl Iterator<Item = anyhow::Result<char>> {
        Chars {
            file: self.file,
            curr: self.start,
            end: self.end,
        }
    }
}

struct Chars<'a, R: Read> {
    file: &'a File<R>,
    curr: usize,
    end: usize,
}

impl<'a, R: Read> Iterator for Chars<'a, R> {
    type Item = anyhow::Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr < self.end {
            Some(self.file.char_at(self.curr).map(|(len, c)| {
                self.curr += len;
                c
            }))
        } else {
            None
        }
    }
}
