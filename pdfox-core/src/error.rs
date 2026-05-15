use thiserror::Error;

use crate::PdfName;

#[derive(Error, Debug, PartialEq, Clone)]
pub enum PdfError {
    #[error("Key '{0}' not found")]
    KeyNotFound(PdfName),
}

pub type Result<T> = std::result::Result<T, PdfError>;
