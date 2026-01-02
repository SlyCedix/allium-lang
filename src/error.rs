use std::io::{self};

use thiserror::Error;

use crate::source::SourceCursor;

#[derive(Debug, Error)]
pub enum AlliumError {
    #[error("{0}: ")]
    Io(#[from] io::Error),

    #[error("Invalid position {0} for SourceFile({1}) of length {2}")]
    InvalidPosition(usize, String, usize),

    #[error("Cannot create span from SourceFile({0}) and SourceFile({1}), are they the same file?")]
    SpanMismatch(String, String),

    #[error(
        "Cannot create span between SourceCursor({0}) and SourceCursor({1}), spans must be of positive length"
    )]
    SpanSize(usize, usize),

    #[error("Cannot create a negative length span")]
    NegativeLengthSpan,

    #[error("Cannot create a zero length span")]
    ZeroLengthSpan,

    #[error("Cannot merge two spans which do not overlap")]
    DiscontinuousSpans,

    #[error("Cannot create block from span, span does not begin with block start")]
    BadBlockMatch,

    #[error("Cannot match a zero-length string")]
    ZeroLengthMatch,

    #[error("Cannot create a block span from patterns with mismatching length")]
    BlockPatternLengthMismatch,

    #[error("Cannot create a block span from identical open and close patterns")]
    BlockPatternEquivalency,

    #[error("Cannot execute seek, would overflow position")]
    SeekOverflow,

    #[error("Failed to parse an atom from token at {0}")]
    NoAtom,

    #[error("Invalid state reached: {0}")]
    Other(String),

    #[error("Reached the end of the file")]
    Eof,
}

/// maximum line length to show in the terminal, longer lines are truncated and centered
/// TODO: make this a command line argument
/// TODO: make this dynamic based on terminal environment width
const MAX_VIEW_WINDOW: usize = 80;

/// use ansi color for generating pretty error messages
/// TODO: make this a command line argument
/// TODO: make this a cli argument
const USE_ANSI_COLOR: bool = true;

/// cloneable struct carrying error information to be printed to the terminal
///
/// should carry all the necessary 
#[derive(Debug, Clone)]
pub struct ErrorCursor {
    /// Message to render before pretty printed file location 
    /// Newline will be automatically appended
    pre: Option<String>,
    
    /// Message to render after pretty printed file location
    /// Newline will be automatically appended
    post: Option<String>,

    /// Path to the file as specified by the `SourceFile` this cursor was created from
    path: String,

    /// Text of the line this cursor is on
    line: String,

    /// Virtual position in view window
    virt_pos: usize,

    /// Line number to append after filename
    line_num: usize,

    /// Position in line to append after line_num
    line_pos: usize,
}

impl ErrorCursor {
    /// create an unbound ErrorCursor from a lifetime bound SourceCursor
    /// may panic on error collecting information
    pub fn new<'a>(cursor: &'a SourceCursor<'a>, pre: Option<String>, post: Option<String>) -> Self {
        let path = cursor
            .file()
            .path();

        let line_num = cursor
            .line_of().expect("Error getting line number from cursor");

        let line_span = cursor
            .file()
            .line(line_num).expect("Error getting span associated with cursor line");

        let mut line_start = line_span.start();

        let line_pos = cursor.pos() - line_start.pos();

        /// line is smaller than "error view"
        if line_span.len() <= MAX_VIEW_WINDOW {
            return Self {
                pre,
                post,
                path,
                line: line_span.to_string(),
                virt_pos: line_pos,
                line_num,
                line_pos
            };
        }
        
        

        if line_pos > MAX_VIEW_WINDOW / 2 {
            line_start = cursor
                .seek_left(MAX_VIEW_WINDOW).expect("Error getting adjusted window start cursor");
        }

        

    }
}


