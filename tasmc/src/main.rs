use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::{env, fs};

/* ========== Lexer ========== */

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

/* ========== AST + Parser ========== */

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

/* ========== Namespaces ========== */

#[derive(Clone, Debug)]
struct Namespaces {
    pub private_identifiers: HashMap<PathBuf, HashMap<String, NamespaceIdentifier>>,
    pub public_identifiers: HashMap<String, NamespaceIdentifier>,
}

#[derive(Clone, Debug)]
struct NamespaceIdentifier {
    id: IdentifierHash,
    name: String,
    idtype: IdentifierType,
}

#[derive(Clone, Debug)]
enum IdentifierType {
    Tag,
    Variable(Option<u64>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct IdentifierHash(u64);

impl Namespaces {
    fn is_private(name: &str) -> bool {
        name.len() <= 0 || name.chars().nth(0).unwrap() == '_'
    }

    fn is_public(name: &str) -> bool {
        !Self::is_private(name)
    }

    // todo: there is a small chance of identifier collision.
    // since self is borrowed as mutable, the hashing function
    // can simply be replaced by an instance-local counter.
    fn hash_private(&mut self, path: &Path, name: &str) -> IdentifierHash {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        name.hash(&mut hasher);
        IdentifierHash(hasher.finish())
    }

    // -||-
    fn hash_public(&mut self, name: &str) -> IdentifierHash {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        IdentifierHash(hasher.finish())
    }

    // todo: big problem - this isn't type checked!!! identifiers may be
    // corrupted/exchanged under the hood if they are used incorrectly.
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
            let id = self.hash_private(path, name);
            let identifiers = self
                .private_identifiers
                .entry(path.to_owned())
                .or_insert(HashMap::new());
            if identifiers.contains_key(name) {
                eprintln!("! variabel redan definerad");
                return Err(());
            }
            identifiers.insert(
                name.to_owned(),
                NamespaceIdentifier {
                    id,
                    name: name.to_owned(),
                    idtype,
                },
            );
            Ok(())
        } else {
            if self.public_identifiers.contains_key(name) {
                eprintln!("! variabel redan definerad");
                return Err(());
            }
            let id = self.hash_public(name);
            self.public_identifiers.insert(
                name.to_owned(),
                NamespaceIdentifier {
                    id,
                    name: name.to_owned(),
                    idtype,
                },
            );
            Ok(())
        }
    }
}

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

/* ========== LIR ========== */

#[derive(Debug)]
struct LIR<'a> {
    instructions: Vec<LIRInstruction>,
    labels: HashMap<LIRLabel, LIRData>,           // <label, value>
    tags: HashMap<LIRTag, usize>,                 // <label, index>
    immediate_handles: HashMap<u64, LIRLabel>,    // <value, label>
    address_handles: HashMap<LIRLabel, LIRLabel>, // <underlying, immediate>

    namespaces: &'a mut Namespaces,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct LIRLabel(IdentifierHash);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct LIRTag(IdentifierHash);

#[derive(Clone, Debug)]
struct LIRArgument {
    label: LIRLabel,
    ptr_mode: LIRPtrMode,
}

#[derive(Clone, Debug)]
enum LIRData {
    Constant(u64),
    AddressOf(LIRLabel),
}

#[derive(Clone, Debug)]
enum LIRPtrMode {
    Unset,
    Pointer,
}

#[derive(Clone, Debug)]
enum LIRInstruction {
    Mov(LIRArgument, LIRArgument),
    Add(LIRArgument, LIRArgument, LIRArgument),
    Sub(LIRArgument, LIRArgument, LIRArgument),
    Mul(LIRArgument, LIRArgument, LIRArgument),
    Divt(LIRArgument, LIRArgument, LIRArgument),
    Divr(LIRArgument, LIRArgument, LIRArgument),
    Rem(LIRArgument, LIRArgument, LIRArgument),
    Cgt(LIRArgument, LIRArgument, LIRArgument),
    Clt(LIRArgument, LIRArgument, LIRArgument),
    Jmp(LIRTag),
    Jie(LIRTag, LIRArgument),
    Jne(LIRTag, LIRArgument),
    Puti(LIRArgument),
    Putc(LIRArgument),
    Imz(LIRArgument),
    Equ(LIRArgument, LIRArgument, LIRArgument),
    Cal(LIRTag),
    Cie(LIRTag, LIRArgument),
    Cne(LIRTag, LIRArgument),
    Ret,
    Hlt,
}

macro_rules! count {
    ($type:ident) => {
        1
    };
    ($type:ident, $($types:ident),*) => {
        count!($($types),*) + 1
    };
}

macro_rules! unpack_vmstep_inner {
    ($lir:expr, $path:expr, $arg:expr, Arg) => {
        $lir.resolve_argument($arg, $path)
    };
    ($lir:expr, $path:expr, $arg:expr, Tag) => {
        $lir.resolve_tag($arg, $path)
    };
}

