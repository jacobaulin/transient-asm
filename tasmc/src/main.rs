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

fn main() -> Result<(), ()> {
    let args: Vec<String> = env::args().collect();
    let source_path: String = args.get(1).expect("ingen indatafil").clone();
    let source_path: PathBuf =
        fs::canonicalize(source_path).expect("lyckades inte normalisera källfil");

    let bytecode_path: String = args.get(2).unwrap_or(&"./out.tbc".to_owned()).clone();
    let bytecode_path: PathBuf =
        fs::canonicalize(bytecode_path).expect("lyckades inte normalisera utdatafil");

    let sources: Vec<Source> = import_library(&source_path, None);

    println!("tasmc: bearbetar datastruktur.");
    let mut namespaces: Namespaces =
        Namespaces::try_from(&sources).expect("lyckades inte lösa namnrymd");

    let lir =
        LIR::generate(&sources, &mut namespaces).expect("lyckades inte generera lågnivåstruktur");

    println!("tasmc: genererar bytekod.");
    let codegen = Codegen::from(lir);
    let bytecode = codegen.dump_bytecode();

    fs::write(bytecode_path, &bytecode).expect("lyckades inte skriva till utdatafil");

    println!("tasmc:\x1B[32m kompilering slutfördes.\x1B[0m");
    Ok(())
}
