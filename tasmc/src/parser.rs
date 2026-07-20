use crate::lexer::Token;

#[derive(Debug, Clone)]
pub struct AST {
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
pub enum ASTInstruction {
    VMStep(ASTOpcode, Vec<ASTArgument>),
    Tag(String), // identifier
}

#[derive(Debug, Clone)]
pub enum ASTArgument {
    Variable(String), // identifier
    Pointer(String),  // identifier
    Address(String),  // identifier
    Tag(String),      // identifier
    Immediate(u64),   // value
}

#[derive(Debug, Clone)]
pub enum ASTOpcode {
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
