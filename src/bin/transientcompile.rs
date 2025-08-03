//! Compiler that transforms Transient-C into TransientIR. (TIR)
//! Currently under development.

/*
0x01: MOV byte from source1 into destination
0x02: ADD source1 and source2 and store result in destination
0x03: SUB source2 from source1 and store result in destination
0x04: MUL source1 and source2 and store result in destination
0x05: DIV source1 by source2 and store result in destination (truncated)
0x06: DIV source1 by source2 and store result in destination (rounded)
0x07: REM divides source1 by source2 and stores the remainder in destination
0x08: CGT compare if source1 is greater than source2, and if so, store 1 in destination
0x09: CLT compare if source1 is less than source2, and if so, store 1 in destination
0x0A: JMP stops current execution and jumps to code in source1
0x0B: JIE stops current execution and jumps to code in source1 ONLY IF source2 is non-zero
0x0C: JNE stops current execution and jumps to code in source1 ONLY IF source2 is zero
0x0D: PUT prints data at source1 to the screen (int)
0x0E: PUT prints data at source1 to the screen (char)
0x0F: IMZ gets the image size that was loaded to ROM and stores it in destination
0x10: EQU compare if source1 and source2 are equal, and if so, store 1 in destination
0x11: CAL stops current execution and jumps to code in source1, saving the program counter
0x12: CIE stops current execution and jumps to code in source1, saving the program counter, ONLY IF source2 is non-zero
0x13: CNE stops current execution and jumps to code in source1, saving the program counter, ONLY IF source2 is zero
0x14: RET restores the saved program counter
0xFF: HLT halts execution and stops processor
*/

use std::collections::HashMap;
use std::env::args;
use std::fs::File;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{Read, Write};
use std::process::exit;

#[derive(Debug, Hash, Eq, PartialEq)]
enum Operation {
    Mov(usize, usize, usize),
    Add(usize, usize, usize, usize),
    Sub(usize, usize, usize, usize),
    Mul(usize, usize, usize, usize),
    DivT(usize, usize, usize, usize),
    DivR(usize, usize, usize, usize),
    Rem(usize, usize, usize, usize),
    Cgt(usize, usize, usize, usize),
    Clt(usize, usize, usize, usize),
    Jmp(usize),
    Jie(usize, usize, usize),
    Jne(usize, usize, usize),
    PutI(usize, usize),
    PutC(usize, usize),
    Imz(usize, usize),
    Equ(usize, usize, usize, usize),
    Cal(usize),
    Cie(usize, usize, usize),
    Cne(usize, usize, usize),
    Ret(),
    Hlt(),
}

fn resolve_operation_opcode(operation: &Operation) -> u8 {
    match operation {
        Operation::Mov(..) => 0x01,
        Operation::Add(..) => 0x02,
        Operation::Sub(..) => 0x03,
        Operation::Mul(..) => 0x04,
        Operation::DivT(..) => 0x05,
        Operation::DivR(..) => 0x06,
        Operation::Rem(..) => 0x07,
        Operation::Cgt(..) => 0x08,
        Operation::Clt(..) => 0x09,
        Operation::Jmp(..) => 0x0A,
        Operation::Jie(..) => 0x0B,
        Operation::Jne(..) => 0x0C,
        Operation::PutI(..) => 0x0D,
        Operation::PutC(..) => 0x0E,
        Operation::Imz(..) => 0x0F,
        Operation::Equ(..) => 0x10,
        Operation::Cal(..) => 0x11,
        Operation::Cie(..) => 0x12,
        Operation::Cne(..) => 0x13,
        Operation::Ret(..) => 0x14,
        Operation::Hlt(..) => 0xFF,
    }
}

