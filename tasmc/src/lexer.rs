#[derive(Clone, Debug)]
pub enum Token {
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

pub struct Lexer {
    chars: Vec<char>,
    index: usize,
}

impl Lexer {
    pub fn new(data: &String) -> Lexer {
        Lexer {
            chars: data.chars().collect(),
            index: 0,
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
    fn lex_token(&mut self) -> Result<Token, ()> {
        let mut lexeme;
        if let Some(mut char_read) = self.advance() {
            if char_read == '\n' {
                Ok(Token::Newline)
            } else if char_read.is_whitespace() {
                while let Some(c) = self.peek()
                    && c.is_whitespace()
                {
                    match self.advance() {
                        Some(_) => (),
                        None => return Ok(Token::EOF),
                    };
                    if let Some(x) = self.peek()
                        && !x.is_whitespace()
                    {
                        break;
                    }
                }
                self.lex_token()
            } else if char_read == '/' && self.peek().is_some() && self.peek().unwrap() == '/' {
                self.advance(); // consume last '/'
                while let Some(c) = self.peek() && c != '\n' {
                    match self.advance() {
                        Some(_) => (),
                        None => return Ok(Token::EOF),
                    }
                }
                self.lex_token()
            } else if char_read.is_ascii_digit() {
                lexeme = char_read.to_string();
                while let Some(c) = self.peek()
                    && c.is_ascii_digit()
                {
                    lexeme += &self.advance().unwrap().to_string();
                }
                if let Ok(x) = str::parse::<u64>(&lexeme) {
                    Ok(Token::Immediate(x))
                } else {
                    eprintln!("! ett tal kunde ej härledas ur data {}", lexeme);
                    Err(())
                }
            } else if char_read == '#' {
                Ok(Token::Hashtag)
            } else if char_read == '>' {
                Ok(Token::RAngleBracket)
            } else if char_read == '&' {
                Ok(Token::Ampersand)
            } else if char_read == '*' {
                let identifier = self.lex_token();
                if let Ok(x) = identifier
                    && let Token::Identifier(x) = x
                    && x == "lib"
                {
                    Ok(Token::Lib)
                } else {
                    eprintln!("! kompilatorkommando `*` måste i nuläget följas av `lib`");
                    Err(())
                }
            } else if Self::is_valid_identifier(char_read) {
                lexeme = char_read.to_string();
                while let Some(c) = self.peek()
                    && Self::is_valid_identifier(c)
                {
                    lexeme += &self.advance().unwrap().to_string();
                }
                Ok(Token::Identifier(lexeme))
            } else {
                eprintln!(
                    "! förstod ej symbol vid index {} med data {}",
                    self.index, char_read
                );
                Err(())
            }
        } else {
            Ok(Token::EOF)
        }
    }
    pub fn lex(&mut self) -> Result<Vec<Token>, ()> {
        let mut token: Token;
        let mut tokens: Vec<Token> = vec![];
        loop {
            token = self.lex_token()?;
            tokens.push(token.clone());
            if let Token::EOF = &token {
                break;
            }
        }
        Ok(tokens)
    }
}
