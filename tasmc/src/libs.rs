use std::fs;
use std::path::{Path, PathBuf};

use crate::Flags;
use crate::errors::{ErrorData, error};
use crate::lexer::Lexer;
use crate::parser::{AST, Parser};

#[derive(Clone, Debug)]
pub struct Source {
    pub path: PathBuf,
    pub ast: AST,
}

pub fn import_library(
    path: &Path,
    state: Option<&Vec<Source>>,
    flags: &Flags,
) -> Result<Vec<Source>, Vec<ErrorData>> {
    // scan state to prevent library link duplication and/or recursion.
    if let Some(state) = state {
        for source in state {
            if source.path == path {
                return Ok(vec![]);
            }
        }
    }
    println!(
        "tasmc:\x1B[36m länkar fil [{:?}].\x1B[0m",
        &path.file_name().unwrap()
    );
    let src = fs::read_to_string(&path).expect("felaktig bibliotekssökväg!");
    if flags.dump_internal {
        dbg!(&src.chars());
    }

    let mut lexer = Lexer::new(&src, path.to_owned());
    let tokens = lexer.lex()?;
    if flags.dump_internal {
        dbg!(&tokens);
    }

    let mut parser = Parser::new(tokens.clone());
    let ast: AST = parser.parse()?;
    if flags.dump_internal {
        dbg!(&ast);
    }

    let mut sources: Vec<Source> = vec![Source {
        path: PathBuf::from(&path),
        ast: ast.clone(),
    }];

    let mut import_errors: Vec<ErrorData> = vec![];
    for library in &ast.libraries {
        let mut lib_path: PathBuf = path.into();
        lib_path.pop();
        lib_path.extend(PathBuf::from(library.path.clone()).iter());

        // verify that library path is reachable
        let lib_path = match fs::canonicalize(lib_path) {
            Ok(path) => path,
            Err(_) => {
                import_errors.push(error!(
                    UnreachableLibraryLink,
                    library.reference.clone(),
                    library.path.clone()
                ));
                continue;
            }
        };

        // state is used for detecting duplicated or recursive library links.
        let mut child_state = sources.clone();
        if let Some(state) = state {
            child_state.extend(state.clone());
        }

        sources.extend(
            import_library(&lib_path, Some(&child_state), flags)?
                .iter()
                .cloned(),
        );
    }
    if import_errors.is_empty() {
        Ok(sources)
    } else {
        Err(import_errors)
    }
}
