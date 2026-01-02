#![allow(dead_code)]

use std::{
    fmt::{self, Display},
    io::{BufRead, Seek},
    marker::PhantomData,
    rc::Rc,
};

use crate::error::AlliumError;

/// Source file loaded entirely into memory for quick seeking
/// TODO: Figure out a scheme for chunking arbitrarily large source files
///
/// Do not implement clone
#[derive(Debug)]
pub struct SourceFile<'a> {
    path: String,

    chars: Rc<[char]>,

    /// Line start indexes
    /// Always sorted (binary search capable)
    idx_lines: Rc<[usize]>,

    _marker: PhantomData<&'a char>,
}

/// Clonable cursor containing information about the location within the attatched source file
#[derive(Debug, Clone)]
pub struct SourceCursor<'a> {
    pos: usize,
    data: &'a char,
    file: &'a SourceFile<'a>,
}

impl<'a> SourceFile<'a> {
    /// Map a utf-8 source-file into memory
    pub fn new<T: BufRead + Seek>(path: String, f: &mut T) -> Result<Self, AlliumError> {
        let mut i = 0usize;

        let mut chars: Vec<char> = Vec::new();
        let mut lines: Vec<usize> = Vec::new();
        let mut line = String::new();

        f.rewind()?;

        // while not eof
        while let 1.. = f.read_line(&mut line)? {
            lines.push(i);
            for c in line.chars() {
                i += 1;
                chars.push(c);
            }

            line.clear();
        }

        Ok(Self {
            path,
            chars: chars.into_boxed_slice().into(),
            idx_lines: lines.into_boxed_slice().into(),
            _marker: PhantomData::default(),
        })
    }

    /// Create a cursor at the specified position
    pub fn cursor(&'a self, i: usize) -> Result<SourceCursor<'a>, AlliumError> {
        match self.chars.get(i) {
            Some(x) => Ok(SourceCursor {
                pos: i,
                data: x,
                file: self,
            }),
            None => Err(AlliumError::InvalidPosition(
                i,
                self.path.clone(),
                self.chars.len(),
            )),
        }
    }

    /// Create a cursor at the first position
    pub fn start(&'a self) -> Result<SourceCursor<'a>, AlliumError> {
        self.cursor(0)
    }

    /// Create a cursor at the last position
    pub fn end(&'a self) -> Result<SourceCursor<'a>, AlliumError> {
        match self.len() {
            0 => Err(AlliumError::Eof),
            x => self.cursor(x - 1),
        }
    }

    /// Create a span from a start index to an end index
    pub fn span(&'a self, start: usize, end: usize) -> Result<SourceSpan<'a>, AlliumError> {
        if start >= end {
            return Err(AlliumError::SpanSize(start, end));
        }

        if end >= self.len() {
            return Err(AlliumError::InvalidPosition(
                end,
                self.path.clone(),
                self.len(),
            ));
        }

        Ok(SourceSpan {
            start: self.cursor(start)?,
            end: self.cursor(end)?,
        })
    }

    /// Get the length of the file in utf-8 scalar values
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Binary search list of line numbers to determine what line pos is located on
    fn search_ln(&self, pos: usize) -> Result<usize, AlliumError> {
        if pos >= self.chars.len() {
            return Err(AlliumError::InvalidPosition(
                pos,
                self.path.clone(),
                self.chars.len(),
            ));
        }

        let mut top = self.idx_lines.len() - 1;
        let mut bot = 0usize;

        loop {
            let mid = (top + bot) / 2;
            match (self.idx_lines.get(mid), self.idx_lines.get(mid + 1)) {
                // current line exactly matches pos
                // return it
                (Some(&line_start), _) if line_start == pos => return Ok(mid),

                // next line exactly matches pos
                // return it
                (_, Some(&next_line_start)) if next_line_start == pos => return Ok(mid + 1),

                // current line starts after pos
                // move left
                (Some(&line_start), _) if line_start > pos => top = mid,

                // next line starts before pos
                // move right
                (_, Some(&next_line_start)) if next_line_start < pos => bot = mid,

                // line_start < pos < next_line_start
                // return current line
                (Some(_), Some(_)) => return Ok(mid),

                // Invalid state
                (_, _) => panic!(
                    "Invalid binary search state: line {mid} did not return a valid character index"
                ),
            }
        }
    }
    
    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn line(&'a self, idx: usize) -> Result<SourceSpan<'a>, AlliumError> {
        match self.idx_lines.get(idx) {
            Some(&c) => {
                self.cursor(c)?
                    .as_span()
                    .grow_until("\n", false, true)
            }
            None => Err(AlliumError::Eof),
        }
    }
}

impl<'a> Display for SourceFile<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl<'a> SourceCursor<'a> {
    /// get the line number associated with this cursor
    pub fn line_of(&self) -> Result<usize, AlliumError> {
        self.file.search_ln(self.pos)
    }

    /// get the next cursor after this one
    pub fn next(&self) -> Result<Self, AlliumError> {
        if self.pos + 1 >= self.file.len() {
            return Err(AlliumError::Eof);
        }
        self.file.cursor(self.pos + 1)
    }

    /// get the character at the cursor
    pub fn to_char(&self) -> char {
        *self.data
    }

    /// create a span of length 1 with this cursor as the start and end
    pub fn as_span(&self) -> SourceSpan<'a> {
        SourceSpan {
            start: self.clone(),
            end: self.clone(),
        }
    }

    /// create a span running from one cursor to another, this function rectifies reversed spans
    pub fn span_to(&self, other: &Self) -> Result<SourceSpan<'a>, AlliumError> {
        if !std::ptr::eq(self.file, other.file) {
            return Err(AlliumError::SpanMismatch(
                self.file.path.clone(),
                other.file.path.clone(),
            ));
        }

        match self.pos.cmp(&other.pos) {
            std::cmp::Ordering::Less => Ok(SourceSpan {
                start: self.clone(),
                end: other.clone(),
            }),
            // works for equals and greater since order doesn't matter for equals
            _ => Ok(SourceSpan {
                start: other.clone(),
                end: self.clone(),
            }),
        }
    }

    /// create a span running from this cursor that runs for n characters
    pub fn span_for(&self, len: usize) -> Result<SourceSpan<'a>, AlliumError> {
        match len {
            0 => Err(AlliumError::ZeroLengthSpan),
            x => Ok(self.file.span(self.pos, self.pos + (x - 1))?),
        }
    }

    /// Get the cursor n positions left of the current cursor
    pub fn seek_left(&self, count: usize) -> Result<Self, AlliumError> {
        match self.pos.checked_sub(count) {
            Some(s) => self.file.cursor(s),
            None => Err(AlliumError::SeekOverflow),
        }
    }

    /// Get the cursor n positions right of the current cursor
    pub fn seek_right(&self, count: usize) -> Result<Self, AlliumError> {
        match self.pos.checked_add(count) {
            Some(s) => self.file.cursor(s),
            None => Err(AlliumError::SeekOverflow),
        }
    }

    pub fn file(&self) -> &'a SourceFile<'a> {
        self.file
    }

    pub fn pos(&self) -> usize {
        self.pos
    }
}

#[derive(Debug, Clone)]
pub struct SourceSpan<'a> {
    start: SourceCursor<'a>,
    end: SourceCursor<'a>,
}

