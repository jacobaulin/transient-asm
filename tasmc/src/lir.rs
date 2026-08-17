use crate::errors::{ErrorData, error, internal_fatal};
use crate::libs::Source;
use crate::namespaces::{IdentifierType, IdentifierUID, Namespaces};
use crate::parser::{
    ASTArgument, ASTArgumentData, ASTInstructionData, ASTOpcode, ASTOpcodeData,
};
use std::collections::HashMap;

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
pub struct LIRLabel(pub IdentifierUID);
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LIRTag(pub IdentifierUID);

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
    ($lir:expr, $arg:expr, Arg) => {
        $lir.resolve_argument($arg)
    };
    ($lir:expr, $arg:expr, Tag) => {
        $lir.resolve_tag($arg)
    };
}

macro_rules! unpack_vmstep {
    ($lir:expr, $opcode:expr, $args:expr, $($type:ident),*) => {{
        let mut args_iter = $args.iter();
        let expected_len = count!($($type),*);
        if expected_len != args_iter.len() {
            return Err(error!(
                InvalidArgumentNumber,
                $opcode.reference.clone(),
                expected_len,
                args_iter.len()
            ))
        }
        Ok((
            $(
                unpack_vmstep_inner!($lir, args_iter.next().unwrap().clone(), $type),
            )*
        ))
    }}
}

