use crate::source::SourceCursor;

pub struct Continuation {

}

pub struct Punctuation<'a> {
    char: char,
    cursor: SourceCursor<'a>
}

