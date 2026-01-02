#![allow(dead_code)]

use crate::{error::AlliumError, source::{SourceCursor, SourceSpan}};

/// Represents a single character with special meaning
#[derive(Debug, Clone)]
pub struct Punct<'a> {
    cursor: SourceCursor<'a>,
}

/// Represents an identity, could be a keyword or variable name
#[derive(Debug, Clone)]
pub struct Ident<'a> {
    span: SourceSpan<'a>,
}

/// Represents an unparsed
#[derive(Debug, Clone)]
pub struct NumericLit<'a> {
    span: SourceSpan<'a>,
}

/// Represents an unparsed string literal, only escaped delimeters are checked
#[derive(Debug, Clone)]
pub struct StringLit<'a> {
    span: SourceSpan<'a>
}

/// Represents an unparsed character literal, only escaped single quote is checked
#[derive(Debug, Clone)]
pub struct CharLit<'a> {
    span: SourceSpan<'a>
}

/// Represents any literal type
#[derive(Debug, Clone)]
pub enum Literal<'a> {
    Numeric(NumericLit<'a>),
    String(StringLit<'a>),
    Char(CharLit<'a>),
}

impl<'a> Literal<'a> {
    fn span(&self) -> SourceSpan<'a> {
        match self {
            Literal::Numeric(numeric_lit) => numeric_lit.span.clone(),
            Literal::String(string_lit) => string_lit.span.clone(),
            Literal::Char(char_lit) => char_lit.span.clone(),
        }
    }
}

/// Represents any continguous span of whitespace
#[derive(Debug, Clone)]
pub struct Whitespace<'a> {
    span: SourceSpan<'a>
}

/// Represents any contiguous comment, line comments "//" spanning multiple lines are merged into a
/// single comment
#[derive(Debug, Clone)]
pub struct Comment<'a> {
    span: SourceSpan<'a>
}

/// Represents any element which serves as a break between other elements
#[derive(Debug, Clone)]
pub enum Break<'a> {
    Whitespace(Whitespace<'a>),
    Comment(Comment<'a>),
}

impl<'a> Break<'a> {
    fn span(&self) -> SourceSpan<'a> {
        match self {
            Break::Whitespace(whitespace) => whitespace.span.clone(),
            Break::Comment(comment) => comment.span.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Atom<'a> {
    Break(Break<'a>),
    Literal(Literal<'a>),
    Punct(Punct<'a>),
    Ident(Ident<'a>),
}

fn parse_comment<'a>(cursor: &SourceCursor<'a>) 
    -> Result<Option<Atom<'a>>, AlliumError> {
    if cursor.to_char() != '/' { return Ok(None); }
    let next = cursor.next()?;
    match next.to_char() {
        '/' => {
            let span = cursor.as_span().grow_until("\n", false, true)?;
            Ok(Some(Atom::Break(Break::Comment(Comment { span }))))
        },
        '*' => {
            let span = cursor.as_span().grow_right(1)?.grow_until_block_end("/*", "*/", true)?;
            Ok(Some(Atom::Break(Break::Comment(Comment { span }))))
        },
        _ => Ok(None),
    }
}

fn parse_whitespace<'a>(cursor: &SourceCursor<'a>)
    -> Result<Option<Atom<'a>>, AlliumError> {
    if !cursor.to_char().is_whitespace() {
        return Ok(None);
    }
    
    let mut head = cursor.clone();

    loop {
        head = match head.next() {
            Ok(c) 
                if c.to_char().is_whitespace() => c,
            Ok(_) | Err(AlliumError::Eof)=> { 
                let span = cursor.span_to(&head)?;
                return Ok(Some(Atom::Break(Break::Whitespace(Whitespace { span }))));
            },
            Err(e) => return Err(e),
        };
    }
}

fn parse_punct<'a>(cursor: &SourceCursor<'a>) -> Result<Option<Atom<'a>>, AlliumError> {
    match cursor.to_char() {
        // block delimeters
        '{' | '}' | 
        '[' | ']' | 
        '(' | ')' |
        // math operators
        '+' | '-' | '*' | '/' | '%' | '=' |
        // comparison operators
        '<' | '>' |
        // logic operators
        '|' | '&' | '^' | '~' | '!' |
        // discard operator
        '_' |
        // other
        '.'  | '@' | '$' => Ok(Some(Atom::Punct(Punct { cursor: cursor.clone() }))), 
        _ => Ok(None),
    }
}


type Parser = for<'a> fn(&SourceCursor<'a>) -> Result<Option<Atom<'a>>, AlliumError>;

/// order determins parsing priority
const PARSERS : [Parser; 3] = [
    parse_whitespace, 
    parse_comment,
    parse_punct,
];



impl<'a> Atom<'a> {
    fn next(&self) -> Result<Option<Atom<'a>>, AlliumError> {
        let next = match self {
            Atom::Punct(punct) => punct.cursor.next()?,
            Atom::Ident(ident) => ident.span.next()?,
            Atom::Literal(literal) => literal.span().next()?,
            Atom::Break(b) => b.span().next()?,
        };

 
        todo!();
    }

    fn parse(cursor: &SourceCursor<'a>) -> Result<Atom<'a>, AlliumError> {
        for parser in PARSERS {
            if let Some(a) = parser(cursor)? {
                return Ok(a);
            }
        }


        todo!();
    }
}

