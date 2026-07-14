use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Clone, Debug)]
enum Token {
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

struct Lexer {
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
                match self.advance() {
                    Some(x) => char_read = x,
                    None => return Ok(Token::EOF),
                }
                while char_read != '\n' {
                    match self.advance() {
                        Some(x) => char_read = x,
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

#[derive(Debug, Clone)]
struct AST {
    pub libraries: Vec<String>,
    pub instructions: Vec<ASTInstruction>,
    pub variables: Vec<String>,
}

impl AST {
    pub fn new() -> Self {
        AST {
            libraries: vec![],
            instructions: vec![],
            variables: vec![],
        }
    }
}

#[derive(Debug, Clone)]
enum ASTInstruction {
    VMStep(ASTOpcode, Vec<ASTArgument>),
    Tag(String), // identifier
}

#[derive(Debug, Clone)]
enum ASTArgument {
    Variable(String), // identifier
    Pointer(String),  // identifier
    Address(String),  // identifier
    Tag(String),      // identifier
    Immediate(u64),   // value
}

#[derive(Debug, Clone)]
enum ASTOpcode {
    Mov,
    Add,
    Sub,
    Mul,
    Divt,
    Divr,
    Rem,
    Cgt,
    Clt,
    Jmp,
    Jie,
    Jne,
    Puti,
    Putc,
    Imz,
    Equ,
    Cal,
    Cie,
    Cne,
    Ret,
    Hlt,
    Set,
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }
    fn advance(&mut self) -> Option<Token> {
        if self.index >= self.tokens.len() {
            None
        } else {
            self.index += 1;
            Some(self.tokens[self.index - 1].clone())
        }
    }
    fn peek(&self) -> Option<Token> {
        if self.index >= self.tokens.len() {
            None
        } else {
            Some(self.tokens[self.index].clone())
        }
    }
    fn parse_argument(&mut self) -> Result<ASTArgument, ()> {
        if let Some(token) = self.advance() {
            match token {
                Token::Immediate(x) => Ok(ASTArgument::Immediate(x)),
                Token::Identifier(x) => Ok(ASTArgument::Variable(x)),
                Token::RAngleBracket => {
                    if let Some(token) = self.advance()
                        && let Token::Identifier(x) = token
                    {
                        Ok(ASTArgument::Pointer(x))
                    } else {
                        eprintln!("! en pekare måste följas av en identifierare");
                        Err(())
                    }
                }
                Token::Ampersand => {
                    if let Some(token) = self.advance()
                        && let Token::Identifier(x) = token
                    {
                        Ok(ASTArgument::Address(x))
                    } else {
                        eprintln!("! en address måste följas av en identifierare");
                        Err(())
                    }
                }
                Token::Hashtag => {
                    if let Some(token) = self.advance()
                        && let Token::Identifier(x) = token
                    {
                        Ok(ASTArgument::Tag(x))
                    } else {
                        eprintln!("! en tagg måste följas av en identifierare");
                        Err(())
                    }
                }
                _ => {
                    eprintln!("! ogiltigt argument upptäcktes");
                    Err(())
                }
            }
        } else {
            eprintln!("! ett argument förväntades");
            Err(())
        }
    }
    pub fn parse(&mut self) -> Result<AST, ()> {
        let mut ast: AST = AST::new();
        loop {
            if let Some(token) = self.advance() {
                match token {
                    Token::Newline => (),
                    Token::EOF => break Ok(ast),
                    Token::Hashtag => {
                        let identifier = self.advance();
                        if let None = identifier {
                            eprintln!("! `#` måste följas av en giltig identifierare");
                            break Err(());
                        }
                        match identifier.unwrap() {
                            Token::Identifier(x) => ast.instructions.push(ASTInstruction::Tag(x)),
                            _ => {
                                eprintln!("! `#` måste följas av en giltig identifierare");
                                break Err(());
                            }
                        }

                        let newline = self
                            .advance()
                            .expect("oväntat slut på lexikografisk data under syntaxanalys");
                        if let Token::Newline = newline {
                            continue;
                        }
                        eprintln!("! överflödiga data påträffat efter `#`");
                        break Err(());
                    }
                    Token::Lib => {
                        let identifier = self.advance();
                        if let None = identifier {
                            eprintln!("! bibliotekslänkar måste följas av en giltig identifierare");
                            break Err(());
                        }
                        match identifier.unwrap() {
                            Token::Identifier(x) => ast.libraries.push(x),
                            _ => {
                                eprintln!(
                                    "! bibliotekslänkar måste följas av en giltig identifierare"
                                );
                                break Err(());
                            }
                        }

                        let newline = self
                            .advance()
                            .expect("oväntat slut på lexikografisk data under syntaxanalys");
                        if let Token::Newline = newline {
                            continue;
                        }
                        eprintln!("! överflödiga data påträffat efter bibliotekslänk");
                        break Err(());
                    }
                    Token::Identifier(x) => {
                        let opcode: ASTOpcode = match x.as_str() {
                            "mov" => ASTOpcode::Mov,
                            "add" => ASTOpcode::Add,
                            "sub" => ASTOpcode::Sub,
                            "mul" => ASTOpcode::Mul,
                            "divt" => ASTOpcode::Divt,
                            "divr" => ASTOpcode::Divr,
                            "rem" => ASTOpcode::Rem,
                            "cgt" => ASTOpcode::Cgt,
                            "clt" => ASTOpcode::Clt,
                            "jmp" => ASTOpcode::Jmp,
                            "jie" => ASTOpcode::Jie,
                            "jne" => ASTOpcode::Jne,
                            "puti" => ASTOpcode::Puti,
                            "putc" => ASTOpcode::Putc,
                            "imz" => ASTOpcode::Imz,
                            "equ" => ASTOpcode::Equ,
                            "cal" => ASTOpcode::Cal,
                            "cie" => ASTOpcode::Cie,
                            "cne" => ASTOpcode::Cne,
                            "ret" => ASTOpcode::Ret,
                            "hlt" => ASTOpcode::Hlt,
                            "set" => ASTOpcode::Set,
                            _ => {
                                eprintln!("! ogiltig operationskod `{}`", x);
                                break Err(());
                            }
                        };
                        let mut args: Vec<ASTArgument> = vec![];
                        loop {
                            let peek = self.peek().expect("");
                            match peek {
                                Token::EOF => break,
                                Token::Newline => break,
                                _ => (),
                            }
                            args.push(self.parse_argument()?);
                        }
                        ast.instructions.push(ASTInstruction::VMStep(opcode, args));
                    }
                    _ => {
                        eprintln!(
                            "! felaktiga data påträffat. tasmc stödjer enbart instruktioner och taggar"
                        );
                        break Err(());
                    }
                }
            } else {
                eprintln!("! ogiltigt slut på källfilen");
                break Err(());
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Source {
    path: PathBuf,
    ast: AST,
}

fn import_library(path: &Path) -> Vec<Source> {
    println!("tasmc: [{:?} | 1/4] läser fil.", &path.file_name().unwrap());
    let src = fs::read_to_string(&path).expect("felaktig bibliotekssökväg!");

    println!(
        "tasmc: [{:?} | 2/4] utför lexikografisk analys.",
        &path.file_name().unwrap()
    );
    let mut lexer = Lexer::new(&src);
    let tokens = lexer
        .lex()
        .expect("fel: lexikografisk analys misslyckades.");

    println!(
        "tasmc: [{:?} | 3/4] utför syntaktisk analys.",
        &path.file_name().unwrap()
    );
    let mut parser = Parser::new(tokens.clone());
    let ast: AST = parser
        .parse()
        .expect("fel: syntaktisk analys misslyckades.");

    println!(
        "tasmc: [{:?} | 4/4] länkar bibliotek.",
        &path.file_name().unwrap()
    );
    let mut sources: Vec<Source> = vec![Source {
        path: PathBuf::from(&path),
        ast: ast.clone(),
    }];
    for library in &ast.libraries {
        let mut lib_path: PathBuf = path.into();
        lib_path.pop();
        lib_path.extend(PathBuf::from(library).iter());
        sources.extend(import_library(&lib_path).iter().cloned());
    }
    sources
}

fn main() -> Result<(), ()> {
    println!("< tasmc >");
    let path: String = env::args().collect::<Vec<String>>()[1].clone();
    let path: PathBuf = fs::canonicalize(path).expect("lyckades inte normalisera källfil");

    let sources: Vec<Source> = import_library(&path);

    dbg!(sources);
    Ok(())
}
