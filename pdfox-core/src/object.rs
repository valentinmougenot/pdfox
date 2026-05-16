use crate::{PdfError, Result};

#[derive(Debug, PartialEq, Clone)]
pub enum PdfObject {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(PdfString),
    Name(PdfName),
    Null,
    Array(Vec<PdfObject>),
    Dictionary(PdfDict),
    IndirectRef(i64, i64),
    IndirectObject {
        num: i64,
        r#gen: i64,
        value: Box<PdfObject>,
    },
    Stream {
        dict: PdfDict,
        data: Box<[u8]>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum PdfString {
    Literal(Box<[u8]>),
    Hex(Box<[u8]>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct PdfName(Box<[u8]>);

impl PdfName {
    pub fn new(value: Box<[u8]>) -> Self {
        Self(value)
    }
}

impl From<Box<[u8]>> for PdfName {
    fn from(value: Box<[u8]>) -> Self {
        Self(value)
    }
}

impl From<&[u8]> for PdfName {
    fn from(value: &[u8]) -> Self {
        Self(value.into())
    }
}

impl<const N: usize> From<&[u8; N]> for PdfName {
    fn from(value: &[u8; N]) -> Self {
        Self(value.as_ref().into())
    }
}

impl std::fmt::Display for PdfName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(&self.0))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PdfDict(Vec<(PdfName, PdfObject)>);

impl PdfDict {
    pub fn new(entries: Vec<(PdfName, PdfObject)>) -> Self {
        Self(entries)
    }

    pub fn get(&self, key: &PdfName) -> Option<&PdfObject> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn get_required(&self, key: &PdfName) -> Result<&PdfObject> {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
            .ok_or(PdfError::KeyNotFound(key.clone()))
    }
}

impl From<Vec<(PdfName, PdfObject)>> for PdfDict {
    fn from(value: Vec<(PdfName, PdfObject)>) -> Self {
        Self(value)
    }
}
