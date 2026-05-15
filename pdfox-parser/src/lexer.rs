use crate::{
    error::{PdfParserError, Result},
    token::{Keyword, Token},
};

pub struct Lexer<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn parse_literal_str(&mut self) -> Result<Token> {
        debug_assert_eq!(self.peek(), b'(');
        self.bump();

        let mut result = Vec::new();
        let mut depth = 0;
        loop {
            if self.pos >= self.buf.len() {
                return Err(PdfParserError::Eof);
            }

            match self.peek() {
                b')' if depth == 0 => {
                    self.bump();
                    break;
                }
                b'(' => {
                    depth += 1;
                    result.push(b'(');
                    self.bump();
                }
                b')' => {
                    depth -= 1;
                    result.push(b')');
                    self.bump();
                }
                b'\\' if self.pos + 1 < self.buf.len() => match self.buf[self.pos + 1] {
                    b'n' => {
                        result.push(b'\n');
                        self.bump();
                        self.bump();
                    }
                    b'r' => {
                        result.push(b'\r');
                        self.bump();
                        self.bump();
                    }
                    b't' => {
                        result.push(b'\t');
                        self.bump();
                        self.bump();
                    }
                    b'b' => {
                        result.push(b'\x08');
                        self.bump();
                        self.bump();
                    }
                    b'f' => {
                        result.push(b'\x0c');
                        self.bump();
                        self.bump();
                    }
                    b'(' => {
                        result.push(b'(');
                        self.bump();
                        self.bump();
                    }
                    b')' => {
                        result.push(b')');
                        self.bump();
                        self.bump();
                    }
                    b'\\' => {
                        result.push(b'\\');
                        self.bump();
                        self.bump();
                    }
                    c => {
                        return Err(PdfParserError::InvalidByte(c, self.pos + 1));
                    }
                },
                c => {
                    result.push(c);
                    self.bump();
                }
            }
        }