impl<'a> SourceSpan<'a> {
    /// Get the cursor immediately after the end of the span
    pub fn next(&self) -> Result<SourceCursor<'a>, AlliumError> {
        self.end.next()
    }

    /// create a new span that is the combination of two spans, the spans must be overlapping or touching
    pub fn merge(&self, other: &Self) -> Result<Self, AlliumError> {
        let (first, second) = match self.start.pos.cmp(&other.start.pos) {
            std::cmp::Ordering::Less => (self, other),
            _ => (other, self),
        };

        // first fully contains second
        if first.end.pos > second.end.pos {
            return Ok(first.clone());
        }

        // overlapping or touching
        if first.end.pos >= second.start.pos - 1 {
            return Ok(Self {
                start: first.start.clone(),
                end: second.end.clone(),
            });
        }

        // no overlap
        Err(AlliumError::DiscontinuousSpans)
    }

    /// create a new span that is smaller than the source span by {count} positions, moving the
    /// start position right-ward
    pub fn shrink_left(&self, count: usize) -> Result<Self, AlliumError> {
        if count > self.len() {
            return Err(AlliumError::NegativeLengthSpan);
        }

        Ok(Self {
            start: self.start.seek_right(count)?,
            end: self.end.clone(),
        })
    }

    /// create a new span that is smaller than the source span by {count} positions, moving the end
    /// position left-ward
    pub fn shrink_right(&self, count: usize) -> Result<Self, AlliumError> {
        if count > self.len() {
            return Err(AlliumError::NegativeLengthSpan);
        }

        Ok(Self {
            start: self.start.clone(),
            end: self.end.seek_left(count)?,
        })
    }

    /// create a new span that is the same width as the source span, with both start and end
    /// shifted left-ward
    pub fn shift_left(&self, count: usize) -> Result<Self, AlliumError> {
        Ok(Self {
            start: self.start.seek_left(count)?,
            end: self.end.seek_left(count)?,
        })
    }

    /// create a new span that is the same width as the source span, with both start and end
    /// shifted right-ward
    pub fn shift_right(&self, count: usize) -> Result<Self, AlliumError> {
        Ok(Self {
            start: self.start.seek_right(count)?,
            end: self.end.seek_right(count)?,
        })
    }

    /// create a new span that is larger than the source span by {count} positions, moving the
    /// start position left-ward
    pub fn grow_left(&self, count: usize) -> Result<Self, AlliumError> {
        Ok(Self {
            start: self.start.seek_left(count)?,
            end: self.end.clone(),
        })
    }

    /// create a new span that is larger than the source span by {count} positions, moving the end
    /// position right-ward
    pub fn grow_right(&self, count: usize) -> Result<Self, AlliumError> {
        Ok(Self {
            start: self.start.clone(),
            end: self.end.seek_left(count)?,
        })
    }

    /// get the length of the span, in characters
    pub fn len(&self) -> usize {
        self.end.pos - self.start.pos + 1
    }

    /// test whether the span matches the specified string, exactly
    pub fn is_match(&self, test_str: &str) -> bool {
        self.chars().eq(test_str.chars())
    }

    /// create a new span that has its end shifted right-ward until the {stop} pattern is matched
    ///
    /// stop: pattern to match
    /// allow_escape: check for escaped patterns prefixed with '\', only escapes a single
    ///     so patterns such as "//" would be matched in the string "\///"
    /// match_eof: indicates whether EOF serves as a valid end to the span, returned span will end
    ///     at the character just prior to the EOF
    pub fn grow_until(
        &self,
        stop: &str,
        allow_escape: bool,
        match_eof: bool,
    ) -> Result<Self, AlliumError> {
        let stop_len = match stop.chars().count() {
            0 => return Err(AlliumError::ZeroLengthMatch),
            x => x,
        };

        // special handling for if we hit eof immediately and that is valid, e.g. line comment at
        // the end of a file
        let next = match self.next() {
            Ok(c) => c,
            Err(AlliumError::Eof) if match_eof => {
                return self.start.span_to(&self.start.file.end()?);
            }
            Err(e) => return Err(e),
        };

        let mut window = match next.span_for(stop_len) {
            Ok(s) => s,
            Err(AlliumError::Eof) if match_eof => {
                return self.start.span_to(&self.start.file.end()?);
            }
            Err(e) => return Err(e),
        };

        let mut escaping = false;

        loop {
            if escaping {
                escaping = false;
            } else {
                if window.is_match(stop) {
                    return Ok(self.start.span_to(&window.end)?);
                }
                escaping = allow_escape && window.start.to_char() == '\\';
            }
            window = match window.shift_left(1) {
                Ok(s) => s,
                Err(AlliumError::Eof) if match_eof => {
                    return self.start.span_to(&self.start.file.end()?);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// grow the specified span until the end of the block type specified by open and close
    ///     supports arbitrarily nested blocks
    /// open: block open pattern (e.g. "/*", "{", "(", "[")
    /// close: block close pattern
    /// allow_escape, whether to permit single character escaping via '\'
    ///
    /// contents of the source span must exactly match data specified by open
    pub fn grow_until_block_end(
        &self,
        open: &str,
        close: &str,
        allow_escape: bool,
    ) -> Result<Self, AlliumError> {
        let pat_len = match (open.chars().count(), close.chars().count()) {
            (0, _) | (_, 0) => return Err(AlliumError::ZeroLengthMatch),
            (o, c) if o != c => return Err(AlliumError::BlockPatternLengthMismatch),
            (o, _) => o
        };

        if open.chars().eq(close.chars()) {
            return Err(AlliumError::BlockPatternEquivalency)
        }

        if !self.is_match(open) {
            return Err(AlliumError::BadBlockMatch);
        }

        let mut window = self.shift_right(pat_len)?;

        let mut depth = 1;
        let mut escaping = true;

        loop {
            if escaping {
                escaping = false;
            } else {
                if window.is_match(open) {
                    depth += 1;
                } else if window.is_match(close) {
                    depth -= 1;
                }
                escaping = allow_escape && window.start.to_char() == '\\';
            }
 
            if depth == 0 {
                return Ok(Self {
                    start: self.start.clone(),
                    end: window.end,
                });
            }

            window = window.shift_left(1)?;
        }
    }

    /// returns an iterator over the characters in the span
    pub fn chars(&self) -> SourceSpanChars<'a> {
        SourceSpanChars {
            curr: self.start.clone(),
            count_left: self.len(),
        }
    }

    pub fn start(&self) -> SourceCursor<'a> {
        self.start.clone()
    }

    pub fn end(&self) -> SourceCursor<'a> {
        self.end.clone()
    }
}

#[derive(Debug, Clone)]
struct SourceSpanChars<'a> {
    curr: SourceCursor<'a>,
    count_left: usize,
}

impl<'a> Iterator for SourceSpanChars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count_left == 0 {
            return None;
        }
        let c = self.curr.to_char();

        // unwrap here since lifetime guarentees that next exists
        self.curr = self.curr.next().unwrap();

        return Some(c);
    }
}

impl<'a> Display for SourceSpan<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(self.start.pos < self.end.pos);
        assert!(self.end.pos < self.start.file.len());

        let s = self.chars().collect::<String>();

        write!(f, "{}", s)
    }
}
