use std::{cmp::Ordering, io::Read};

use anyhow::Context;

use crate::source::{File, Span};

#[derive(Clone)]
pub struct Cursor<'a, R: Read> {
    file: &'a File<R>,
    pos: usize,
}

impl<'a, R: Read> std::fmt::Debug for Cursor<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cursor")
            .field("file", self.file)
            .field("pos", &self.pos)
            .finish()
    }
}

impl<'a, R: Read> PartialOrd for Cursor<'a, R> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if !std::ptr::eq(self, other) {
            None
        } else {
            self.pos.partial_cmp(&other.pos)
        }
    }
}

impl<'a, R: Read> PartialEq for Cursor<'a, R> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.file, other.file) && self.pos == other.pos
    }
}

impl<'a, R: Read> Eq for Cursor<'a, R> {}

impl<'a, R: Read> Cursor<'a, R> {
    /// internal unchecked cursor constructor, use `File::start` to retrieve a cursor for use in crate
    /// consumer
    pub(in crate::source) fn new(file: &'a File<R>, idx: usize) -> Self {
        Self { pos: idx, file }
    }

    /// get the position of this cursor
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// get the file associated with this cursor
    pub fn file(&self) -> &'a File<R> {
        self.file
    }

    /// get the cursor immediately following this one
    ///
    /// next cursor is not guarenteed to refer to a valid position in the file
    pub fn next(&self) -> anyhow::Result<Self> {
        let b = self.deref()?;
        let c = Cursor::new(self.file, self.pos + b.0);

        Ok(c)
    }

    /// get char associated with the cursor
    pub fn char(&self) -> anyhow::Result<char> {
        Ok(self.deref()?.1)
    }

    /// Get byte length and char associated with cursor
    pub fn deref(&self) -> anyhow::Result<(usize, char)> {
        self.file.char_at(self.pos)
    }

    /// create a new span between this cursor and another in the same file.
    ///
    /// ordering will be rectified such that the span length is positive
    ///
    /// calling `self.span_to(self)` will result in a span with char length 1, referring to the
    /// bytes associated with this char only
    pub fn span_to(&self, other: &Cursor<'a, R>) -> anyhow::Result<Span<'a, R>> {
        let (first, second) = match self.partial_cmp(other) {
            Some(Ordering::Less) => (self, other),
            Some(Ordering::Equal) => (self, other),
            Some(Ordering::Greater) => (other, self),
            None => {
                return Err(anyhow::anyhow!(
                    "Cannot create a span between {self:?} and {other:?}: They refer to two different files"
                ));
            }
        };

        _ = first.deref().context("Invalid left bound of span")?;

        let end = second.deref().context("Invalid right bound of span")?.0;

        Ok(Span::new(self.file, first.pos, second.pos + end - 1))
    }
}
