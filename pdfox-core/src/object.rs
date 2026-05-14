#[derive(Debug, PartialEq)]
pub enum PdfObject {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    String(PdfString),
    Name(Box<[u8]>),
    Null,
    Array(Vec<PdfObject>),
    Dictionary(Vec<(Box<[u8]>, PdfObject)>),
    IndirectRef(i64, i64),
    IndirectObject {
        num: i64,
        r#gen: i64,
        value: Box<PdfObject>,
    },
    Stream {
        dict: Vec<(Box<[u8]>, PdfObject)>,
        data: Box<[u8]>,
    },
}

#[derive(Debug, PartialEq)]
pub enum PdfString {
    Literal(Box<[u8]>),
    Hex(Box<[u8]>),
}
