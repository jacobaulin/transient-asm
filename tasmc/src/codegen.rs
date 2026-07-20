use crate::lir::{LIR, LIRArgument, LIRData, LIRInstruction, LIRLabel, LIRPtrMode, LIRTag};
use std::collections::HashMap;

const BYTES_PER_INSTRUCTION: usize = 8;
const BYTES_PER_DATA: usize = 8;

#[derive(Clone, Debug)]
pub struct Codegen {
    instructions: Vec<LIRInstruction>,

    // labels must have a stable order.
    labels: Vec<(LIRLabel, LIRData)>,
    tags: HashMap<LIRTag, usize>,

    memory_metadata: MemoryMetadata,
}

#[derive(Clone, Debug)]
struct MemoryMetadata {
    mem_size_text: usize,
    mem_offset_text: usize,
    mem_size_data: usize,
    mem_offset_data: usize,
}

impl<'a> From<LIR<'a>> for Codegen {
    fn from(lir: LIR) -> Codegen {
        // compute memory addresses.
        // these sizes are in bytes.
        let mem_size_text = lir.instructions.len() * BYTES_PER_INSTRUCTION;
        let mem_offset_text = 0x00;
        let mem_size_data = lir.labels.len() * BYTES_PER_DATA;
        let mem_offset_data = mem_size_text;

        Codegen {
            instructions: lir.instructions,
            labels: lir
                .labels
                .iter()
                .map(|(x, y)| (x.clone(), y.clone()))
                .collect(),
            tags: lir.tags,
            memory_metadata: MemoryMetadata {
                mem_size_text,
                mem_offset_text,
                mem_size_data,
                mem_offset_data,
            },
        }
    }
}

impl Codegen {
    /* The current bytecode format assumes the following:
     * - Every instruction is 8 bytes.
     * - The (initial) text section starts at offset 0x00.
     * - Text may be present anywhere in the image.
     * - Data (static or variable) are fixed at 8 bytes.
     * - Data (static or variable) may be present anywhere in the image, other than at offset 0x00
     * - The l.i.r. is not malformed
     *   -> i.e., it must not allow for the instruction pointer to point to a non-text section.
     */
    pub fn dump_bytecode(&self) -> Vec<u8> {
        let mut bytecode: Vec<u8> = vec![];

        // generate the text section.
        for instruction in &self.instructions {
            bytecode.extend(self.generate_instruction(&instruction));
        }

        // generate the data section.
        assert_eq!(bytecode.len(), self.memory_metadata.mem_offset_data);
        for index in 0..self.labels.len() {
            let (_, data) = &self.labels[index];
            match data {
                LIRData::Constant(value) => {
                    let value_bytes = value.to_be_bytes();
                    assert_eq!(value_bytes.len(), 8);
                    bytecode.extend(value_bytes);
                }
                LIRData::AddressOf(label) => {
                    let value = self.get_absolute_address_label(*label);
                    let value_bytes = value.to_be_bytes();
                    assert_eq!(value_bytes.len(), 8);
                    bytecode.extend(value_bytes);
                }
            }
        }

        assert_eq!(
            bytecode.len(),
            self.memory_metadata.mem_size_text + self.memory_metadata.mem_size_data
        );
        bytecode
    }