macro_rules! unpack_vmstep {
    ($lir:expr, $path:expr, $args:expr, $($type:ident),*) => {{
        let mut args_iter = $args.iter();
        assert_eq!(count!($($type),*), args_iter.len());
        (
            $(
                unpack_vmstep_inner!($lir, $path, args_iter.next().unwrap().clone(), $type),
            )*
        )
    }}
}

impl<'a> LIR<'a> {
    fn push_label(&mut self, label: LIRLabel, data: LIRData) {
        if self.labels.contains_key(&label) {
            panic!("[internt fel] kollision mellan dataetiketter i l.i.r.");
        }
        self.labels.insert(label, data).unwrap();
    }
    fn resolve_address(&mut self, path: &Path, addr_label: LIRLabel) -> LIRArgument {
        if let Some(id) = self.address_handles.get(&addr_label) {
            LIRArgument {
                label: id.clone(),
                ptr_mode: LIRPtrMode::Unset,
            }
        } else {
            // todo: also hacky.
            let id = self
                .namespaces
                .hash_private(path, &format!("&{}", addr_label.0.0.to_string()));
            assert!(self.labels.contains_key(&addr_label));
            self.address_handles.insert(addr_label, LIRLabel(id));
            assert!(!self.labels.contains_key(&LIRLabel(id)));
            self.labels
                .insert(LIRLabel(id), LIRData::AddressOf(addr_label));
            LIRArgument {
                label: LIRLabel(id),
                ptr_mode: LIRPtrMode::Unset,
            }
        }
    }
    fn resolve_immediate(&mut self, value: u64) -> LIRArgument {
        if let Some(id) = self.immediate_handles.get(&value) {
            LIRArgument {
                label: *id,
                ptr_mode: LIRPtrMode::Unset,
            }
        } else {
            // todo: kind of a hack for now. hash_[] should accept all hashable data as key.
            // or, a unique ID should be generated and stored in the immediate_handles.
            let id = self.namespaces.hash_public(&value.to_string());
            self.labels.insert(LIRLabel(id), LIRData::Constant(value));
            self.immediate_handles.insert(value, LIRLabel(id));
            LIRArgument {
                label: LIRLabel(id),
                ptr_mode: LIRPtrMode::Unset,
            }
        }
    }
    fn resolve_argument(&mut self, ast_arg: ASTArgument, path: &Path) -> LIRArgument {
        match ast_arg {
            ASTArgument::Tag(_) => panic!("argument förväntades, men tagg hittades"),
            ASTArgument::Variable(x) => {
                let id = self
                    .namespaces
                    .get_identifier(&x, Some(path))
                    .expect("variabeln kunde ej hittas");
                LIRArgument {
                    label: LIRLabel(id.id),
                    ptr_mode: LIRPtrMode::Unset,
                }
            }
            ASTArgument::Pointer(x) => {
                let id = self
                    .namespaces
                    .get_identifier(&x, Some(path))
                    .expect("variabeln kunde ej hittas");
                LIRArgument {
                    label: LIRLabel(id.id),
                    ptr_mode: LIRPtrMode::Pointer,
                }
            }
            ASTArgument::Address(x) => {
                let addr_id = self
                    .namespaces
                    .get_identifier(&x, Some(path))
                    .expect("variabeln kunde ej hittas");
                self.resolve_address(path, LIRLabel(addr_id.id))
            }
            ASTArgument::Immediate(x) => self.resolve_immediate(x),
        }
    }
    fn resolve_tag(&mut self, ast_arg: ASTArgument, path: &Path) -> LIRTag {
        match ast_arg {
            ASTArgument::Tag(x) => {
                let id = self
                    .namespaces
                    .get_identifier(&x, Some(path))
                    .expect("kunde ej hitta taggen");
                LIRTag(id.id)
            }
            _ => panic!("tagg förväntades, men variabel hittades"),
        }
    }
    fn lower_instruction(&mut self, path: &Path, opcode: &ASTOpcode, args: &Vec<ASTArgument>) {
        match opcode {
            ASTOpcode::Set => (),
            ASTOpcode::Mov => {
                let (src1, dst) = unpack_vmstep!(self, path, args, Arg, Arg);
                self.instructions.push(LIRInstruction::Mov(src1, dst));
            }
            ASTOpcode::Add => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions.push(LIRInstruction::Add(src1, src2, dst));
            }
            ASTOpcode::Sub => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions.push(LIRInstruction::Sub(src1, src2, dst));
            }
            ASTOpcode::Mul => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions.push(LIRInstruction::Mul(src1, src2, dst));
            }
            ASTOpcode::Divt => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions
                    .push(LIRInstruction::Divt(src1, src2, dst));
            }
            ASTOpcode::Divr => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions
                    .push(LIRInstruction::Divr(src1, src2, dst));
            }
            ASTOpcode::Rem => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions.push(LIRInstruction::Rem(src1, src2, dst));
            }
            ASTOpcode::Cgt => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions.push(LIRInstruction::Cgt(src1, src2, dst));
            }
            ASTOpcode::Clt => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions.push(LIRInstruction::Clt(src1, src2, dst));
            }
            ASTOpcode::Jmp => {
                let (tag,) = unpack_vmstep!(self, path, args, Tag);
                self.instructions.push(LIRInstruction::Jmp(tag));
            }
            ASTOpcode::Jie => {
                let (tag, src2) = unpack_vmstep!(self, path, args, Tag, Arg);
                self.instructions.push(LIRInstruction::Jie(tag, src2));
            }
            ASTOpcode::Jne => {
                let (tag, src2) = unpack_vmstep!(self, path, args, Tag, Arg);
                self.instructions.push(LIRInstruction::Jne(tag, src2));
            }
            ASTOpcode::Puti => {
                let (src1,) = unpack_vmstep!(self, path, args, Arg);
                self.instructions.push(LIRInstruction::Puti(src1));
            }
            ASTOpcode::Putc => {
                let (src1,) = unpack_vmstep!(self, path, args, Arg);
                self.instructions.push(LIRInstruction::Putc(src1));
            }
            ASTOpcode::Imz => {
                let (dst,) = unpack_vmstep!(self, path, args, Arg);
                self.instructions.push(LIRInstruction::Imz(dst));
            }
            ASTOpcode::Equ => {
                let (src1, src2, dst) = unpack_vmstep!(self, path, args, Arg, Arg, Arg);
                self.instructions.push(LIRInstruction::Equ(src1, src2, dst));
            }
            ASTOpcode::Cal => {
                let (tag,) = unpack_vmstep!(self, path, args, Tag);
                self.instructions.push(LIRInstruction::Cal(tag));
            }
            ASTOpcode::Cie => {
                let (tag, src2) = unpack_vmstep!(self, path, args, Tag, Arg);
                self.instructions.push(LIRInstruction::Cie(tag, src2));
            }
            ASTOpcode::Cne => {
                let (tag, src2) = unpack_vmstep!(self, path, args, Tag, Arg);
                self.instructions.push(LIRInstruction::Cne(tag, src2));
            }
            ASTOpcode::Ret => {
                assert_eq!(args.len(), 0);
                self.instructions.push(LIRInstruction::Ret);
            }
            ASTOpcode::Hlt => {
                assert_eq!(args.len(), 0);
                self.instructions.push(LIRInstruction::Ret);
            }
        }
    }
    pub fn generate(sources: &Vec<Source>, namespaces: &'a mut Namespaces) -> Result<LIR<'a>, ()> {
        let mut lir = LIR {
            instructions: vec![],
            tags: HashMap::new(),
            labels: HashMap::new(),
            immediate_handles: HashMap::new(),
            address_handles: HashMap::new(),
            namespaces,
        };
        // add resolved labels.
        for map_id in lir.namespaces.private_identifiers.values() {
            for id in map_id.values() {
                if let IdentifierType::Variable(value) = id.idtype {
                    lir.labels
                        .insert(LIRLabel(id.id), LIRData::Constant(value.unwrap_or(0)));
                }
            }
        }
        for id in lir.namespaces.public_identifiers.values() {
            if let IdentifierType::Variable(value) = id.idtype {
                lir.labels
                    .insert(LIRLabel(id.id), LIRData::Constant(value.unwrap_or(0)));
            }
        }
        // sources have to be lowered separately still.
        // this is because the identifiers aren't fully resolved yet.
        for source in sources {
            for instruction in &source.ast.instructions {
                if let ASTInstruction::Tag(tag) = instruction {
                    let tag_id = lir
                        .namespaces
                        .get_identifier(tag, Some(&source.path))
                        .expect("tagg existerar ej")
                        .id;
                    // this computes where the tag will point to.
                    lir.tags.insert(LIRTag(tag_id), lir.instructions.len());
                } else if let ASTInstruction::VMStep(opcode, args) = instruction {
                    lir.lower_instruction(&source.path, opcode, args);
                }
            }
        }
        Ok(lir)
    }
}

/* ========== Driver ========== */

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

    println!("tasmc: [källor] löser namnrymd");
    let mut namespaces: Namespaces = (&sources).try_into().expect("lyckades inte lösa namnrymd");

    println!("tasmc: [källor] genererar lågnivåstruktur");
    let lir =
        LIR::generate(&sources, &mut namespaces).expect("lyckades inte generera lågnivåstruktur");

    // dbg!(&namespaces);
    //dbg!(&lir);
    Ok(())
}
