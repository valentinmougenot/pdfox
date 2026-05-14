use std::{collections::VecDeque, iter::Peekable};

use pdfox_core::{PdfObject, PdfString};

use crate::{
    Lexer, PdfParserError,
    error::Result,
    token::{Keyword, Token},
};

pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
    tokens_queue: VecDeque<Result<Token>>,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Self {
        Self {
            lexer: lexer.peekable(),
            tokens_queue: VecDeque::with_capacity(4),
        }
    }

    pub fn parse_object(&mut self) -> Result<PdfObject> {
        match self.next_token() {
            Some(Ok(Token::Boolean(b))) => Ok(PdfObject::Boolean(b)),
            Some(Ok(Token::Integer(n))) => {
                if let Some(Ok(Token::Integer(g))) = self.peek_token() {
                    let g = *g;
                    if self.peek_snd_token() == Some(&Ok(Token::Keyword(Keyword::R))) {
                        self.next_token();
                        self.next_token();
                        return Ok(PdfObject::IndirectRef(n, g));
                    } else if self.peek_snd_token() == Some(&Ok(Token::Keyword(Keyword::Obj))) {
                        self.next_token();
                        self.next_token();

                        let value = self.parse_object()?;

                        return match self.next_token() {
                            Some(Ok(Token::Keyword(Keyword::EndObj))) => {
                                Ok(PdfObject::IndirectObject {
                                    num: n,
                                    r#gen: g,
                                    value: Box::new(value),
                                })
                            }
                            Some(Ok(Token::Keyword(Keyword::Stream))) => {
                                todo!("Handle stream object")
                            }
                            Some(Ok(other)) => Err(PdfParserError::UnexpectedToken(other)),
                            Some(Err(e)) => Err(e),
                            None => Err(PdfParserError::Eof),
                        };
                    }
                }

                Ok(PdfObject::Integer(n))
            }
            Some(Ok(Token::Real(x))) => Ok(PdfObject::Real(x)),
            Some(Ok(Token::Null)) => Ok(PdfObject::Null),
            Some(Ok(Token::Name(b))) => Ok(PdfObject::Name(b)),
            Some(Ok(Token::LiteralString(b))) => Ok(PdfObject::String(PdfString::Literal(b))),
            Some(Ok(Token::HexString(b))) => Ok(PdfObject::String(PdfString::Hex(b))),
            Some(Ok(Token::ArrayBegin)) => {
                let mut array = Vec::new();
                loop {
                    match self.peek_token() {
                        Some(Ok(Token::ArrayEnd)) => {
                            self.next_token();
                            return Ok(PdfObject::Array(array));
                        }
                        Some(Ok(_)) => {
                            array.push(self.parse_object()?);
                        }
                        Some(Err(e)) => return Err(e.clone()),
                        None => return Err(PdfParserError::Eof),
                    }
                }
            }
            Some(Ok(Token::DictBegin)) => {
                let mut dict = Vec::new();
                loop {
                    match self.next_token() {
                        Some(Ok(Token::DictEnd)) => {
                            return Ok(PdfObject::Dictionary(dict));
                        }
                        Some(Ok(Token::Name(key))) => {
                            let value = self.parse_object()?;
                            dict.push((key, value));
                        }
                        Some(Ok(other)) => {
                            return Err(PdfParserError::UnexpectedToken(other));
                        }
                        Some(Err(e)) => return Err(e.clone()),
                        None => return Err(PdfParserError::Eof),
                    }
                }
            }
            None => Err(PdfParserError::Eof),
            _ => todo!(),
        }
    }

    fn next_token(&mut self) -> Option<Result<Token>> {
        if let Some(token) = self.tokens_queue.pop_front() {
            Some(token)
        } else {
            self.lexer.next()
        }
    }

    fn peek_token(&mut self) -> Option<&Result<Token>> {
        self.peek_n_token(0)
    }

    fn peek_snd_token(&mut self) -> Option<&Result<Token>> {
        self.peek_n_token(1)
    }

    fn peek_n_token(&mut self, n: usize) -> Option<&Result<Token>> {
        while self.tokens_queue.len() < n + 1 {
            self.tokens_queue.push_back(self.lexer.next()?);
        }
        self.tokens_queue.get(n)
    }
}

#[cfg(test)]
mod tests {
    use pdfox_core::{PdfObject, PdfString};

    use crate::{Lexer, PdfParserError, token::{Keyword, Token}};

    use super::Parser;

