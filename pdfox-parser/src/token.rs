#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Boolean(bool),
    Integer(i64),
    Real(f64),
    Name(Box<[u8]>),
    LiteralString(Box<[u8]>),
    HexString(Box<[u8]>),
    Null,
    ArrayBegin,
    ArrayEnd,
    DictBegin,
    DictEnd,
    Keyword(Keyword),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Keyword {
    Obj,
    EndObj,
    Stream,
    EndStream,
    R,
    XRef,
    Trailer,
    StartXRef,
}
