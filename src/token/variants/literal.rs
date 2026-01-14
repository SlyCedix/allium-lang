use crate::token::Tok;

/// Representation of a literal of a given type
///
/// TODO: refactored into multiple files
#[derive(Debug, Clone)]
pub enum Literal {
    // a character identifier begins with single quote(`'`)
    Char(u32, String),
    RawChar(u32, String),
    String(String, String),
    RawString(String, String),
    ByteString(String, String),
    CString(Vec<u8>, String),
    Integer(u128, String),
    Decimal(String, String),
}
