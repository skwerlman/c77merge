use std::fmt::Debug;
use std::io::{self, Read};

use tracing::instrument;

use crate::error::{ParseError, Result};
use crate::token::AcfToken;

#[derive(Debug)]
pub struct AcfTokenStream<R: Read + Debug> {
    read: R,
}
impl<R: Read + Debug> Iterator for AcfTokenStream<R> {
    type Item = Result<AcfToken>;
    fn next(&mut self) -> Option<Result<AcfToken>> {
        self.try_next().transpose()
    }
}
impl<R: Read + Debug> AcfTokenStream<R> {
    #[instrument(level = "trace", ret)]
    pub fn new(read: R) -> Self {
        Self { read }
    }

    #[instrument(level = "trace", ret)]
    pub fn try_next(&mut self) -> Result<Option<AcfToken>> {
        Ok(match self.next_non_whitespace_char()? {
            Some('{') => Some(AcfToken::DictStart),
            Some('}') => Some(AcfToken::DictEnd),
            Some('"') => self.parse_str()?,
            Some(c) => Err(ParseError::UnexpectedCharacter(c))?,
            None => None,
        })
    }

    // TODO: handle UTF-8 better, possibly by making this work with bytes and letting parse_str handle it
    #[instrument(level = "trace", ret)]
    fn next_char(&mut self) -> io::Result<Option<char>> {
        let mut buf: [u8; 1] = [0];
        Ok(if self.read.read(&mut buf)? == 1 {
            Some(buf[0] as char)
        } else {
            None
        })
    }

    #[instrument(level = "trace", ret)]
    fn next_non_whitespace_char(&mut self) -> Result<Option<char>, ParseError> {
        while let Some(c) = self.next_char()? {
            if !c.is_whitespace() {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    #[instrument(level = "trace", ret)]
    fn parse_str(&mut self) -> Result<Option<AcfToken>, ParseError> {
        let mut buf = String::new();
        loop {
            match self.next_char()? {
                Some('"') => return Ok(Some(AcfToken::String(buf))),
                // TODO: handle escape sequences?
                Some(c) => buf.push(c),
                None => return Err(ParseError::UnterminatedString),
            }
        }
    }
}
