use thiserror::Error;

use crate::token::Token;

#[derive(Error, Debug, PartialEq, Clone)]
pub enum PdfParserError {
    #[error("Unexpected EOF")]
    Eof,
    #[error("Invalid byte '{}' at pos {}", *.0 as char, .1)]
    InvalidByte(u8, usize),
    #[error("Unknown keyword '{0}' at pos {1}")]
    UnknownKeyword(String, usize),
    #[error("Unexpected token '{0:?}'")]
    UnexpectedToken(Token),
}

pub type Result<T> = std::result::Result<T, PdfParserError>;
