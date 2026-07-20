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
    println!("< tasmc >");
    let path: String = env::args().collect::<Vec<String>>()[1].clone();
    let path: PathBuf = fs::canonicalize(path).expect("lyckades inte normalisera källfil");

    let sources: Vec<Source> = import_library(&path, None);

    println!("tasmc: löser namnrymd");
    let mut namespaces: Namespaces =
        Namespaces::try_from(&sources).expect("lyckades inte lösa namnrymd");

    println!("tasmc: genererar lågnivåstruktur");
    let lir =
        LIR::generate(&sources, &mut namespaces).expect("lyckades inte generera lågnivåstruktur");

    println!("tasmc: skapar bytekod");
    let codegen = Codegen::from(lir);
    let bytecode = codegen.dump_bytecode();

    // dbg!(&bytecode);

    Ok(())
}
