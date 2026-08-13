use crate::errors::{ErrorData, IndexReference, error};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum TokenData {
    Hashtag,
    Identifier(String),
    Immediate(u64),
    /* Strings, */
    RAngleBracket,
    Ampersand,
    Lib,
    Newline,
    EOF,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub inner: TokenData,
    pub reference: IndexReference,
}

impl Token {
    pub fn new(inner: TokenData, reference: IndexReference) -> Self {
        Token { inner, reference }
    }
}

pub struct Lexer {
    chars: Vec<char>,
    index: usize,
    path: PathBuf,
}

impl Lexer {
    pub fn new(data: &String, path: PathBuf) -> Lexer {
        Lexer {
            chars: data.chars().collect(),
            index: 0,
            path,
        }
    }
    fn advance(&mut self) -> Option<char> {
        if self.index >= self.chars.len() {
            None
        } else {
            self.index += 1;
            Some(self.chars[self.index - 1])
        }
    }
    fn peek(&self) -> Option<char> {
        if self.index >= self.chars.len() {
            None
        } else {
            Some(self.chars[self.index])
        }
    }
    fn is_valid_identifier(c: char) -> bool {
        c.is_alphanumeric() || c == '/' || c == '.' || c == '_' || c == '-'
    }
    fn lex_token(&mut self) -> Result<Token, ErrorData> {
        let mut lexeme;
        if let Some(char_read) = self.advance() {
            if char_read == '\n' {
                Ok(Token::new(
                    TokenData::Newline,
                    IndexReference::new(self.path.clone(), self.index - 1, 1),
                ))
            } else if char_read.is_whitespace() {
                while let Some(c) = self.peek()
                    && c.is_whitespace()
                    && c != '\n'
                {
                    self.advance();
                }
                self.lex_token()
            } else if char_read == '/'
                && self.peek().is_some()
                && self.peek().unwrap() == '/'
            {
                self.advance(); // consume last '/'
                while let Some(c) = self.peek()
                    && c != '\n'
                {
                    match self.advance() {
                        Some(_) => (),
                        None => {
                            return Ok(Token::new(
                                TokenData::EOF,
                                IndexReference::new(
                                    self.path.clone(),
                                    self.index - 1,
                                    0,
                                ),
                            ));
                        }
                    }
                }
                self.lex_token()
            } else if char_read.is_ascii_digit() {
                let start_index = self.index - 1;
                lexeme = char_read.to_string();
                while let Some(c) = self.peek()
                    && c.is_ascii_digit()
                {
                    lexeme += &self.advance().unwrap().to_string();
                }
                if let Ok(x) = str::parse::<u64>(&lexeme) {
                    Ok(Token::new(
                        TokenData::Immediate(x),
                        IndexReference::new(
                            self.path.clone(),
                            start_index,
                            self.index - start_index,
                        ),
                    ))
                } else {
                    Err(error!(
                        InvalidNumberLexeme,
                        IndexReference::new(
                            self.path.clone(),
                            start_index,
                            self.index - start_index,
                        ),
                        lexeme
                    ))
                }
            } else if char_read == '#' {
                Ok(Token::new(
                    TokenData::Hashtag,
                    IndexReference::new(self.path.clone(), self.index - 1, 1),
                ))
            } else if char_read == '>' {
                Ok(Token::new(
                    TokenData::RAngleBracket,
                    IndexReference::new(self.path.clone(), self.index - 1, 1),
                ))
            } else if char_read == '&' {
                Ok(Token::new(
                    TokenData::Ampersand,
                    IndexReference::new(self.path.clone(), self.index - 1, 1),
                ))
            } else if char_read == '*' {
                let start_index = self.index - 1;
                let identifier = self.lex_token();
                if let Ok(x) = identifier
                    && let TokenData::Identifier(x) = x.inner
                    && x == "lib"
                {
                    Ok(Token::new(
                        TokenData::Lib,
                        IndexReference::new(
                            self.path.clone(),
                            start_index,
                            self.index - start_index,
                        ),
                    ))
                } else {
                    Err(error!(
                        InvalidCompilerOperation,
                        IndexReference::new(
                            self.path.clone(),
                            start_index,
                            self.index - start_index
                        )
                    ))
                }
            } else if Self::is_valid_identifier(char_read) {
                let start_index = self.index - 1;
                lexeme = char_read.to_string();
                while let Some(c) = self.peek()
                    && Self::is_valid_identifier(c)
                {
                    lexeme += &self.advance().unwrap().to_string();
                }
                Ok(Token::new(
                    TokenData::Identifier(lexeme),
                    IndexReference::new(
                        self.path.clone(),
                        start_index,
                        self.index - start_index,
                    ),
                ))
            } else {
                Err(error!(
                    InvalidInputByte,
                    IndexReference {
                        path: self.path.clone(),
                        index: self.index - 1,
                        len: 1,
                    },
                    char_read
                ))
            }
        } else {
            Ok(Token::new(
                TokenData::EOF,
                IndexReference::new(self.path.clone(), self.index, 0),
            ))
        }
    }
    pub fn lex(&mut self) -> Result<Vec<Token>, Vec<ErrorData>> {
        let mut tokens: Vec<Token> = vec![];
        let mut error_data: Vec<ErrorData> = vec![];
        loop {
            match self.lex_token() {
                Ok(token) => {
                    tokens.push(token.clone());
                    if let TokenData::EOF = &token.inner {
                        break;
                    }
                }
                Err(err) => {
                    error_data.push(err);
                }
            }
        }

        if error_data.is_empty() {
            Ok(tokens)
        } else {
            Err(error_data)
        }
    }
}
