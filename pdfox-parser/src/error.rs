use pdfox_core::{PdfError, PdfName};
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
    #[error("Invalid value for key '{0}'")]
    InvalidDictValue(PdfName),
    #[error("Stream keyword must follow a dictionary")]
    StreamWithoutDict,
    #[error("{0}")]
    PdfError(#[from] PdfError),
}

pub type Result<T> = std::result::Result<T, PdfParserError>;
