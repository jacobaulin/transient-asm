use crate::libs::Source;
use crate::namespaces::{IdentifierHash, IdentifierType, Namespaces};
use crate::parser::{ASTArgument, ASTInstruction, ASTOpcode};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
pub struct LIR<'a> {
    pub instructions: Vec<LIRInstruction>,
    pub labels: HashMap<LIRLabel, LIRData>, // <label, value>
    pub tags: HashMap<LIRTag, usize>,       // <label, index>
    immediate_handles: HashMap<u64, LIRLabel>, // <value, label>
    address_handles: HashMap<LIRLabel, LIRLabel>, // <underlying, immediate>

    namespaces: &'a mut Namespaces,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LIRLabel(pub IdentifierHash);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LIRTag(pub IdentifierHash);

#[derive(Clone, Debug)]
pub struct LIRArgument {
    pub label: LIRLabel,
    pub ptr_mode: LIRPtrMode,
}

#[derive(Clone, Debug)]
pub enum LIRData {
    Constant(u64),
    AddressOf(LIRLabel),
}

#[derive(Clone, Debug)]
pub enum LIRPtrMode {
    Unset,
    Pointer,
}

#[derive(Clone, Debug)]
pub enum LIRInstruction {
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
        self.labels.insert(label, data);
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
            self.push_label(LIRLabel(id), LIRData::Constant(value));
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
                self.instructions.push(LIRInstruction::Hlt);
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
