use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum PdfLexerError {
    #[error("Unexpected EOF")]
    Eof,
    #[error("Invalid byte '{}' at pos {}", *.0 as char, .1)]
    InvalidByte(u8, usize),
    #[error("Unknown keyword '{0}' at pos {1}")]
    UnknownKeyword(String, usize),
}

pub type Result<T> = std::result::Result<T, PdfLexerError>;
