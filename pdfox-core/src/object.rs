#[derive(Debug, PartialEq)]
pub enum PdfObject {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(PdfString),
    Name(Box<[u8]>),
    Null,
}

#[derive(Debug, PartialEq)]
pub enum PdfString {
    Literal(Box<[u8]>),
    Hex(Box<[u8]>),
}