impl<'a> LIR<'a> {
    fn push_label(&mut self, label: LIRLabel, data: LIRData) {
        if self.labels.contains_key(&label) {
            internal_fatal!(
                "kollision mellan dataetiketter i l.i.r., etikett {:?} finns redan",
                label
            );
        }
        self.labels.insert(label, data);
    }
    fn resolve_address(
        &mut self,
        addr_label: LIRLabel,
    ) -> Result<LIRArgument, ErrorData> {
        if let Some(id) = self.address_handles.get(&addr_label) {
            Ok(LIRArgument {
                label: id.clone(),
                ptr_mode: LIRPtrMode::Unset,
            })
        } else {
            let uid = self.namespaces.create_uid();
            assert!(self.labels.contains_key(&addr_label));
            self.address_handles.insert(addr_label, LIRLabel(uid));
            assert!(!self.labels.contains_key(&LIRLabel(uid)));
            self.labels
                .insert(LIRLabel(uid), LIRData::AddressOf(addr_label));
            Ok(LIRArgument {
                label: LIRLabel(uid),
                ptr_mode: LIRPtrMode::Unset,
            })
        }
    }
    fn resolve_immediate(&mut self, value: u64) -> LIRArgument {
        if let Some(id) = self.immediate_handles.get(&value) {
            LIRArgument {
                label: *id,
                ptr_mode: LIRPtrMode::Unset,
            }
        } else {
            let uid = self.namespaces.create_uid();
            self.push_label(LIRLabel(uid), LIRData::Constant(value));
            self.immediate_handles.insert(value, LIRLabel(uid));
            LIRArgument {
                label: LIRLabel(uid),
                ptr_mode: LIRPtrMode::Unset,
            }
        }
    }
    fn resolve_argument(
        &mut self,
        ast_arg: ASTArgument,
    ) -> Result<LIRArgument, ErrorData> {
        match ast_arg.inner {
            ASTArgumentData::Tag(_) => Err(error!(
                InvalidASTArgumentData,
                ast_arg.reference.clone(),
                "variabel".into(),
                "tagg".into()
            )),
            ASTArgumentData::Variable(x) => {
                let ident = self
                    .namespaces
                    .resolve_identifier(&x, ast_arg.reference)?;
                Ok(LIRArgument {
                    label: LIRLabel(ident.uid),
                    ptr_mode: LIRPtrMode::Unset,
                })
            }
            ASTArgumentData::Pointer(x) => {
                let ident = self
                    .namespaces
                    .resolve_identifier(&x, ast_arg.reference)?;
                Ok(LIRArgument {
                    label: LIRLabel(ident.uid),
                    ptr_mode: LIRPtrMode::Pointer,
                })
            }
            ASTArgumentData::Address(x) => {
                let addr_id = self
                    .namespaces
                    .resolve_identifier(&x, ast_arg.reference.clone())?;
                self.resolve_address(LIRLabel(addr_id.uid))
            }
            ASTArgumentData::Immediate(x) => Ok(self.resolve_immediate(x)),
        }
    }
    fn resolve_tag(
        &mut self,
        ast_arg: ASTArgument,
    ) -> Result<LIRTag, ErrorData> {
        match ast_arg.inner {
            ASTArgumentData::Tag(x) => {
                let id = self
                    .namespaces
                    .resolve_identifier(&x, ast_arg.reference)?;
                Ok(LIRTag(id.uid))
            }
            _ => Err(error!(
                InvalidASTArgumentData,
                ast_arg.reference.clone(),
                "tagg".into(),
                "variabel".into()
            )),
        }
    }
    fn lower_instruction(
        &mut self,
        opcode: &ASTOpcode,
        args: &Vec<ASTArgument>,
    ) -> Result<(), ErrorData> {
        match opcode.inner {
            ASTOpcodeData::Set => (),
            ASTOpcodeData::Mov => {
                let (src1, dst) = unpack_vmstep!(self, opcode, args, Arg, Arg)?;
                self.instructions.push(LIRInstruction::Mov(src1?, dst?));
            }
            ASTOpcodeData::Add => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Add(src1?, src2?, dst?));
            }
            ASTOpcodeData::Sub => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Sub(src1?, src2?, dst?));
            }
            ASTOpcodeData::Mul => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Mul(src1?, src2?, dst?));
            }
            ASTOpcodeData::Divt => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Divt(src1?, src2?, dst?));
            }
            ASTOpcodeData::Divr => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Divr(src1?, src2?, dst?));
            }
            ASTOpcodeData::Rem => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Rem(src1?, src2?, dst?));
            }
            ASTOpcodeData::Cgt => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Cgt(src1?, src2?, dst?));
            }
            ASTOpcodeData::Clt => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Clt(src1?, src2?, dst?));
            }
            ASTOpcodeData::Jmp => {
                let (tag,) = unpack_vmstep!(self, opcode, args, Tag)?;
                self.instructions.push(LIRInstruction::Jmp(tag?));
            }
            ASTOpcodeData::Jie => {
                let (tag, src2) = unpack_vmstep!(self, opcode, args, Tag, Arg)?;
                self.instructions.push(LIRInstruction::Jie(tag?, src2?));
            }
            ASTOpcodeData::Jne => {
                let (tag, src2) = unpack_vmstep!(self, opcode, args, Tag, Arg)?;
                self.instructions.push(LIRInstruction::Jne(tag?, src2?));
            }
            ASTOpcodeData::Puti => {
                let (src1,) = unpack_vmstep!(self, opcode, args, Arg)?;
                self.instructions.push(LIRInstruction::Puti(src1?));
            }
            ASTOpcodeData::Putc => {
                let (src1,) = unpack_vmstep!(self, opcode, args, Arg)?;
                self.instructions.push(LIRInstruction::Putc(src1?));
            }
            ASTOpcodeData::Imz => {
                let (dst,) = unpack_vmstep!(self, opcode, args, Arg)?;
                self.instructions.push(LIRInstruction::Imz(dst?));
            }
            ASTOpcodeData::Equ => {
                let (src1, src2, dst) =
                    unpack_vmstep!(self, opcode, args, Arg, Arg, Arg)?;
                self.instructions
                    .push(LIRInstruction::Equ(src1?, src2?, dst?));
            }
            ASTOpcodeData::Cal => {
                let (tag,) = unpack_vmstep!(self, opcode, args, Tag)?;
                self.instructions.push(LIRInstruction::Cal(tag?));
            }
            ASTOpcodeData::Cie => {
                let (tag, src2) = unpack_vmstep!(self, opcode, args, Tag, Arg)?;
                self.instructions.push(LIRInstruction::Cie(tag?, src2?));
            }
            ASTOpcodeData::Cne => {
                let (tag, src2) = unpack_vmstep!(self, opcode, args, Tag, Arg)?;
                self.instructions.push(LIRInstruction::Cne(tag?, src2?));
            }
            ASTOpcodeData::Ret => {
                if args.len() != 0 {
                    return Err(error!(
                        InvalidArgumentNumber,
                        opcode.reference.clone(),
                        0,
                        args.len()
                    ));
                }
                self.instructions.push(LIRInstruction::Ret);
            }
            ASTOpcodeData::Hlt => {
                if args.len() != 0 {
                    return Err(error!(
                        InvalidArgumentNumber,
                        opcode.reference.clone(),
                        0,
                        args.len()
                    ));
                }
                self.instructions.push(LIRInstruction::Hlt);
            }
        }
        Ok(())
    }
    pub fn generate(
        sources: &Vec<Source>,
        namespaces: &'a mut Namespaces,
    ) -> Result<LIR<'a>, Vec<ErrorData>> {
        let mut lir = LIR {
            instructions: vec![],
            tags: HashMap::new(),
            labels: HashMap::new(),
            immediate_handles: HashMap::new(),
            address_handles: HashMap::new(),
            namespaces,
        };
        // add resolved labels.
        for map_idents in lir.namespaces.private_identifiers.values() {
            for ident in map_idents.values() {
                if let IdentifierType::Variable(value) = ident.idtype {
                    lir.labels.insert(
                        LIRLabel(ident.uid),
                        LIRData::Constant(value.unwrap_or(0)),
                    );
                }
            }
        }
        for ident in lir.namespaces.public_identifiers.values() {
            if let IdentifierType::Variable(value) = ident.idtype {
                lir.labels.insert(
                    LIRLabel(ident.uid),
                    LIRData::Constant(value.unwrap_or(0)),
                );
            }
        }
        // sources have to be lowered separately still.
        // this is because the identifiers aren't fully resolved yet.
        let mut error_data: Vec<ErrorData> = vec![];
        for source in sources {
            for instruction in &source.ast.instructions {
                match instruction.inner.clone() {
                    ASTInstructionData::Tag(tag) => {
                        let tag_uid = match lir.namespaces.resolve_identifier(
                            &tag,
                            instruction.reference.clone(),
                        ) {
                            Ok(ident) => ident.uid,
                            Err(err) => {
                                error_data.push(err);
                                continue;
                            }
                        };
                        // this computes where the tag will point to.
                        lir.tags
                            .insert(LIRTag(tag_uid), lir.instructions.len());
                    }
                    ASTInstructionData::VMStep(opcode, args) => {
                        match lir.lower_instruction(&opcode, &args) {
                            Ok(()) => (),
                            Err(err) => {
                                error_data.push(err);
                            }
                        };
                    }
                }
            }
        }
        if error_data.is_empty() {
            Ok(lir)
        } else {
            Err(error_data)
        }
    }
}