        Ok(Token::LiteralString(result.into_boxed_slice()))
    }

    fn parse_hex_str(&mut self) -> Result<Token> {
        debug_assert_eq!(self.peek(), b'<');
        self.bump();

        let mut result = Vec::new();
        loop {
            self.bump_while(is_whitespace);
            if self.is_eof() {
                return Err(PdfParserError::Eof);
            }
            if self.peek() == b'>' {
                self.bump();
                break;
            }
            let a = (self.peek() as char)
                .to_digit(16)
                .ok_or(PdfParserError::InvalidByte(self.peek(), self.pos))?;
            self.bump();
            self.bump_while(is_whitespace);
            let b = if !self.is_eof() && self.peek() != b'>' {
                let digit = (self.peek() as char)
                    .to_digit(16)
                    .ok_or(PdfParserError::InvalidByte(self.peek(), self.pos))?;
                self.bump();
                digit
            } else {
                0
            };
            result.push(((a << 4) | b) as u8);
        }

        Ok(Token::HexString(result.into_boxed_slice()))
    }

    fn parse_name(&mut self) -> Result<Token> {
        debug_assert_eq!(self.peek(), b'/');
        self.bump();

        let mut result = Vec::new();
        while self.pos < self.buf.len() {
            match self.peek() {
                b'#' if self.pos + 2 < self.buf.len() => {
                    self.bump();
                    let a = (self.peek() as char)
                        .to_digit(16)
                        .ok_or(PdfParserError::InvalidByte(self.peek(), self.pos))?;
                    self.bump();
                    let b = (self.peek() as char)
                        .to_digit(16)
                        .ok_or(PdfParserError::InvalidByte(self.peek(), self.pos))?;
                    result.push(((a << 4) | b) as u8);
                }
                c if is_whitespace(c) || is_delimiter(c) => break,
                c => result.push(c),
            }

            self.bump();
        }

        Ok(Token::Name(result.into_boxed_slice()))
    }

    fn parse_keyword(&mut self) -> Result<Token> {
        let start = self.pos;
        self.bump_while(|c| c.is_ascii_alphabetic());

        match &self.buf[start..self.pos] {
            b"true" => Ok(Token::Boolean(true)),
            b"false" => Ok(Token::Boolean(false)),
            b"null" => Ok(Token::Null),
            b"obj" => Ok(Token::Keyword(Keyword::Obj)),
            b"endobj" => Ok(Token::Keyword(Keyword::EndObj)),
            b"stream" => Ok(Token::Keyword(Keyword::Stream)),
            b"endstream" => Ok(Token::Keyword(Keyword::EndStream)),
            b"R" => Ok(Token::Keyword(Keyword::R)),
            b"xref" => Ok(Token::Keyword(Keyword::XRef)),
            b"trailer" => Ok(Token::Keyword(Keyword::Trailer)),
            b"startxref" => Ok(Token::Keyword(Keyword::StartXRef)),
            other => Err(PdfParserError::UnknownKeyword(
                String::from_utf8_lossy(other).to_string(),
                start,
            )),
        }
    }

    fn parse_number(&mut self) -> Result<Token> {
        debug_assert!(matches!(self.peek(), b'+' | b'-' | b'.') || self.peek().is_ascii_digit());

        let sign: i64 = match self.peek() {
            b'-' => {
                self.bump();
                -1
            }
            b'+' => {
                self.bump();
                1
            }
            _ => 1,
        };

        let mut int_part: i64 = 0;
        while !self.is_eof() && self.peek().is_ascii_digit() {
            int_part = int_part * 10 + (self.peek() - b'0') as i64;
            self.bump();
        }

        if !self.is_eof() && self.peek() == b'.' {
            self.bump();
            let mut frac: f64 = 0.0;
            let mut divisor = 10.0f64;
            while !self.is_eof() && self.peek().is_ascii_digit() {
                frac += (self.peek() - b'0') as f64 / divisor;
                divisor *= 10.0;
                self.bump();
            }
            return Ok(Token::Real(sign as f64 * (int_part as f64 + frac)));
        }

        Ok(Token::Integer(sign * int_part))
    }

    #[inline]
    fn peek(&self) -> u8 {
        self.buf[self.pos]
    }

    #[inline]
    fn is_eof(&self) -> bool {
        self.pos >= self.buf.len()
    }

    #[inline]
    fn bump(&mut self) {
        self.pos += 1;
    }

    fn bump_while<F: Fn(u8) -> bool>(&mut self, predicate: F) {
        while self.pos < self.buf.len() && predicate(self.peek()) {
            self.pos += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        self.bump_while(is_whitespace);
    }

    fn skip_comment(&mut self) {
        debug_assert_eq!(self.peek(), b'%');

        self.bump();
        self.bump_while(|c| c != b'\r' && c != b'\n');
        if !self.is_eof() {
            self.bump();
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        let mut has_skiped = true;
        while !self.is_eof() && has_skiped {
            has_skiped = false;
            if is_whitespace(self.peek()) {
                self.skip_whitespace();
                has_skiped = true;
            }
            if !self.is_eof() && self.peek() == b'%' {
                self.skip_comment();
                has_skiped = true;
            }
        }
    }

    pub fn read_stream_data(&mut self, length: usize) -> Result<Box<[u8]>> {
        if !self.is_eof() && self.peek() == b'\r' {
            self.bump();
        }
        if !self.is_eof() && self.peek() == b'\n' {
            self.bump();
        }

        if self.pos + length > self.buf.len() {
            return Err(PdfParserError::Eof);
        }

        let data = self.buf[self.pos..self.pos + length]
            .to_vec()
            .into_boxed_slice();
        self.pos += length;
        Ok(data)
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace_and_comments();

        if self.pos >= self.buf.len() {
            return None;
        }

        let result = match self.peek() {
            b'(' => self.parse_literal_str(),
            b'<' if self.pos + 1 < self.buf.len() && self.buf[self.pos + 1] != b'<' => {
                self.parse_hex_str()
            }
            b'/' => self.parse_name(),
            b'<' if self.pos + 1 < self.buf.len() && self.buf[self.pos + 1] == b'<' => {
                self.bump();
                self.bump();
                Ok(Token::DictBegin)
            }
            b'>' if self.pos + 1 < self.buf.len() && self.buf[self.pos + 1] == b'>' => {
                self.bump();
                self.bump();
                Ok(Token::DictEnd)
            }
            b'[' => {
                self.bump();
                Ok(Token::ArrayBegin)
            }
            b']' => {
                self.bump();
                Ok(Token::ArrayEnd)
            }
            c if matches!(c, b'+' | b'-' | b'.') || c.is_ascii_digit() => self.parse_number(),
            c if c.is_ascii_alphabetic() => self.parse_keyword(),
            c => Err(PdfParserError::InvalidByte(c, self.pos)),
        };

        Some(result)
    }
}

fn is_whitespace(c: u8) -> bool {
    matches!(c, 0 | b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

fn is_delimiter(c: u8) -> bool {
    matches!(
        c,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_when_empty_should_return_none() {
        let buf = b"";
        let mut lexer = Lexer::new(buf);

        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_next_when_only_whitespace_should_return_none() {
        let buf = b"\0\t\n\x0c\r ";
        let mut lexer = Lexer::new(buf);

        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn test_next_should_return_name_when_starting_with_slash() {
        let buf = b"  /Name other";
        let mut lexer = Lexer::new(buf);

        assert_eq!(
            lexer.next().unwrap().unwrap(),
            Token::Name((*b"Name").into())
        );
    }

    #[test]
    fn test_next_should_return_correct_name_when_valid_hex_code() {
        let buf = b"/paired#28#29parentheses";
        let mut lexer = Lexer::new(buf);

        let result = lexer.next().unwrap().unwrap();

        assert_eq!(result, Token::Name((*b"paired()parentheses").into()));
    }

    #[test]
    fn test_next_should_return_error_when_invalid_hex_code() {
        let buf = b"/paired#XXparentheses";
        let mut lexer = Lexer::new(buf);

        assert_eq!(
            lexer.next().unwrap().unwrap_err(),
            PdfParserError::InvalidByte(b'X', 8)
        );
    }

    #[test]
    fn test_integer_positive() {
        assert_eq!(Lexer::new(b"123").next(), Some(Ok(Token::Integer(123))));
    }

    #[test]
    fn test_integer_explicit_plus() {
        assert_eq!(Lexer::new(b"+17").next(), Some(Ok(Token::Integer(17))));
    }

    #[test]
    fn test_integer_negative() {
        assert_eq!(Lexer::new(b"-98").next(), Some(Ok(Token::Integer(-98))));
    }

    #[test]
    fn test_integer_zero() {
        assert_eq!(Lexer::new(b"0").next(), Some(Ok(Token::Integer(0))));
    }

    #[test]
    fn test_integer_stops_at_delimiter() {
        let mut lexer = Lexer::new(b"42/Name");
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(42))));
        assert_eq!(lexer.next(), Some(Ok(Token::Name((*b"Name").into()))));
    }

    #[test]
    fn test_real_simple() {
        assert_eq!(Lexer::new(b"34.5").next(), Some(Ok(Token::Real(34.5))));
    }

    #[test]
    fn test_real_negative() {
        assert_eq!(Lexer::new(b"-3.5").next(), Some(Ok(Token::Real(-3.5))));
    }

    #[test]
    fn test_real_explicit_plus() {
        assert_eq!(Lexer::new(b"+123.5").next(), Some(Ok(Token::Real(123.5))));
    }

    #[test]
    fn test_real_trailing_dot() {
        assert_eq!(Lexer::new(b"4.").next(), Some(Ok(Token::Real(4.0))));
    }

    #[test]
    fn test_real_leading_dot() {
        assert_eq!(Lexer::new(b".5").next(), Some(Ok(Token::Real(0.5))));
    }

    #[test]
    fn test_real_negative_leading_dot() {
        assert_eq!(Lexer::new(b"-.5").next(), Some(Ok(Token::Real(-0.5))));
    }

    #[test]
    fn test_real_zero() {
        assert_eq!(Lexer::new(b"0.0").next(), Some(Ok(Token::Real(0.0))));
    }

    #[test]
    fn test_boolean_true() {
        assert_eq!(Lexer::new(b"true").next(), Some(Ok(Token::Boolean(true))));
    }

    #[test]
    fn test_boolean_false() {
        assert_eq!(Lexer::new(b"false").next(), Some(Ok(Token::Boolean(false))));
    }

    #[test]
    fn test_boolean_stops_at_whitespace() {
        let mut lexer = Lexer::new(b"true false");
        assert_eq!(lexer.next(), Some(Ok(Token::Boolean(true))));
        assert_eq!(lexer.next(), Some(Ok(Token::Boolean(false))));
    }

    #[test]
    fn test_boolean_stops_at_delimiter() {
        let mut lexer = Lexer::new(b"true/Name");
        assert_eq!(lexer.next(), Some(Ok(Token::Boolean(true))));
        assert_eq!(lexer.next(), Some(Ok(Token::Name((*b"Name").into()))));
    }

    #[test]
    fn test_boolean_true_not_prefix_matched() {
        assert_eq!(
            Lexer::new(b"truecolor").next(),
            Some(Err(PdfParserError::UnknownKeyword(
                "truecolor".to_owned(),
                0
            )))
        );
    }

    #[test]
    fn test_null() {
        assert_eq!(Lexer::new(b"null").next(), Some(Ok(Token::Null)));
    }

    #[test]
    fn test_keyword_obj() {
        assert_eq!(
            Lexer::new(b"obj").next(),
            Some(Ok(Token::Keyword(Keyword::Obj)))
        );
    }

    #[test]
    fn test_keyword_endobj() {
        assert_eq!(
            Lexer::new(b"endobj").next(),
            Some(Ok(Token::Keyword(Keyword::EndObj)))
        );
    }

    #[test]
    fn test_keyword_stream() {
        assert_eq!(
            Lexer::new(b"stream").next(),
            Some(Ok(Token::Keyword(Keyword::Stream)))
        );
    }

    #[test]
    fn test_keyword_endstream() {
        assert_eq!(
            Lexer::new(b"endstream").next(),
            Some(Ok(Token::Keyword(Keyword::EndStream)))
        );
    }

    #[test]
    fn test_keyword_r() {
        assert_eq!(
            Lexer::new(b"R").next(),
            Some(Ok(Token::Keyword(Keyword::R)))
        );
    }

    #[test]
    fn test_keyword_xref() {
        assert_eq!(
            Lexer::new(b"xref").next(),
            Some(Ok(Token::Keyword(Keyword::XRef)))
        );
    }

    #[test]
    fn test_keyword_trailer() {
        assert_eq!(
            Lexer::new(b"trailer").next(),
            Some(Ok(Token::Keyword(Keyword::Trailer)))
        );
    }

    #[test]
    fn test_keyword_startxref() {
        assert_eq!(
            Lexer::new(b"startxref").next(),
            Some(Ok(Token::Keyword(Keyword::StartXRef)))
        );
    }

    #[test]
    fn test_keyword_unknown() {
        assert_eq!(
            Lexer::new(b"obj").next(),
            Some(Ok(Token::Keyword(Keyword::Obj)))
        );
        assert_eq!(
            Lexer::new(b"foobar").next(),
            Some(Err(PdfParserError::UnknownKeyword("foobar".to_owned(), 0)))
        );
    }

    #[test]
    fn test_keyword_stops_at_whitespace() {
        let mut lexer = Lexer::new(b"obj endobj");
        assert_eq!(lexer.next(), Some(Ok(Token::Keyword(Keyword::Obj))));
        assert_eq!(lexer.next(), Some(Ok(Token::Keyword(Keyword::EndObj))));
    }

    #[test]
    fn test_keyword_stops_at_delimiter() {
        let mut lexer = Lexer::new(b"obj/Name");
        assert_eq!(lexer.next(), Some(Ok(Token::Keyword(Keyword::Obj))));
        assert_eq!(lexer.next(), Some(Ok(Token::Name((*b"Name").into()))));
    }

    #[test]
    fn test_indirect_reference_sequence() {
        let mut lexer = Lexer::new(b"12 0 R");
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(12))));
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(0))));
        assert_eq!(lexer.next(), Some(Ok(Token::Keyword(Keyword::R))));
    }

    // --- hex strings ---

    #[test]
    fn test_hex_string_simple() {
        assert_eq!(
            Lexer::new(b"<48656C6C6F>").next(),
            Some(Ok(Token::HexString((*b"Hello").into())))
        );
    }

    #[test]
    fn test_hex_string_lowercase() {
        assert_eq!(
            Lexer::new(b"<48656c6c6f>").next(),
            Some(Ok(Token::HexString((*b"Hello").into())))
        );
    }

    #[test]
    fn test_hex_string_with_internal_whitespace() {
        assert_eq!(
            Lexer::new(b"<48 65 6C 6C 6F>").next(),
            Some(Ok(Token::HexString((*b"Hello").into())))
        );
    }

    #[test]
    fn test_hex_string_odd_length_pads_with_zero() {
        // <9> means <90> per spec
        assert_eq!(
            Lexer::new(b"<9>").next(),
            Some(Ok(Token::HexString(vec![0x90].into_boxed_slice())))
        );
    }

    #[test]
    fn test_hex_string_empty() {
        assert_eq!(
            Lexer::new(b"<>").next(),
            Some(Ok(Token::HexString(vec![].into_boxed_slice())))
        );
    }

    #[test]
    fn test_hex_string_invalid_byte() {
        assert_eq!(
            Lexer::new(b"<XY>").next(),
            Some(Err(PdfParserError::InvalidByte(b'X', 1)))
        );
    }

    #[test]
    fn test_hex_string_eof() {
        assert_eq!(Lexer::new(b"<48").next(), Some(Err(PdfParserError::Eof)));
    }

    // --- dict tokens ---

    #[test]
    fn test_dict_begin() {
        assert_eq!(Lexer::new(b"<<").next(), Some(Ok(Token::DictBegin)));
    }

    #[test]
    fn test_dict_end() {
        assert_eq!(Lexer::new(b">>").next(), Some(Ok(Token::DictEnd)));
    }

    #[test]
    fn test_dict_begin_then_end() {
        let mut lexer = Lexer::new(b"<< >>");
        assert_eq!(lexer.next(), Some(Ok(Token::DictBegin)));
        assert_eq!(lexer.next(), Some(Ok(Token::DictEnd)));
    }

    #[test]
    fn test_dict_with_entry() {
        let mut lexer = Lexer::new(b"<</Type /Page>>");
        assert_eq!(lexer.next(), Some(Ok(Token::DictBegin)));
        assert_eq!(lexer.next(), Some(Ok(Token::Name((*b"Type").into()))));
        assert_eq!(lexer.next(), Some(Ok(Token::Name((*b"Page").into()))));
        assert_eq!(lexer.next(), Some(Ok(Token::DictEnd)));
    }

    // --- array tokens ---

    #[test]
    fn test_array_begin() {
        assert_eq!(Lexer::new(b"[").next(), Some(Ok(Token::ArrayBegin)));
    }

    #[test]
    fn test_array_end() {
        assert_eq!(Lexer::new(b"]").next(), Some(Ok(Token::ArrayEnd)));
    }

    #[test]
    fn test_array_with_integers() {
        let mut lexer = Lexer::new(b"[1 2 3]");
        assert_eq!(lexer.next(), Some(Ok(Token::ArrayBegin)));
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(1))));
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(2))));
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(3))));
        assert_eq!(lexer.next(), Some(Ok(Token::ArrayEnd)));
    }

    // --- comments ---

    #[test]
    fn test_comment_skipped() {
        assert_eq!(
            Lexer::new(b"% this is a comment\ntrue").next(),
            Some(Ok(Token::Boolean(true)))
        );
    }

    #[test]
    fn test_comment_at_eof() {
        assert_eq!(Lexer::new(b"% comment at eof").next(), None);
    }

    #[test]
    fn test_comment_between_tokens() {
        let mut lexer = Lexer::new(b"1 % comment\n2");
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(1))));
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(2))));
    }

    #[test]
    fn test_multiple_comments_skipped() {
        let mut lexer = Lexer::new(b"% first\n% second\n42");
        assert_eq!(lexer.next(), Some(Ok(Token::Integer(42))));
    }
}
