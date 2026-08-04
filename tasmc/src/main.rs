use std::path::PathBuf;
use std::{env, fs};

use crate::codegen::Codegen;
use crate::libs::{Source, import_library};
use crate::lir::LIR;
use crate::namespaces::Namespaces;

mod codegen;
mod lexer;
mod libs;
mod lir;
mod namespaces;
mod parser;

struct Flags {
    dump_internal: bool,
}

impl Flags {
    fn defaults() -> Self {
        Self {
            dump_internal: false,
        }
    }
}

fn parse_arguments(args: &Vec<String>) -> (String, Flags) {
    let mut source_path = String::new();
    let mut flags = Flags::defaults();
    for arg in args {
        if let Some(i) = arg.find("--") && i == 0 {
            match &arg[2..] {
                "dump-internal" => flags.dump_internal = true,
                _ => panic!("okänd kompilatorflagga."),
            }
        } else if source_path.is_empty() {
            source_path = arg.clone();
        } else {
            panic!("mer än en källfil uppgavs.");
        }
    }

    (source_path, flags)
}

fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().skip(1).collect();

    let (source_path, flags) = parse_arguments(&args);

    let source_path: PathBuf =
        fs::canonicalize(source_path).expect("lyckades inte normalisera källfil");

    let bytecode_path: String = args.get(2).unwrap_or(&"./out.tbc".to_owned()).clone();

    let sources: Vec<Source> = import_library(&source_path, None, &flags);

    println!("tasmc: bearbetar datastruktur.");
    let mut namespaces: Namespaces =
        Namespaces::try_from(&sources).expect("lyckades inte lösa namnrymd");
    if flags.dump_internal {
        dbg!(&namespaces);
    }

    let lir =
        LIR::generate(&sources, &mut namespaces).expect("lyckades inte generera lågnivåstruktur");
    if flags.dump_internal {
        dbg!(&lir);
    }

    println!("tasmc: genererar bytekod.");
    let codegen = Codegen::from(lir);
    let bytecode = codegen.dump_bytecode();
    if flags.dump_internal {
        dbg!(&bytecode);
    }

    fs::write(bytecode_path, &bytecode).expect("lyckades inte skriva till utdatafil");

    println!("tasmc:\x1B[32m kompilering slutfördes.\x1B[0m");
    Ok(())
}
