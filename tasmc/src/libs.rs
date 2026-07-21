use std::fs;
use std::path::{Path, PathBuf};

use crate::lexer::Lexer;
use crate::parser::{AST, Parser};

#[derive(Clone, Debug)]
pub struct Source {
    pub path: PathBuf,
    pub ast: AST,
}

pub fn import_library(path: &Path, state: Option<&Vec<Source>>) -> Vec<Source> {
    // scan state to prevent library link duplication and/or recursion.
    if let Some(state) = state {
        for source in state {
            if source.path == path {
                return vec![];
            }
        }
    }
    println!(
        "tasmc:\x1B[36m länkar fil [{:?}].\x1B[0m",
        &path.file_name().unwrap()
    );
    let src = fs::read_to_string(&path).expect("felaktig bibliotekssökväg!");

    let mut lexer = Lexer::new(&src);
    let tokens = lexer
        .lex()
        .expect("fel: lexikografisk analys misslyckades.");

    let mut parser = Parser::new(tokens.clone());
    let ast: AST = parser
        .parse()
        .expect("fel: syntaktisk analys misslyckades.");

    let mut sources: Vec<Source> = vec![Source {
        path: PathBuf::from(&path),
        ast: ast.clone(),
    }];
    for library in &ast.libraries {
        let mut lib_path: PathBuf = path.into();
        lib_path.pop();
        lib_path.extend(PathBuf::from(library).iter());

        // state is used for detecting duplicated or recursive library links.
        let mut child_state = sources.clone();
        if let Some(state) = state {
            child_state.extend(state.clone());
        }

        sources.extend(
            import_library(&lib_path, Some(&child_state))
                .iter()
                .cloned(),
        );
    }
    sources
}
