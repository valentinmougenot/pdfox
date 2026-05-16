use pdfox_core::PdfObject;
use pdfox_parser::{PdfParserError, Token};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum PdfDocumentError {
    #[error("Unexpected EOF")]
    Eof,
    #[error("Invalid header")]
    InvalidHeader,
    #[error("Invalid XRef")]
    InvalidXRef,
    #[error("Unexpected object {0:?}")]
    UnexpectedObject(PdfObject),
    #[error("Unexpected token {0:?}")]
    UnexpectedToken(Token),
    #[error("{0}")]
    PdfParserError(#[from] PdfParserError),
}

pub type Result<T> = std::result::Result<T, PdfDocumentError>;