    fn parser(input: &[u8]) -> Parser<'_> {
        Parser::new(Lexer::new(input))
    }

    // --- scalaires ---

    #[test]
    fn test_boolean_true() {
        assert_eq!(parser(b"true").parse_object(), Ok(PdfObject::Boolean(true)));
    }

    #[test]
    fn test_boolean_false() {
        assert_eq!(
            parser(b"false").parse_object(),
            Ok(PdfObject::Boolean(false))
        );
    }

    #[test]
    fn test_integer() {
        assert_eq!(parser(b"42").parse_object(), Ok(PdfObject::Integer(42)));
    }

    #[test]
    fn test_real() {
        assert_eq!(parser(b"3.14").parse_object(), Ok(PdfObject::Real(3.14)));
    }

    #[test]
    fn test_null() {
        assert_eq!(parser(b"null").parse_object(), Ok(PdfObject::Null));
    }

    #[test]
    fn test_name() {
        assert_eq!(
            parser(b"/Type").parse_object(),
            Ok(PdfObject::Name((*b"Type").into()))
        );
    }

    #[test]
    fn test_literal_string() {
        assert_eq!(
            parser(b"(Hello)").parse_object(),
            Ok(PdfObject::String(PdfString::Literal((*b"Hello").into())))
        );
    }

    #[test]
    fn test_hex_string() {
        assert_eq!(
            parser(b"<48656C6C6F>").parse_object(),
            Ok(PdfObject::String(PdfString::Hex((*b"Hello").into())))
        );
    }

    // --- array ---

    #[test]
    fn test_array_empty() {
        assert_eq!(parser(b"[]").parse_object(), Ok(PdfObject::Array(vec![])));
    }

    #[test]
    fn test_array_integers() {
        assert_eq!(
            parser(b"[1 2 3]").parse_object(),
            Ok(PdfObject::Array(vec![
                PdfObject::Integer(1),
                PdfObject::Integer(2),
                PdfObject::Integer(3),
            ]))
        );
    }

    #[test]
    fn test_array_mixed() {
        assert_eq!(
            parser(b"[true 1 /Name]").parse_object(),
            Ok(PdfObject::Array(vec![
                PdfObject::Boolean(true),
                PdfObject::Integer(1),
                PdfObject::Name((*b"Name").into()),
            ]))
        );
    }

    #[test]
    fn test_array_nested() {
        assert_eq!(
            parser(b"[[1 2] [3]]").parse_object(),
            Ok(PdfObject::Array(vec![
                PdfObject::Array(vec![PdfObject::Integer(1), PdfObject::Integer(2)]),
                PdfObject::Array(vec![PdfObject::Integer(3)]),
            ]))
        );
    }

    #[test]
    fn test_array_eof() {
        assert_eq!(parser(b"[1 2").parse_object(), Err(PdfParserError::Eof));
    }

    // --- dictionary ---

    #[test]
    fn test_dict_empty() {
        assert_eq!(
            parser(b"<<>>").parse_object(),
            Ok(PdfObject::Dictionary(vec![]))
        );
    }

    #[test]
    fn test_dict_single_entry() {
        assert_eq!(
            parser(b"<</Type /Page>>").parse_object(),
            Ok(PdfObject::Dictionary(vec![(
                (*b"Type").into(),
                PdfObject::Name((*b"Page").into()),
            )]))
        );
    }

    #[test]
    fn test_dict_multiple_entries() {
        assert_eq!(
            parser(b"<</Width 100 /Height 200>>").parse_object(),
            Ok(PdfObject::Dictionary(vec![
                ((*b"Width").into(), PdfObject::Integer(100)),
                ((*b"Height").into(), PdfObject::Integer(200)),
            ]))
        );
    }

    #[test]
    fn test_dict_nested() {
        assert_eq!(
            parser(b"<</Inner <</Key 1>>>>").parse_object(),
            Ok(PdfObject::Dictionary(vec![(
                (*b"Inner").into(),
                PdfObject::Dictionary(vec![((*b"Key").into(), PdfObject::Integer(1),)]),
            )]))
        );
    }

    #[test]
    fn test_dict_with_array_value() {
        assert_eq!(
            parser(b"<</Kids [1 2]>>").parse_object(),
            Ok(PdfObject::Dictionary(vec![(
                (*b"Kids").into(),
                PdfObject::Array(vec![PdfObject::Integer(1), PdfObject::Integer(2)]),
            )]))
        );
    }

    #[test]
    fn test_dict_invalid_key() {
        assert_eq!(
            parser(b"<<true /Val>>").parse_object(),
            Err(PdfParserError::UnexpectedToken(Token::Boolean(true)))
        );
    }

    #[test]
    fn test_dict_eof() {
        assert_eq!(parser(b"<</Key").parse_object(), Err(PdfParserError::Eof));
    }

    // --- références indirectes ---

    #[test]
    fn test_indirect_ref_simple() {
        assert_eq!(
            parser(b"1 0 R").parse_object(),
            Ok(PdfObject::IndirectRef(1, 0))
        );
    }

    #[test]
    fn test_indirect_ref_nonzero_generation() {
        assert_eq!(
            parser(b"5 3 R").parse_object(),
            Ok(PdfObject::IndirectRef(5, 3))
        );
    }

    #[test]
    fn test_integer_not_indirect_ref_no_second_int() {
        assert_eq!(parser(b"42").parse_object(), Ok(PdfObject::Integer(42)));
    }

    #[test]
    fn test_integer_not_indirect_ref_no_r() {
        let mut p = parser(b"1 2");
        assert_eq!(p.parse_object(), Ok(PdfObject::Integer(1)));
        assert_eq!(p.parse_object(), Ok(PdfObject::Integer(2)));
    }

    #[test]
    fn test_indirect_object_eof_after_obj() {
        assert_eq!(
            parser(b"1 2 obj").parse_object(),
            Err(PdfParserError::Eof)
        );
    }

    #[test]
    fn test_indirect_ref_in_array() {
        assert_eq!(
            parser(b"[1 0 R 2 0 R]").parse_object(),
            Ok(PdfObject::Array(vec![
                PdfObject::IndirectRef(1, 0),
                PdfObject::IndirectRef(2, 0),
            ]))
        );
    }

    #[test]
    fn test_indirect_ref_as_dict_value() {
        assert_eq!(
            parser(b"<</Parent 3 0 R>>").parse_object(),
            Ok(PdfObject::Dictionary(vec![(
                (*b"Parent").into(),
                PdfObject::IndirectRef(3, 0),
            )]))
        );
    }

    #[test]
    fn test_indirect_ref_followed_by_other_token() {
        let mut p = parser(b"1 0 R /Next");
        assert_eq!(p.parse_object(), Ok(PdfObject::IndirectRef(1, 0)));
        assert_eq!(p.parse_object(), Ok(PdfObject::Name((*b"Next").into())));
    }

    // --- objets indirects ---

    #[test]
    fn test_indirect_object_integer() {
        assert_eq!(
            parser(b"1 0 obj 42 endobj").parse_object(),
            Ok(PdfObject::IndirectObject {
                num: 1,
                r#gen: 0,
                value: Box::new(PdfObject::Integer(42)),
            })
        );
    }

    #[test]
    fn test_indirect_object_nonzero_generation() {
        assert_eq!(
            parser(b"3 2 obj true endobj").parse_object(),
            Ok(PdfObject::IndirectObject {
                num: 3,
                r#gen: 2,
                value: Box::new(PdfObject::Boolean(true)),
            })
        );
    }

    #[test]
    fn test_indirect_object_string() {
        assert_eq!(
            parser(b"2 0 obj (Hello) endobj").parse_object(),
            Ok(PdfObject::IndirectObject {
                num: 2,
                r#gen: 0,
                value: Box::new(PdfObject::String(PdfString::Literal((*b"Hello").into()))),
            })
        );
    }

    #[test]
    fn test_indirect_object_dictionary() {
        assert_eq!(
            parser(b"4 0 obj <</Type /Page>> endobj").parse_object(),
            Ok(PdfObject::IndirectObject {
                num: 4,
                r#gen: 0,
                value: Box::new(PdfObject::Dictionary(vec![(
                    (*b"Type").into(),
                    PdfObject::Name((*b"Page").into()),
                )])),
            })
        );
    }

    #[test]
    fn test_indirect_object_with_indirect_ref_value() {
        assert_eq!(
            parser(b"5 0 obj 3 0 R endobj").parse_object(),
            Ok(PdfObject::IndirectObject {
                num: 5,
                r#gen: 0,
                value: Box::new(PdfObject::IndirectRef(3, 0)),
            })
        );
    }

    #[test]
    fn test_indirect_object_missing_endobj() {
        assert_eq!(
            parser(b"1 0 obj 42").parse_object(),
            Err(PdfParserError::Eof)
        );
    }

    #[test]
    fn test_indirect_object_unexpected_token_instead_of_endobj() {
        assert_eq!(
            parser(b"1 0 obj 42 obj").parse_object(),
            Err(PdfParserError::UnexpectedToken(Token::Keyword(Keyword::Obj)))
        );
    }

    #[test]
    fn test_indirect_object_followed_by_another() {
        let mut p = parser(b"1 0 obj 42 endobj 2 0 obj true endobj");
        assert_eq!(
            p.parse_object(),
            Ok(PdfObject::IndirectObject {
                num: 1,
                r#gen: 0,
                value: Box::new(PdfObject::Integer(42)),
            })
        );
        assert_eq!(
            p.parse_object(),
            Ok(PdfObject::IndirectObject {
                num: 2,
                r#gen: 0,
                value: Box::new(PdfObject::Boolean(true)),
            })
        );
    }
}
