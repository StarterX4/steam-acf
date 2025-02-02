use std::io::{self, Read};

use thiserror::Error;


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcfToken {
    String(String),
    DictStart,
    DictEnd,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Generic I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Unexpected Character '{0:?}'")]
    UnexpectedCharacter(char),
    #[error("Unterminated String literal")]
    UnterminatedString,
    #[error("Unexpected EOF")]
    UnexpectedEof,
}

type Res<A> = Result<A, ParseError>;

pub struct AcfTokenStream<R> {
    read: R,
}
impl<R: Read> Iterator for AcfTokenStream<R> {
    type Item = Res<AcfToken>;
    fn next(&mut self) -> Option<Res<AcfToken>> {
        self.try_next().transpose()
    }
}
impl<R: Read> AcfTokenStream<R> {
    pub fn new(read: R) -> Self {
        Self { read }
    }

    pub fn try_next(&mut self) -> Res<Option<AcfToken>> {
        Ok(match self.next_non_whitespace_char()? {
            Some('{') => Some(AcfToken::DictStart),
            Some('}') => Some(AcfToken::DictEnd),
            Some('"') => self.parse_str()?,
            Some(c) => {
                Err(ParseError::UnexpectedCharacter(c))?
            },
            None => None,
        })
    }

    fn next_char(&mut self) -> io::Result<Option<char>> {
        let mut buf = [0; 4]; // Buffer to hold up to 4 bytes (max UTF-8 char size)
        let mut bytes_read = 0;

        // Read bytes until we have a complete UTF-8 character or reach EOF
        loop {
            match self.read.read(&mut buf[bytes_read..bytes_read + 1]) {
                Ok(1) => bytes_read += 1,
                Ok(0) => break, // EOF
                Err(e) => return Err(e),
            }

            // Check if we have a complete UTF-8 character
            match str::from_utf8(&buf[..bytes_read]) {
                Ok(s) => return Ok(s.chars().next()), // Return the first char
                Err(_) => {
                    if bytes_read == 4 {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "Invalid UTF-8 sequence",
                        ));
                    } // Invalid UTF-8, but we read less than 4 bytes, so we read more
                }
            }
        }

        Ok(None) // EOF
     }

    fn next_non_whitespace_char(&mut self) -> io::Result<Option<char>> {
        while let Some(c) = self.next_char()? {
            if !c.is_whitespace() {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    fn parse_str(&mut self) -> Res<Option<AcfToken>> {
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
