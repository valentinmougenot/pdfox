use std::collections::HashMap;

use pdfox_core::{PdfDict, PdfObject};
use pdfox_parser::{Keyword, Lexer, Parser, Token};

use crate::{PdfDocumentError, Result};

pub struct XRefTable {
    entries: HashMap<u32, XRefEntry>,
}

impl XRefTable {
    pub fn parse(data: &[u8], offset: usize) -> Result<(Self, PdfDict)> {
        let data = &data[offset..];
        let mut parser = Parser::new(Lexer::new(data.into()));
        parser.expect_token(Token::Keyword(Keyword::XRef))?;

        let mut entries = HashMap::new();

        loop {
            match parser.next_token().ok_or(PdfDocumentError::Eof)?? {
                Token::Keyword(Keyword::Trailer) => match parser.parse_object()? {
                    PdfObject::Dictionary(dict) => return Ok((Self { entries }, dict)),
                    other => return Err(PdfDocumentError::UnexpectedObject(other)),
                },
                Token::Integer(id) => {
                    let count = match parser.next_token().ok_or(PdfDocumentError::Eof)?? {
                        Token::Integer(n) => n,
                        other => return Err(PdfDocumentError::UnexpectedToken(other)),
                    };

                    parser.skip_whitespace();
                    for i in 0..count {
                        let data = parser.read_bytes(20)?;

                        let offset = std::str::from_utf8(&data[..10])
                            .map_err(|_| PdfDocumentError::InvalidXRef)?
                            .trim()
                            .parse()
                            .map_err(|_| PdfDocumentError::InvalidXRef)?;

                        let r#gen = std::str::from_utf8(&data[11..16])
                            .map_err(|_| PdfDocumentError::InvalidXRef)?
                            .trim()
                            .parse()
                            .map_err(|_| PdfDocumentError::InvalidXRef)?;

                        let status = data.iter().find(|&&b| b == b'n' || b == b'f');

                        let in_use = match status {
                            Some(b'n') => true,
                            Some(b'f') => false,
                            _ => return Err(PdfDocumentError::InvalidXRef),
                        };

                        entries.insert(
                            (id + i) as u32,
                            XRefEntry {
                                offset,
                                r#gen,
                                in_use,
                            },
                        );
                    }
                }
                other => return Err(PdfDocumentError::UnexpectedToken(other)),
            }
        }
    }
}

pub struct XRefEntry {
    offset: usize,
    r#gen: u16,
    in_use: bool,
}

#[cfg(test)]
mod tests {
    use pdfox_core::PdfObject;

    use super::XRefTable;
    use crate::PdfDocumentError;

    // --- XRefTable::parse ---

    #[test]
    fn test_parse_simple() {
        let data = b"xref\n0 2\n0000000000 65535 f\r\n0000000009 00000 n\r\ntrailer\n<</Size 2>>";
        let (_, trailer) = XRefTable::parse(data, 0).unwrap();
        assert_eq!(trailer.get(&b"Size".into()), Some(&PdfObject::Integer(2)));
    }

    #[test]
    fn test_parse_with_root() {
        let data =
            b"xref\n0 2\n0000000000 65535 f\r\n0000000009 00000 n\r\ntrailer\n<</Size 2 /Root 1 0 R>>";
        let (_, trailer) = XRefTable::parse(data, 0).unwrap();
        assert_eq!(
            trailer.get(&b"Root".into()),
            Some(&PdfObject::IndirectRef(1, 0))
        );
    }

    #[test]
    fn test_parse_multiple_subsections() {
        let data =
            b"xref\n0 1\n0000000000 65535 f\r\n5 1\n0000000100 00000 n\r\ntrailer\n<</Size 6>>";
        let (_, trailer) = XRefTable::parse(data, 0).unwrap();
        assert_eq!(trailer.get(&b"Size".into()), Some(&PdfObject::Integer(6)));
    }

    #[test]
    fn test_parse_at_offset() {
        // xref table starts after some body content
        let body = b"1 0 obj\ntrue\nendobj\n"; // 20 bytes
        let xref = b"xref\n0 1\n0000000000 65535 f\r\ntrailer\n<</Size 1>>";
        let mut data = body.to_vec();
        data.extend_from_slice(xref);
        let (_, trailer) = XRefTable::parse(&data, body.len()).unwrap();
        assert_eq!(trailer.get(&b"Size".into()), Some(&PdfObject::Integer(1)));
    }

    #[test]
    fn test_parse_missing_xref_keyword() {
        let data = b"0 1\n0000000000 65535 f\r\ntrailer\n<<>>";
        assert!(XRefTable::parse(data, 0).is_err());
    }

    #[test]
    fn test_parse_invalid_gen() {
        let data = b"xref\n0 1\n0000000000 XXXXX n\r\ntrailer\n<<>>";
        assert!(matches!(
            XRefTable::parse(data, 0),
            Err(PdfDocumentError::InvalidXRef)
        ));
    }

    #[test]
    fn test_parse_invalid_status() {
        // status is neither 'n' nor 'f'
        let data = b"xref\n0 1\n0000000000 00000 x\r\ntrailer\n<<>>";
        assert!(matches!(
            XRefTable::parse(data, 0),
            Err(PdfDocumentError::InvalidXRef)
        ));
    }

    #[test]
    fn test_parse_truncated_entry() {
        // entry is only 10 bytes instead of 20
        let data = b"xref\n0 1\n0000000000";
        assert!(XRefTable::parse(data, 0).is_err());
    }

    #[test]
    fn test_parse_missing_trailer() {
        let data = b"xref\n0 1\n0000000000 65535 f\r\n";
        assert!(matches!(
            XRefTable::parse(data, 0),
            Err(PdfDocumentError::Eof)
        ));
    }

    #[test]
    fn test_parse_trailer_not_dict() {
        let data = b"xref\n0 1\n0000000000 65535 f\r\ntrailer\n42";
        assert!(matches!(
            XRefTable::parse(data, 0),
            Err(PdfDocumentError::UnexpectedObject(_))
        ));
    }
}
