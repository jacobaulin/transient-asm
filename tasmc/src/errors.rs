use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct IndexReference {
    pub path: PathBuf,
    pub index: usize,
    pub len: usize,
}

impl IndexReference {
    pub fn new(path: PathBuf, index: usize, len: usize) -> Self {
        Self { path, index, len }
    }
}

pub enum Renderable {
    Header {
        reference: IndexReference,
        msg: String,
    },
    SourceView {
        reference: IndexReference,
        hint: Option<String>,
    },
}

struct LineData {
    content: String,
    line_num: usize,
    ref_index: usize,
    ref_len: usize,
}

impl From<&IndexReference> for LineData {
    fn from(reference: &IndexReference) -> Self {
        let text = fs::read_to_string(&reference.path)
            .expect("could not read source file during error rendering");
        let prefix_stub = text
            .chars()
            .take(reference.index + 1)
            .collect::<Vec<char>>()
            .into_iter();
        let prefix_newline =
            prefix_stub.clone().rposition(|c| c == '\n').unwrap_or(0);
        let line_num: usize =
            prefix_stub.clone().filter(|x| x == &'\n').count() + 1;
        let content: String =
            text.split('\n').collect::<Vec<&str>>()[line_num - 1].to_string();
        Self {
            content,
            line_num,
            ref_index: reference.index - prefix_newline - 1,
            ref_len: reference.len,
        }
    }
}

impl Renderable {
    pub fn new_header(reference: IndexReference, msg: String) -> Self {
        Renderable::Header { reference, msg }
    }

    pub fn new_source_view(
        reference: IndexReference,
        hint: Option<String>,
    ) -> Self {
        Renderable::SourceView { reference, hint }
    }

    pub fn render(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Renderable::Header { reference, msg } => {
                f.write_str(&std::format!(
                    "[{}:{}]\x1B[1m\x1B[31m fel: {}.\x1B[0m\n",
                    reference.path.to_str().unwrap(),
                    LineData::from(reference).line_num,
                    msg
                ))?;
                Ok(())
            }
            Renderable::SourceView { reference, hint } => {
                let line_data = LineData::from(reference);
                let content_prefix: &str =
                    &line_data.content[0..line_data.ref_index];
                let content_symbol: &str = &line_data.content[line_data
                    .ref_index
                    ..line_data.ref_index + line_data.ref_len];
                let content_suffix: &str = &line_data.content
                    [line_data.ref_index + line_data.ref_len..];

                f.write_str("    ╷\n")?;
                f.write_str(&format!(
                    "{:3} ┆ {}\x1B[4m{}\x1B[0m{}\n",
                    line_data.line_num,
                    content_prefix,
                    content_symbol,
                    content_suffix
                ))?;
                f.write_str(&format!(
                    "    ╵ {}↑ \x1B[1m{}\x1B[0m\n",
                    " ".repeat(content_prefix.len()),
                    hint.clone().unwrap_or("felet uppstod här".to_string()),
                ))?;
                Ok(())
            }
        }
    }
}

macro_rules! define_error_data {
    ($(( $typename:ident, $(($($data:ident),*))?, $self_ref:ident, $renderables:expr )),*) => {
        pub enum ErrorData {
            $(
                $typename(($($($data),*)?,))
            ),*
        }

        impl Display for ErrorData {
            fn fmt(&self, f: &mut Formatter) -> fmt::Result {
                match self {
                    $(
                        crate::errors::ErrorData::$typename(x) => {
                            let renderables = (|$self_ref: &($($($data),*)?,)| { $renderables }) (x);
                            for renderable in renderables {
                                renderable.render(f)?;
                            }
                            Ok(())
                        }
                    ),*
                }
            }
        }
    }
}

macro_rules! error {
    ($typename: ident, $($data:expr),*) => {
        crate::errors::ErrorData::$typename(($($data),*,))
    }
}
pub(crate) use error;

macro_rules! user_fatal {
    ($($arg:expr),+) => {
        eprintln!("fel: {}.", format!($($arg),+));
        std::process::exit(1);
    }
}

macro_rules! internal_fatal {
    ($($arg:expr),+) => {
        eprintln!("\n--- !! kompilatorhaveri !! ---");
        eprintln!("ödesdigert haveri: kompilatorn stötte på ett internt problem och kunde ej återhämtas!");
        eprintln!("info: detta är en allvarlig kompilatorbugg. var god rapportera problemet.");
        eprintln!("info: feldata: {}.", format!($($arg),+));

        panic!("internt kompilatorhaveri")
    }
}
pub(crate) use internal_fatal;

pub fn err_and_quit(error_data: Vec<ErrorData>) -> ! {
    for err in error_data {
        eprintln!("{}", err);
    }
    std::process::exit(1);
}

