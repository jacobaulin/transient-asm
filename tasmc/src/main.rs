use std::path::PathBuf;
use std::process::exit;
use std::{env, fs};

use crate::codegen::Codegen;
use crate::errors::err_and_quit;
use crate::libs::{Source, import_library};
use crate::lir::LIR;
use crate::namespaces::Namespaces;

mod codegen;
mod lexer;
mod libs;
mod lir;
mod namespaces;
mod parser;
#[macro_use]
mod errors;

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

fn parse_arguments(args: &Vec<String>) -> (String, String, Flags) {
    let mut source_path = String::new();
    let mut bytecode_path = String::new();
    let mut flags = Flags::defaults();
    for arg in args {
        if let Some(i) = arg.find("--")
            && i == 0
        {
            match &arg[2..] {
                "dump-internal" => flags.dump_internal = true,
                _ => {
                    user_fatal!(
                        "fel: okänd kompilatorflagga vid namn `{}`",
                        &arg[2..]
                    );
                }
            }
        } else if arg.contains(".tasm") {
            if !source_path.is_empty() {
                user_fatal!("fel: mer än en källfil uppgavs (`{}`)", &arg);
            }
            source_path = arg.clone();
        } else if arg.contains(".tbc") {
            if !bytecode_path.is_empty() {
                user_fatal!("mer än en utdatafil uppgavs (`{}`).", &arg);
            }
            bytecode_path = arg.clone();
        }
    }

    (source_path, bytecode_path, flags)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (source_path, bytecode_path, flags) = parse_arguments(&args);

    if source_path.is_empty() {
        user_fatal!("fel: ingen källfil angavs");
    }

    let source_path: PathBuf =
        fs::canonicalize(&source_path).unwrap_or_else(|_| {
            user_fatal!("källfilen `{}` är oåtkomlig", &source_path);
        });
    let bytecode_path: PathBuf = if !bytecode_path.is_empty() {
        bytecode_path.into()
    } else {
        "./out.tbc".into()
    };

    let sources: Vec<Source> = import_library(&source_path, None, &flags)
        .unwrap_or_else(|err| err_and_quit(err));

    println!("tasmc: bearbetar datastruktur.");
    let mut namespaces: Namespaces =
        Namespaces::try_from(&sources).unwrap_or_else(|err| err_and_quit(err));
    if flags.dump_internal {
        dbg!(&namespaces);
    }

    let lir = LIR::generate(&sources, &mut namespaces)
        .unwrap_or_else(|err| err_and_quit(err));
    if flags.dump_internal {
        dbg!(&lir);
    }

    println!("tasmc: genererar bytekod.");
    let codegen = Codegen::from(lir);
    let bytecode = codegen.dump_bytecode();
    if flags.dump_internal {
        dbg!(&bytecode);
    }

    fs::write(&bytecode_path, &bytecode).unwrap_or_else(|_| {
        user_fatal!(
            "lyckades inte skriva bytekod till utdatafil `{}`",
            &bytecode_path.to_str().unwrap()
        );
    });

    println!("tasmc:\x1B[32m kompilering slutfördes.\x1B[0m");
}