fn preprocess_source_code(
    source_code: Vec<String>,
) -> (Vec<Operation>, HashMap<String, (usize, u64)>) {
    let mut source_code = source_code;
    
    // Pass 1
    // Include all external libraries
    let mut i = 0;
    while i < source_code.len() {
        if !source_code[i].starts_with("*lib ") {
            i += 1;
            continue;
        }
        let libname: String = source_code[i][5..].to_owned();
        let libcontent = std::fs::read_to_string(format!("{libname}")).unwrap_or_else(|_| { halt_compilation("[E011] Failed to load library, check permissions and path", &source_code[i])});
        source_code[i] = "".to_owned();
        for line in libcontent.lines() {
            source_code.push(line.to_owned());
        }
        i += 1;
    }
    dbg!(&source_code);

    // Pass 1
    // Remove all comments
    source_code.retain(|x| !x.starts_with("//"));

    // Pass 2
    // Calculate all intermediates
    let mut intermediates: HashMap<u64, usize> = HashMap::new();
    for line in source_code.iter() {
        let line_tokens: Vec<String> = line.split(" ").map(|x| x.to_owned()).collect();
        if line.starts_with("set") {
            continue;
        }
        for token in &line_tokens {
            if !token.parse::<usize>().is_ok() {
                continue;
            }
            let value = token.parse::<usize>().unwrap_or_else(|_| {
                halt_compilation(
                    "[E012] Failed to parse intermediate value: Only integers are allowed",
                    &line,
                )
            });
            let mut hasher = DefaultHasher::new();
            token.hash(&mut hasher);
            let hash = hasher.finish();
            if intermediates.get(&hash).is_some() {
                continue;
            }
            intermediates.insert(hash, value);
        }
    }
    // Pass 3
    // Insert new intermediate variable declarations
    for (hash, value) in intermediates.iter() {
        for line in source_code.iter_mut() {
            if line.len() >= 3 && &line[..3] == "set" {
                continue;
            }
            *line = line.replace(&format!(" {value} "), &format!(" __internal_{hash} "));
            if line.ends_with(&format!(" {value}")) {
                *line = line.strip_suffix(&format!(" {value}")).unwrap().to_owned() + &format!(" __internal_{hash}");
            }
        }
        source_code.insert(0, format!("set __internal_{hash} {value}"));
    }

    // Pass 4
    // Count IR LoC
    let mut lines_of_ir = 0usize;
    for line in &source_code {
        // Check if it's actual IR
        if !line.is_empty()
        && !line.starts_with("#")
        && !line.starts_with("//")
        && !line.starts_with("set")
        {
            lines_of_ir += 1;
        }
    }
    let ir_size_bytes = lines_of_ir * 8;

    // Pass 5
    // Build hashmap of variables to memory
    let mut memory_map: HashMap<String, (usize, u64)> = HashMap::new(); // Address, value
    let mut memory_offset = 0usize;
    for line in &source_code {
        // Skip if not declaration
        if !line.starts_with("set") {
            continue;
        }
        // set{bits} $variable value
        let line_tokens: Vec<String> = line.split(" ").map(|x| x.to_owned()).collect();
        if line_tokens.len() != 3 && line_tokens.len() != 2 {
            halt_compilation(
                "[E001] Invalid set syntax: Number of arguments are invalid.",
                line,
            );
        }
        // Check if variable exists
        if memory_map.get(&line_tokens[1]).is_some() {
            halt_compilation(
                "[E010] Variable memory collision: Did you initialize the same variable twice?",
                &line,
            );
        }
        let mut value = 0;
        if line_tokens.len() == 3 {
            value = match u64::from_str_radix(&line_tokens[2], 10) {
                Ok(x) => x,
                Err(..) => halt_compilation(
                    "[E004] Failed to parse value: Only integer values are allowed",
                    line,
                ),
            };
        }

        memory_map.insert(
            line_tokens[1].to_string(),
            (ir_size_bytes + memory_offset, value),
        );
        memory_offset += 8
    }

    // Pass 6
    // Erase sets, and empty lines
    source_code.retain(|line| !line.is_empty() && !line.starts_with("set"));

    // Pass 7
    // Calculate pointer-based memory address references
    for line in source_code.iter_mut() {
        let line_tokens: Vec<String> = line.split(" ").map(|x| x.to_owned()).collect();
        let mut address_references: HashMap<String, usize> = HashMap::new();
        for token in line_tokens {
            if !token.starts_with("&") {
                continue;
            }
            // let (address, value, size) = memory_map[token[1..]];
            let address = memory_map.get(&token[1..].to_owned()).unwrap_or_else(|| { halt_compilation("[E002] Memory resolution failed: Try checking your spelling", &line) }).0;

            address_references.insert(token[1..].to_owned(), address);
            // *token = address.to_string().as_str();
            // source_code.insert(0, format!("set __internal_{hash} {value}"));
        }
        for (ptr_ref, address) in address_references {
            *line = line.replace(&format!("&{ptr_ref}"), &format!("{address}"));
        }
    }

    // Pass 8
    // Recalculate new pointer-based intermediates
    let mut intermediates: HashMap<u64, usize> = HashMap::new();
    for line in source_code.iter() {
        let line_tokens: Vec<String> = line.split(" ").map(|x| x.to_owned()).collect();
        if line.starts_with("set") {
            continue;
        }
        for token in &line_tokens {
            if !token.parse::<usize>().is_ok() {
                continue;
            }
            let value = token.parse::<usize>().unwrap_or_else(|_| {
                halt_compilation(
                    "[E012] Failed to parse intermediate value: Only integers are allowed",
                    &line,
                )
            });
            let mut hasher = DefaultHasher::new();
            token.hash(&mut hasher);
            let hash = hasher.finish();
            if intermediates.get(&hash).is_some() {
                continue;
            }
            intermediates.insert(hash, value);
        }
    }
    // Pass 9
    // Insert new poiner-based intermediate variable declarations
    for (hash, value) in intermediates.iter() {
        for line in source_code.iter_mut() {
            if line.len() >= 3 && &line[..3] == "set" {
                continue;
            }
            *line = line.replace(&format!(" {value} "), &format!(" __internal_{hash} "));
            if line.ends_with(&format!(" {value}")) {
                *line = line.strip_suffix(&format!(" {value}")).unwrap().to_owned() + &format!(" __internal_{hash}");
            }
            // *line = line.replace(&format!(" {value}"), &format!(" __internal_{hash}"));
        }
        source_code.insert(0, format!("set __internal_{hash} {value}"));
    }

    // Pass 10
    // Appends memory map with variable reference declarations
    for line in &source_code {
        // Skip if not declaration
        if !line.starts_with("set") {
            continue;
        }
        // set{bits} $variable value
        let line_tokens: Vec<String> = line.split(" ").map(|x| x.to_owned()).collect();
        if line_tokens.len() != 3 && line_tokens.len() != 2 {
            halt_compilation(
                "[E001] Invalid set syntax: Number of arguments are invalid.",
                line,
            );
        }
        // Check if variable exists
        if memory_map.get(&line_tokens[1]).is_some() {
                dbg!(&source_code);
                dbg!(&memory_map);
            halt_compilation(
                "[E010] Variable memory collision: Did you initialize the same variable twice?",
                &line,
            );
        }
        let mut value = 0;
        if line_tokens.len() == 3 {
            value = match u64::from_str_radix(&line_tokens[2], 10) {
                Ok(x) => x,
                Err(..) => halt_compilation(
                    "[E004] Failed to parse value: Only integer values are allowed",
                    line,
                ),
            };
        }

        memory_map.insert(
            line_tokens[1].to_string(),
            (ir_size_bytes + memory_offset, value),
        );
        memory_offset += 8
    }

    // Pass 11
    // Final cleanup of variable reference sets
    source_code.retain(|line| !line.is_empty() && !line.starts_with("set"));

    // Pass 12
    // Repeatedly scan and generate tag addresses
    let mut jump_addresses: HashMap<String, usize> = HashMap::new();
    loop {
        let mut clean = true;
        let mut index_to_remove: usize = 0;
        for (index, line) in source_code.iter().enumerate() {
            if line.starts_with("#") {
                clean = false;
                jump_addresses.insert(line[1..].to_owned(), index * 8);
                index_to_remove = index;
                break;
            }
        }
        if clean {
            break;
        } else {
            source_code.remove(index_to_remove);
        }
    }

    // Pass 13
    // Build abstract syntax tree
    let mut abstract_syntax_tree: Vec<Operation> = Vec::new();
    for line in &source_code {
        let line_tokens: Vec<String> = line.split(" ").map(|x| x.to_owned()).collect();
        let opcode: String = line_tokens[0].clone();
        let args: Vec<usize> = line_tokens[1..].iter().map(|x|{
            if x.starts_with("#") {
                jump_addresses.get(&x[1..]).unwrap_or_else(|| { halt_compilation("[E005] Jump address resolution failed: Try checking your spelling", &line) }).clone()
            } else if x.starts_with(">") {
                memory_map.get(&x[1..]).unwrap_or_else(|| { halt_compilation("[E006] Memory resolution failed: Try checking your spelling", &line) }).0
            } else {
                memory_map.get(x).unwrap_or_else(|| { halt_compilation("[E003] Memory resolution failed: Try checking your spelling", &line) }).0
            } // else {
            // halt_compilation("[E007] Invalid argument to function: Only variables and tags are allowed as arguments", &line);
            // }
        }).collect();
        let mut ptrs: Vec<usize> = Vec::new();
        for i in 1..line_tokens.len() {
            if line_tokens[i].starts_with(">") {
                ptrs.push(1);
            } else {
                ptrs.push(0);
            }
        }
        abstract_syntax_tree.push(match &opcode[..] {
            "mov" => {
                if args.len() != 2 {
                    halt_compilation("[E008] This function takes 2 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 4;
                Operation::Mov(ptr_mode, args[0], args[1])
            }
            "add" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::Add(ptr_mode, args[0], args[1], args[2])
            }
            "sub" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::Sub(ptr_mode, args[0], args[1], args[2])
            }
            "mul" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::Mul(ptr_mode, args[0], args[1], args[2])
            }
            "divt" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::DivT(ptr_mode, args[0], args[1], args[2])
            }
            "divr" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::DivR(ptr_mode, args[0], args[1], args[2])
            }
            "rem" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::Rem(ptr_mode, args[0], args[1], args[2])
            }
            "cgt" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::Cgt(ptr_mode, args[0], args[1], args[2])
            }
            "clt" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::Clt(ptr_mode, args[0], args[1], args[2])
            }
            "jmp" => {
                if args.len() != 1 {
                    halt_compilation("[E008] This function takes 1 argument", &line);
                }
                Operation::Jmp(args[0])
            }
            "jie" => {
                if args.len() != 2 {
                    halt_compilation("[E008] This function takes 2 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2;
                Operation::Jie(ptr_mode, args[0], args[1])
            }
            "jne" => {
                if args.len() != 2 {
                    halt_compilation("[E008] This function takes 2 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2;
                Operation::Jne(ptr_mode, args[0], args[1])
            }
            "puti" => {
                if args.len() != 1 {
                    halt_compilation("[E008] This function takes 1 argument", &line);
                }
                let ptr_mode = ptrs[0] * 1;
                Operation::PutI(ptr_mode, args[0])
            }
            "putc" => {
                if args.len() != 1 {
                    halt_compilation("[E008] This function takes 1 argument", &line);
                }
                let ptr_mode = ptrs[0] * 1;
                Operation::PutC(ptr_mode, args[0])
            }
            "imz" => {
                if args.len() != 1 {
                    halt_compilation("[E008] This function takes 1 argument", &line);
                }
                let ptr_mode = ptrs[0] * 4;
                Operation::Imz(ptr_mode, args[0])
            }
            "equ" => {
                if args.len() != 3 {
                    halt_compilation("[E008] This function takes 3 argument", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2 | ptrs[2] * 4;
                Operation::Equ(ptr_mode, args[0], args[1], args[2])
            }
            "cal" => {
                if args.len() != 1 {
                    halt_compilation("[E008] This function takes 1 argument", &line);
                }
                Operation::Cal(args[0])
            }
            "cie" => {
                if args.len() != 2 {
                    halt_compilation("[E008] This function takes 2 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2;
                Operation::Cie(ptr_mode, args[0], args[1])
            }
            "cne" => {
                if args.len() != 2 {
                    halt_compilation("[E008] This function takes 2 arguments", &line);
                }
                let ptr_mode = ptrs[0] * 1 | ptrs[1] * 2;
                Operation::Cne(ptr_mode, args[0], args[1])
            }
            "ret" => Operation::Ret(),
            "hlt" => Operation::Hlt(),
            _ => {
                halt_compilation("[E009] Invalid opcode. Check your spelling", &line);
            }
        })
    }

    (abstract_syntax_tree, memory_map)
}

fn gen_binary_instruction(
    opcode: u8,
    ptr_mode: usize,
    src1: usize,
    src2: usize,
    dest: usize,
) -> [u8; 8] {
    [
        opcode,
        ptr_mode as u8,
        (src1 as u16).to_be_bytes()[0],
        (src1 as u16).to_be_bytes()[1],
        (src2 as u16).to_be_bytes()[0],
        (src2 as u16).to_be_bytes()[1],
        (dest as u16).to_be_bytes()[0],
        (dest as u16).to_be_bytes()[1],
    ]
}

fn codegen(
    abstract_syntax_tree: &Vec<Operation>,
    memory_map: &HashMap<String, (usize, u64)>,
) -> Vec<u8> {
    let mut image: Vec<u8> = vec![];

    // Write instructions to image
    for (_index, instruction) in abstract_syntax_tree.iter().enumerate() {
        let opcode = resolve_operation_opcode(&instruction);
        match *instruction {
            Operation::Mov(ptr_mode, src1, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, 0x00, dest));
            }
            Operation::Add(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::Sub(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::Mul(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::DivT(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::DivR(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::Rem(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::Cgt(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::Clt(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::Jmp(src1) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, 0x00, src1, 0x00, 0x00));
            }
            Operation::Jie(ptr_mode, src1, src2) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, 0x00));
            }
            Operation::Jne(ptr_mode, src1, src2) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, 0x00));
            }
            Operation::PutI(ptr_mode, src1) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, 0x00, 0x00));
            }
            Operation::PutC(ptr_mode, src1) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, 0x00, 0x00));
            }
            Operation::Imz(ptr_mode, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, 0x00, 0x00, dest));
            }
            Operation::Equ(ptr_mode, src1, src2, dest) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, dest));
            }
            Operation::Cal(src1) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, 0x00, src1, 0x00, 0x00));
            }
            Operation::Cie(ptr_mode, src1, src2) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, 0x00));
            }
            Operation::Cne(ptr_mode, src1, src2) => {
                image.extend_from_slice(&gen_binary_instruction(opcode, ptr_mode, src1, src2, 0x00));
            }
            Operation::Ret() => {
                image.extend_from_slice(&gen_binary_instruction(opcode, 0x00, 0x00, 0x00, 0x00));
            }
            Operation::Hlt() => {
                image.extend_from_slice(&gen_binary_instruction(opcode, 0x00, 0x00, 0x00, 0x00));
            }
        }
    }

    // Calculate amount of space that variables take
    let mut var_size = 0;
    for (_address, _value) in memory_map.values() {
        var_size += 8;
    }

    // Allocate size for new vars
    image.resize(image.len() + var_size, 0);

    // Write variables to image
    for (address, value) in memory_map.values() {
        image[*address..][..8].copy_from_slice(
            value.to_be_bytes()[value.to_be_bytes().len() - 8..]
                .try_into()
                .expect("[COMPILER PANIC]: Failed to write variable to image"),
        )
    }

    image
}