define_error_data!(
    (
        InvalidInputByte, (IndexReference, char), data, vec![
            Renderable::new_header(
                data.0.clone(),
                format!("symbol `{}` stöds ej av språket", data.1),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("symbolen påträffades här".into()),
            ),
        ]
    ),
    (
        InvalidCompilerOperation, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "kompilatorkommando `*` måste efterföljas av ett giltigt alternativ".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("kompilatorkommando anropat här".into()),
            ),
        ]
    ),
    (
        InvalidNumberLexeme, (IndexReference, String), data, vec![
            Renderable::new_header(
                data.0.clone(),
                format!("kunde ej tolka `{}` som ett tal", data.1),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("talet förväntades här".into()),
            ),
        ]
    ),
    (
        MissingPointerIdentifier, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "pekaroperationen måste appliceras på en variabel".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("pekaroperationen används här".into()),
            ),
        ]
    ),
    (
        MissingAddressIdentifier, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "adressoperationen måste appliceras på en variabel".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("adressoperationen används här".into()),
            ),
        ]
    ),
    (
        MissingTagIdentifier, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "en tagg måste följas av en identifierare".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("taggen används här".into()),
            ),
        ]
    ),
    (
        InvalidArgumentSymbol, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "ogiltigt argument till instruktion".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("argumentet används här".into()),
            ),
        ]
    ),
    (
        ArgumentExpected, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "argument förväntades".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some(
                    "argumentet förväntades till den här funktionen".into(),
                ),
            ),
        ]
    ),
    (
        TrailingTagData, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "överflödiga data efter tagg".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("överflödiga data börjar här".into()),
            ),
        ]
    ),
    (
        MissingLibraryIdentifier, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "bibliotekslänk måste efterföljas av en identifierare"
                    .into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("bibliotekslänk används här".into()),
            ),
        ]
    ),
    (
        TrailingLibraryData, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "överflödiga data efter bibliotekslänk".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("överflödiga data börjar här".into()),
            ),
        ]
    ),
    (
        InvalidOpcode, (IndexReference, String), data, vec![
            Renderable::new_header(
                data.0.clone(),
                format!("operationskoden `{}` är ogiltig", data.1),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("ogiltig operationskod används här".into()),
            ),
        ]
    ),
    (
        InvalidStatement, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "endast instruktioner och taggar stöds".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("ogiltig instruktion påträffades här".into()),
            ),
        ]
    ),
    (
        UnexpectedSymbolTermination, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "oväntat slut på källkod".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("mer data förväntades här".into()),
            ),
        ]
    ),
    (
        UnreachableLibraryLink, (IndexReference, String), data, vec![
            Renderable::new_header(
                data.0.clone(),
                format!(
                    "biblioteket `{}` är oåtkomligt",
                    data.1
                ),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("biblioteket försökte länkas här".into()),
            ),
        ]
    ),
    (
        IdentifierResolutionFailed, (IndexReference, String), data, vec![
            Renderable::new_header(
                data.0.clone(),
                format!(
                    "variabeln `{}` existerar inte",
                    data.1
                ),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("variabeln försökte användas här".into()),
            ),
        ]
    ),
    (
        IdentifierAlreadyDefined, (IndexReference, IndexReference, String), data, vec![
            Renderable::new_header(
                data.1.clone(),
                format!(
                    "variabeln `{}` är redan definerad",
                    data.2
                ),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("variabeln definerades först här...".into()),
            ),
            Renderable::new_source_view(
                data.1.clone(),
                Some("...för att sedan omdefineras här".into()),
            ),
        ]
    ),
    (
        InvalidSetConstant, (IndexReference, String), data, vec![
            Renderable::new_header(
                data.0.clone(),
                format!(
                    "ogiltigt initieringsvärde för variabeln `{}`",
                    data.1
                ),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("ett initieringsvärde förväntades här".into()),
            ),
        ]
    ),
    (
        NoSetVariable, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "`set` måste efterföljas av en variabel".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("`set` används här".into()),
            ),
        ]
    ),
    (
        InvalidSetVariable, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "ogiltigt argument vid `set`".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("variabel förväntades här".into()),
            ),
        ]
    ),
    (
        TrailingSetData, (IndexReference), data, vec![
            Renderable::new_header(
                data.0.clone(),
                "överflödiga data påträffades efter `set`".into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("överflödiga data börjar här".into()),
            ),
        ]
    ),
    (
        InvalidASTArgumentData, (IndexReference, String, String), data, vec![
            Renderable::new_header(
                data.0.clone(),
                format!(
                    "argumentet förväntades vara en {}, men var i stället en {}",
                    data.1,
                    data.2
                ).into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("överflödiga data börjar här".into()),
            ),
        ]
    ),
    (
        InvalidArgumentNumber, (IndexReference, usize, usize), data, vec![
            Renderable::new_header(
                data.0.clone(),
                format!(
                    "en instruktion förväntade sig {} argument, men {} argument påträffades",
                    data.1,
                    data.2
                ).into(),
            ),
            Renderable::new_source_view(
                data.0.clone(),
                Some("instruktionen används här".into()),
            ),
        ]
    )
);
