use crate::errors::{ErrorData, IndexReference, error};
use crate::lexer::{Token, TokenData};

#[derive(Debug, Clone)]
pub struct AST {
    pub libraries: Vec<ASTLibraryLink>,
    pub instructions: Vec<ASTInstruction>,
}

#[derive(Debug, Clone)]
pub struct ASTLibraryLink {
    pub path: String,
    pub reference: IndexReference,
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
pub enum ASTInstructionData {
    VMStep(ASTOpcode, Vec<ASTArgument>),
    Tag(String), // identifier
}

#[derive(Debug, Clone)]
pub struct ASTInstruction {
    pub inner: ASTInstructionData,
    pub reference: IndexReference,
}

impl ASTInstruction {
    pub fn new(inner: ASTInstructionData, reference: IndexReference) -> Self {
        Self { inner, reference }
    }
}

macro_rules! ast_instruction {
    ($name:ident $(($($data:expr),*))?, $reference:expr) => {
        ASTInstruction::new(ASTInstructionData::$name $(($($data),*))?, $reference)
    }
}

#[derive(Debug, Clone)]
pub enum ASTArgumentData {
    Variable(String), // identifier
    Pointer(String),  // identifier
    Address(String),  // identifier
    Tag(String),      // identifier
    Immediate(u64),   // value
}

#[derive(Debug, Clone)]
pub struct ASTArgument {
    pub inner: ASTArgumentData,
    pub reference: IndexReference,
}

impl ASTArgument {
    pub fn new(inner: ASTArgumentData, reference: IndexReference) -> Self {
        Self { inner, reference }
    }
}

macro_rules! ast_argument {
    ($name:ident $(($($data:expr),*))?, $reference:expr) => {
        ASTArgument::new(ASTArgumentData::$name $(($($data),*))?, $reference)
    }
}

#[derive(Debug, Clone)]
pub enum ASTOpcodeData {
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

#[derive(Clone, Debug)]
pub struct ASTOpcode {
    pub inner: ASTOpcodeData,
    pub reference: IndexReference,
}

impl ASTOpcode {
    pub fn new(inner: ASTOpcodeData, reference: IndexReference) -> Self {
        Self { inner, reference }
    }
}

macro_rules! ast_opcode {
    ($name:ident $(($($data:expr),*))?, $reference:expr) => {
        ASTOpcode::new(ASTOpcodeData::$name $(($($data),*))?, $reference)
    }
}

pub struct Parser {
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
    fn previous(&self) -> Token {
        self.tokens[self.index - 1].clone()
    }
    fn synchronise_lossy(&mut self) {
        while self.peek().is_some() {
            match self.peek() {
                Some(Token {
                    inner: TokenData::Newline,
                    ..
                }) => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    self.advance();
                }
                None => (),
            }
        }
    }
    fn parse_argument(
        &mut self,
        opcode_reference: &IndexReference,
    ) -> Result<ASTArgument, ErrorData> {
        if let Some(token) = self.advance() {
            match token.inner {
                TokenData::Immediate(x) => {
                    Ok(ast_argument!(Immediate(x), token.reference))
                }
                TokenData::Identifier(x) => {
                    Ok(ast_argument!(Variable(x), token.reference))
                }
                TokenData::RAngleBracket => {
                    if let Some(token) = self.advance()
                        && let TokenData::Identifier(x) = token.inner
                    {
                        Ok(ast_argument!(Pointer(x), token.reference))
                    } else {
                        Err(error!(MissingPointerIdentifier, token.reference))
                    }
                }
                TokenData::Ampersand => {
                    if let Some(token) = self.advance()
                        && let TokenData::Identifier(x) = token.inner
                    {
                        Ok(ast_argument!(Address(x), token.reference))
                    } else {
                        Err(error!(MissingAddressIdentifier, token.reference))
                    }
                }
                TokenData::Hashtag => {
                    if let Some(token) = self.advance()
                        && let TokenData::Identifier(x) = token.inner
                    {
                        Ok(ast_argument!(Tag(x), token.reference))
                    } else {
                        Err(error!(MissingTagIdentifier, token.reference))
                    }
                }
                _ => Err(error!(InvalidArgumentSymbol, token.reference)),
            }
        } else {
            Err(error!(ArgumentExpected, opcode_reference.clone()))
        }
    }
    pub fn parse(&mut self) -> Result<AST, Vec<ErrorData>> {
        let mut ast: AST = AST::new();
        let mut error_data: Vec<ErrorData> = vec![];
        loop {
            if let Some(token) = self.advance() {
                match token.inner {
                    TokenData::Newline => (),
                    TokenData::EOF => {
                        if error_data.is_empty() {
                            break Ok(ast);
                        } else {
                            break Err(error_data);
                        }
                    }
                    TokenData::Hashtag => {
                        let identifier = self.advance();
                        if let None = identifier {
                            error_data.push(error!(
                                MissingTagIdentifier,
                                token.reference
                            ));
                            self.synchronise_lossy();
                            continue;
                        }
                        match identifier.clone().unwrap().inner {
                            TokenData::Identifier(x) => {
                                ast.instructions.push(ast_instruction!(
                                    Tag(x),
                                    identifier.unwrap().reference
                                ));
                            }
                            _ => {
                                error_data.push(error!(
                                    MissingTagIdentifier,
                                    token.reference
                                ));
                                self.synchronise_lossy();
                                continue;
                            }
                        }

                        let newline = self
                            .advance()
                            .expect("oväntat slut på lexikografisk data under syntaxanalys"); // todo:
                        // tänk igenom mer ordentligt
                        if let TokenData::Newline = newline.inner {
                            continue;
                        }
                        error_data
                            .push(error!(TrailingTagData, newline.reference));
                        self.synchronise_lossy();
                        continue;
                    }
                    TokenData::Lib => {
                        let identifier = self.advance();
                        if let None = identifier {
                            error_data.push(error!(
                                MissingLibraryIdentifier,
                                token.reference
                            ));
                            self.synchronise_lossy();
                            continue;
                        }
                        let identifier = identifier.unwrap();
                        match identifier.inner {
                            TokenData::Identifier(path) => {
                                ast.libraries.push(ASTLibraryLink {
                                    path,
                                    reference: identifier.reference,
                                })
                            }
                            _ => {
                                error_data.push(error!(
                                    MissingLibraryIdentifier,
                                    token.reference
                                ));
                                self.synchronise_lossy();
                                continue;
                            }
                        }

                        let newline = self
                            .advance()
                            .expect("oväntat slut på lexikografisk data under syntaxanalys");
                        if let TokenData::Newline = newline.inner {
                            continue;
                        }
                        error_data.push(error!(
                            TrailingLibraryData,
                            newline.reference
                        ));
                        self.synchronise_lossy();
                        continue;
                    }
                    TokenData::Identifier(x) => {
                        let opcode: ASTOpcode = match x.as_str() {
                            "mov" => ast_opcode!(Mov, token.reference.clone()),
                            "add" => ast_opcode!(Add, token.reference.clone()),
                            "sub" => ast_opcode!(Sub, token.reference.clone()),
                            "mul" => ast_opcode!(Mul, token.reference.clone()),
                            "divt" => {
                                ast_opcode!(Divt, token.reference.clone())
                            }
                            "divr" => {
                                ast_opcode!(Divr, token.reference.clone())
                            }
                            "rem" => ast_opcode!(Rem, token.reference.clone()),
                            "cgt" => ast_opcode!(Cgt, token.reference.clone()),
                            "clt" => ast_opcode!(Clt, token.reference.clone()),
                            "jmp" => ast_opcode!(Jmp, token.reference.clone()),
                            "jie" => ast_opcode!(Jie, token.reference.clone()),
                            "jne" => ast_opcode!(Jne, token.reference.clone()),
                            "puti" => {
                                ast_opcode!(Puti, token.reference.clone())
                            }
                            "putc" => {
                                ast_opcode!(Putc, token.reference.clone())
                            }
                            "imz" => ast_opcode!(Imz, token.reference.clone()),
                            "equ" => ast_opcode!(Equ, token.reference.clone()),
                            "cal" => ast_opcode!(Cal, token.reference.clone()),
                            "cie" => ast_opcode!(Cie, token.reference.clone()),
                            "cne" => ast_opcode!(Cne, token.reference.clone()),
                            "ret" => ast_opcode!(Ret, token.reference.clone()),
                            "hlt" => ast_opcode!(Hlt, token.reference.clone()),
                            "set" => ast_opcode!(Set, token.reference.clone()),
                            _ => {
                                error_data.push(error!(
                                    InvalidOpcode,
                                    token.reference.clone(),
                                    x.clone()
                                ));
                                self.synchronise_lossy();
                                continue;
                            }
                        };
                        let mut args: Vec<ASTArgument> = vec![];
                        loop {
                            let peek = self.peek().expect(""); // todo: what is going on here?
                            match peek.inner {
                                TokenData::EOF => {
                                    self.advance();
                                    break;
                                }
                                TokenData::Newline => {
                                    self.advance();
                                    break;
                                }
                                _ => (),
                            }
                            match self.parse_argument(&opcode.reference) {
                                Ok(x) => args.push(x),
                                Err(x) => {
                                    error_data.push(x);
                                    self.synchronise_lossy();
                                    continue;
                                }
                            }
                        }
                        ast.instructions.push(ast_instruction!(
                            VMStep(opcode, args),
                            token.reference
                        ));
                    }
                    _ => {
                        error_data.push(error!(
                            InvalidStatement,
                            token.reference.clone()
                        ));
                        self.synchronise_lossy();
                        continue;
                    }
                }
            } else {
                error_data.push(error!(
                    UnexpectedSymbolTermination,
                    self.previous().reference.clone()
                ));
                return Err(error_data);
            }
        }
    }
}