fn halt_compilation(message: &str, line: &str) -> ! {
    eprintln!("--------------------------------------------");
    eprintln!("Error: {}", message);
    eprintln!("-> Compilation failed on line `{}`", line);
    eprintln!("--------------------------------------------");
    exit(-1);
}

fn format_ast(ast: &Vec<Operation>) -> String {
    let mut out = String::new();
    for operation in ast {
        out += &format!("{:?}\n", operation);
    }
    out
}

fn format_mm(mm: &HashMap<String, (usize, u64)>) -> String {
    let mut out = String::new();
    for (name, (address, value)) in mm {
        out += &format!("[{}]: {} = {}\n", address, name, value);
    }
    out
}

fn main() {
    // Verify input parameters
    let args: Vec<String> = args().collect();
    if args.len() < 3 {
        println!("Stop: Incorrect amount of arguments!");
        return;
    }

    let mut verbose = false;
    if args.len() > 3 {
        verbose = args[3] == "--ast";
    }

    // Open file for reading
    let mut input_file = match File::open(&args[1]) {
        Ok(x) => x,
        Err(_) => {
            panic!("Stop: Failed to open file");
        }
    };
    let output_file_name = &args[2];

    // Read bytes into buffer
    let mut source_code: String = String::new();
    if let Err(_) = input_file.read_to_string(&mut source_code) {
        panic!("Stop: Failed to read file contents");
    }
    let source_code: Vec<String> = source_code.split("\n").map(|x| x.to_owned()).collect();
    print!("Compiling... [          ]\r");
    std::io::stdout().flush().unwrap();

    // Preprocess, resolve memory addresses, and generate abstract syntax tree
    let (abstract_syntax_tree, memory_map) = preprocess_source_code(source_code);
    print!("Compiling... [======    ]\r");
    std::io::stdout().flush().unwrap();

    // Codegen
    let executable = codegen(&abstract_syntax_tree, &memory_map);
    print!("Compiling... [========= ]\r");
    std::io::stdout().flush().unwrap();

    // Write output file
    let mut output_file = File::create(output_file_name).expect("Failed to create output file");
    output_file
        .write(&executable)
        .expect("Failed to write to output file");
    print!("Compiling... [==========]\n");

    if verbose {
        println!(
            "AST:\n{}\nMM:\n{}",
            format_ast(&abstract_syntax_tree),
            format_mm(&memory_map)
        )
    }

    // Done!
    println!("Success: Compilation finished ✔");
}