    /* todo: this is slow. iterating over all labels
     * is unnecessary and ideally another map of labels
     * to their indexes in the memory vector is a lot
     * more efficient.
     */
    fn get_absolute_address_label(&self, label: LIRLabel) -> u16 {
        for index in 0..self.labels.len() {
            let (id, _) = self.labels[index];
            if id == label {
                return (self.memory_metadata.mem_offset_data + index * BYTES_PER_DATA)
                    .try_into()
                    .expect("minnesadresser överstiger maxvärdet");
            }
        }
        panic!("absolute address label lookup yielded no results");
    }
    fn get_absolute_address_tag(&self, tag: &LIRTag) -> u16 {
        let index = self.tags.get(&tag).expect("could not find memory tag");
        (self.memory_metadata.mem_offset_text + index * BYTES_PER_INSTRUCTION)
            .try_into()
            .expect("minnesadresser överstiger maxvärdet")
    }
    fn generate_instruction(&self, instruction: &LIRInstruction) -> Vec<u8> {
        let opcode = match instruction {
            LIRInstruction::Mov(..) => 0x01,
            LIRInstruction::Add(..) => 0x02,
            LIRInstruction::Sub(..) => 0x03,
            LIRInstruction::Mul(..) => 0x04,
            LIRInstruction::Divt(..) => 0x05,
            LIRInstruction::Divr(..) => 0x06,
            LIRInstruction::Rem(..) => 0x07,
            LIRInstruction::Cgt(..) => 0x08,
            LIRInstruction::Clt(..) => 0x09,
            LIRInstruction::Jmp(..) => 0x0A,
            LIRInstruction::Jie(..) => 0x0B,
            LIRInstruction::Jne(..) => 0x0C,
            LIRInstruction::Puti(..) => 0x0D,
            LIRInstruction::Putc(..) => 0x0E,
            LIRInstruction::Imz(..) => 0x0F,
            LIRInstruction::Equ(..) => 0x10,
            LIRInstruction::Cal(..) => 0x11,
            LIRInstruction::Cie(..) => 0x12,
            LIRInstruction::Cne(..) => 0x13,
            LIRInstruction::Ret => 0x14,
            LIRInstruction::Hlt => 0xFF,
        };
        let (ptr_data, src1, src2, dst): (u8, u16, u16, u16) = match instruction {
            LIRInstruction::Mov(src1, dst) => (
                Self::make_pointer_data(Some(src1), None, Some(dst)),
                self.get_absolute_address_label(src1.label),
                0x00,
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Add(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Sub(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Mul(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Divt(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Divr(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Rem(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Cgt(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Clt(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Jmp(tag) => (0x00, self.get_absolute_address_tag(tag), 0x00, 0x00),
            LIRInstruction::Jie(tag, src2) => (
                Self::make_pointer_data(None, Some(src2), None),
                self.get_absolute_address_tag(tag),
                self.get_absolute_address_label(src2.label),
                0x00,
            ),
            LIRInstruction::Jne(tag, src2) => (
                Self::make_pointer_data(None, Some(src2), None),
                self.get_absolute_address_tag(tag),
                self.get_absolute_address_label(src2.label),
                0x00,
            ),
            LIRInstruction::Puti(src1) => (
                Self::make_pointer_data(Some(src1), None, None),
                self.get_absolute_address_label(src1.label),
                0x00,
                0x00,
            ),
            LIRInstruction::Putc(src1) => (
                Self::make_pointer_data(Some(src1), None, None),
                self.get_absolute_address_label(src1.label),
                0x00,
                0x00,
            ),
            LIRInstruction::Imz(dst) => (
                Self::make_pointer_data(None, None, Some(dst)),
                0x00,
                0x00,
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Equ(src1, src2, dst) => (
                Self::make_pointer_data(Some(src1), Some(src2), Some(dst)),
                self.get_absolute_address_label(src1.label),
                self.get_absolute_address_label(src2.label),
                self.get_absolute_address_label(dst.label),
            ),
            LIRInstruction::Cal(tag) => (0x00, self.get_absolute_address_tag(tag), 0x00, 0x00),
            LIRInstruction::Cie(tag, src2) => (
                Self::make_pointer_data(None, Some(src2), None),
                self.get_absolute_address_tag(tag),
                self.get_absolute_address_label(src2.label),
                0x00,
            ),
            LIRInstruction::Cne(tag, src2) => (
                Self::make_pointer_data(None, Some(src2), None),
                self.get_absolute_address_tag(tag),
                self.get_absolute_address_label(src2.label),
                0x00,
            ),
            LIRInstruction::Ret => (0x00, 0x00, 0x00, 0x00),
            LIRInstruction::Hlt => (0x00, 0x00, 0x00, 0x00),
        };

        let src1_bytes = src1.to_be_bytes();
        let src2_bytes = src2.to_be_bytes();
        let dst_bytes = dst.to_be_bytes();

        assert_eq!(src1_bytes.len(), 2);
        assert_eq!(src2_bytes.len(), 2);
        assert_eq!(dst_bytes.len(), 2);

        vec![
            opcode,
            ptr_data,
            src1_bytes[0],
            src1_bytes[1],
            src2_bytes[0],
            src2_bytes[1],
            dst_bytes[0],
            dst_bytes[1],
        ]
    }
    fn make_pointer_data(
        src1: Option<&LIRArgument>,
        src2: Option<&LIRArgument>,
        dst: Option<&LIRArgument>,
    ) -> u8 {
        let src1_flag = if let Some(src1) = src1
            && let LIRPtrMode::Pointer = src1.ptr_mode
        {
            1
        } else {
            0
        };
        let src2_flag = if let Some(src2) = src2
            && let LIRPtrMode::Pointer = src2.ptr_mode
        {
            1
        } else {
            0
        };
        let dst_flag = if let Some(dst) = dst
            && let LIRPtrMode::Pointer = dst.ptr_mode
        {
            1
        } else {
            0
        };

        src1_flag + src2_flag >> 1 + dst_flag >> 2
    }
}
