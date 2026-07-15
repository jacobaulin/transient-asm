use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
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
}

impl AST {
    pub fn new() -> Self {
        AST {
            libraries: vec![],
            instructions: vec![],
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
                                Token::EOF => {
                                    self.advance();
                                    break;
                                }
                                Token::Newline => {
                                    self.advance();
                                    break;
                                }
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

#[derive(Clone, Debug)]
struct IR {}

#[derive(Clone, Debug)]
struct Namespaces {
    pub private_identifiers: HashMap<PathBuf, HashMap<String, NamespaceIdentifier>>,
    pub public_identifiers: HashMap<String, NamespaceIdentifier>,
}

#[derive(Clone, Debug)]
struct NamespaceIdentifier {
    id: u64,
    name: String,
    idtype: IdentifierType,
}

#[derive(Clone, Debug)]
enum IdentifierType {
    Tag,
    Variable(Option<u64>),
}

impl Namespaces {
    fn is_private(name: &str) -> bool {
        name.len() <= 0 || name.chars().nth(0).unwrap() == '_'
    }
    fn is_public(name: &str) -> bool {
        !Self::is_private(name)
    }
    fn hash_private(path: &Path, name: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        name.hash(&mut hasher);
        hasher.finish()
    }
    fn hash_public(name: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish()
    }
    pub fn get_identifier(
        &self,
        name: &str,
        path: Option<&Path>,
    ) -> Result<NamespaceIdentifier, ()> {
        // private identifier resolution.
        if Self::is_private(name)
            && let Some(path) = path
        {
            if let Some(identifiers) = self.private_identifiers.get(path)
                && let Some(identifier) = identifiers.get(name)
            {
                return Ok(identifier.clone());
            }
        }

        // public identifier resolution.
        if let Some(identifier) = self.public_identifiers.get(name) {
            return Ok(identifier.clone());
        }

        // identifier does not exist.
        Err(())
    }
    // fails if identifier is already set.
    fn set_identifier(
        &mut self,
        name: &str,
        path: &Path,
        idtype: IdentifierType,
    ) -> Result<(), ()> {
        if Self::is_private(name) {
            let identifiers = self
                .private_identifiers
                .entry(path.to_owned())
                .or_insert(HashMap::new());
            if identifiers.contains_key(name) {
                return Err(());
            }
            identifiers.insert(
                name.to_owned(),
                NamespaceIdentifier {
                    id: Self::hash_private(path, name),
                    name: name.to_owned(),
                    idtype,
                },
            );
            Ok(())
        } else {
            if self.public_identifiers.contains_key(name) {
                return Err(());
            }
            self.public_identifiers.insert(
                name.to_owned(),
                NamespaceIdentifier {
                    id: Self::hash_public(name),
                    name: name.to_owned(),
                    idtype,
                },
            );
            Ok(())
        }
    }
}

// att göra: kontrollera så att identifierare inte dupliceras / defineras flera gånger.
//           slutföra variabler.
impl TryFrom<&Vec<Source>> for Namespaces {
    type Error = ();
    fn try_from(sources: &Vec<Source>) -> Result<Self, Self::Error> {
        let mut namespaces = Namespaces {
            private_identifiers: HashMap::new(),
            public_identifiers: HashMap::new(),
        };

        for source in sources {
            for instruction in &source.ast.instructions {
                match instruction.clone() {
                    ASTInstruction::Tag(x) => {
                        namespaces.set_identifier(&x, &source.path, IdentifierType::Tag)?;
                    }
                    ASTInstruction::VMStep(opcode, args) => {
                        match opcode {
                            ASTOpcode::Set => (),
                            _ => continue,
                        }
                        if let Some(arg1) = args.get(0)
                            && let ASTArgument::Variable(name) = arg1
                        {
                            let value: Option<u64> = match args.get(1) {
                                None => None,
                                Some(ASTArgument::Immediate(x)) => Some(*x),
                                _ => {
                                    eprintln!("! ogiltigt initieringsvärde för `set`");
                                    return Err(());
                                }
                            };
                            if args.len() > 2 {
                                eprintln!("! överflödig data efter `set`");
                                return Err(());
                            }
                            namespaces.set_identifier(
                                &name,
                                &source.path,
                                IdentifierType::Variable(value),
                            )?;
                        } else {
                            eprintln!("! `set` måste efterföljas av en giltig identifierare");
                            return Err(());
                        }
                    }
                }
            }
        }

        Ok(namespaces)
    }
}

fn import_library(path: &Path, state: Option<&Vec<Source>>) -> Vec<Source> {
    // scan state to prevent library link duplication and/or recursion.
    if let Some(state) = state {
        for source in state {
            if source.path == path {
                return vec![];
            }
        }
    }
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

        // state is used for detecting duplicated or recursive library links.
        let mut child_state = sources.clone();
        if let Some(state) = state {
            child_state.extend(state.clone());
        }

        sources.extend(
            import_library(&lib_path, Some(&child_state))
                .iter()
                .cloned(),
        );
    }
    sources
}

fn main() -> Result<(), ()> {
    println!("< tasmc >");
    let path: String = env::args().collect::<Vec<String>>()[1].clone();
    let path: PathBuf = fs::canonicalize(path).expect("lyckades inte normalisera källfil");

    let sources: Vec<Source> = import_library(&path, None);

    dbg!(&sources);

    println!("tasmc: [källor] löser namnrymd");
    let namespaces: Namespaces = (&sources).try_into()?;

    dbg!(&namespaces);
    Ok(())
}
