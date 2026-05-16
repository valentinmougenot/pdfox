use pdfox_core::PdfDict;

use crate::xref::XRefTable;
use crate::{PdfDocumentError, Result};

pub struct PdfDocument {
    data: Box<[u8]>,
    xref: XRefTable,
    trailer: PdfDict,
}

impl PdfDocument {
    pub fn from_bytes(data: impl Into<Box<[u8]>>) -> Result<Self> {
        let data = data.into();

        let xref_offset = find_startxref(&data)?;
        let (xref, trailer) = XRefTable::parse(&data, xref_offset)?;

        Ok(Self {
            data,
            xref,
            trailer,
        })
    }
}

fn find_startxref(data: &[u8]) -> Result<usize> {
    let data = if data.len() > 100 {
        &data[data.len() - 100..]
    } else {
        data
    };

    let label = b"startxref";
    let startxref_pos = data
        .windows(label.len())
        .position(|w| w == label)
        .ok_or(PdfDocumentError::InvalidHeader)?;

    let num_pos = match data.get(startxref_pos + label.len()) {
        Some(b'\r') => match data.get(startxref_pos + label.len() + 1) {
            Some(b'\n') => startxref_pos + label.len() + 2,
            Some(_) => return Err(PdfDocumentError::InvalidHeader),
            None => return Err(PdfDocumentError::Eof),
        },
        Some(b'\n') => startxref_pos + label.len() + 1,
        Some(_) => return Err(PdfDocumentError::InvalidHeader),
        None => return Err(PdfDocumentError::Eof),
    };

    let num_end = data[num_pos..]
        .iter()
        .position(|c| !c.is_ascii_digit())
        .map(|p| p + num_pos)
        .unwrap_or(data.len());

    String::from_utf8_lossy(&data[num_pos..num_end])
        .parse()
        .map_err(|_| PdfDocumentError::InvalidHeader)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- find_startxref ---

    #[test]
    fn test_find_startxref_lf() {
        assert_eq!(find_startxref(b"startxref\n42\n%%EOF").unwrap(), 42);
    }

    #[test]
    fn test_find_startxref_crlf() {
        assert_eq!(
            find_startxref(b"startxref\r\n12345\r\n%%EOF").unwrap(),
            12345
        );
    }

    #[test]
    fn test_find_startxref_large_file() {
        let mut data = vec![b'X'; 200];
        data.extend_from_slice(b"startxref\n99\n%%EOF");
        assert_eq!(find_startxref(&data).unwrap(), 99);
    }

    #[test]
    fn test_find_startxref_missing() {
        assert!(matches!(
            find_startxref(b"%%EOF"),
            Err(PdfDocumentError::InvalidHeader)
        ));
    }

    #[test]
    fn test_find_startxref_invalid_eol() {
        assert!(matches!(
            find_startxref(b"startxref X42\n%%EOF"),
            Err(PdfDocumentError::InvalidHeader)
        ));
    }

    // --- from_bytes ---

    #[test]
    fn test_from_bytes_lf() {
        let data = b"xref\n0 1\n0000000000 65535 f\r\ntrailer\n<</Size 1>>\nstartxref\n0\n%%EOF";
        assert!(PdfDocument::from_bytes(data.to_vec()).is_ok());
    }

    #[test]
    fn test_from_bytes_crlf() {
        let data =
            b"xref\n0 1\n0000000000 65535 f\r\ntrailer\n<</Size 1>>\r\nstartxref\r\n0\r\n%%EOF";
        assert!(PdfDocument::from_bytes(data.to_vec()).is_ok());
    }

    #[test]
    fn test_from_bytes_missing_startxref() {
        assert!(matches!(
            PdfDocument::from_bytes(b"garbage data here".to_vec()),
            Err(PdfDocumentError::InvalidHeader)
        ));
    }
}
