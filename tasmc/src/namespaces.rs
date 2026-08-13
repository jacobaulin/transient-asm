use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::errors::{ErrorData, IndexReference, error};
use crate::libs::Source;
use crate::parser::{
    ASTArgument, ASTArgumentData, ASTInstructionData, ASTOpcodeData,
};

#[derive(Clone, Debug)]
pub struct Namespaces {
    pub private_identifiers:
        HashMap<PathBuf, HashMap<String, NamespaceIdentifier>>,
    pub public_identifiers: HashMap<String, NamespaceIdentifier>,

    /* used for uid generation. */
    state: u64,
}

#[derive(Clone, Debug)]
pub struct NamespaceIdentifier {
    pub uid: IdentifierUID,
    pub name: String,
    pub idtype: IdentifierType,
    pub init_reference: IndexReference,
}

#[derive(Clone, Debug)]
pub enum IdentifierType {
    Tag,
    Variable(Option<u64>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdentifierUID(pub u64);

impl Namespaces {
    fn is_private(name: &str) -> bool {
        name.len() <= 0 || name.chars().nth(0).unwrap() == '_'
    }

    pub fn create_uid(&mut self) -> IdentifierUID {
        let uid = self.state;
        self.state += 1;
        IdentifierUID(uid)
    }

    // todo: big problem - this isn't type checked!!! identifiers may be
    // corrupted/exchanged under the hood if they are used incorrectly.
    pub fn resolve_identifier(
        &self,
        name: &str,
        ident_ref: IndexReference,
    ) -> Result<NamespaceIdentifier, ErrorData> {
        // private identifier resolution.
        if Self::is_private(name) {
            if let Some(identifiers) =
                self.private_identifiers.get(&ident_ref.path)
                && let Some(identifier) = identifiers.get(name)
            {
                return Ok(identifier.clone());
            }
        }

        // public identifier resolution.
        if let Some(identifier) = self.public_identifiers.get(name) {
            return Ok(identifier.clone());
        }

        // identifier does not exist.
        Err(error!(IdentifierResolutionFailed, ident_ref, name.into()))
    }

    // fails if identifier is already set.
    fn set_identifier(
        &mut self,
        name: &str,
        path: &Path,
        idtype: IdentifierType,
        init_reference: IndexReference,
    ) -> Result<(), ErrorData> {
        if Self::is_private(name) {
            let uid = self.create_uid();
            let identifiers = self
                .private_identifiers
                .entry(path.to_owned())
                .or_insert(HashMap::new());
            if identifiers.contains_key(name) {
                return Err(error!(
                    IdentifierAlreadyDefined,
                    identifiers.get(name).unwrap().init_reference.clone(),
                    init_reference,
                    name.into()
                ));
            }
            identifiers.insert(
                name.to_owned(),
                NamespaceIdentifier {
                    uid,
                    name: name.to_owned(),
                    idtype,
                    init_reference,
                },
            );
            Ok(())
        } else {
            if self.public_identifiers.contains_key(name) {
                return Err(error!(
                    IdentifierAlreadyDefined,
                    self.public_identifiers
                        .get(name)
                        .unwrap()
                        .init_reference
                        .clone(),
                    init_reference,
                    name.into()
                ));
            }
            let uid = self.create_uid();
            self.public_identifiers.insert(
                name.to_owned(),
                NamespaceIdentifier {
                    uid,
                    name: name.to_owned(),
                    idtype,
                    init_reference,
                },
            );
            Ok(())
        }
    }
}

impl TryFrom<&Vec<Source>> for Namespaces {
    type Error = Vec<ErrorData>;
    fn try_from(sources: &Vec<Source>) -> Result<Self, Self::Error> {
        let mut namespaces = Namespaces {
            private_identifiers: HashMap::new(),
            public_identifiers: HashMap::new(),
            state: 0,
        };

        let mut error_data: Vec<ErrorData> = vec![];
        for source in sources {
            for instruction in &source.ast.instructions {
                match instruction.clone().inner {
                    ASTInstructionData::Tag(x) => {
                        match namespaces.set_identifier(
                            &x,
                            &source.path,
                            IdentifierType::Tag,
                            instruction.reference.clone(),
                        ) {
                            Ok(()) => (),
                            Err(err) => {
                                error_data.push(err);
                            }
                        };
                    }
                    ASTInstructionData::VMStep(opcode, args) => {
                        match opcode.inner {
                            ASTOpcodeData::Set => (),
                            _ => continue,
                        }
                        let name = match args.get(0) {
                            None => {
                                error_data.push(error!(
                                    NoSetVariable,
                                    instruction.reference.clone()
                                ));
                                continue;
                            }
                            Some(arg1)
                                if let ASTArgumentData::Variable(name) =
                                    arg1.inner.clone() =>
                            {
                                name
                            }
                            Some(err) => {
                                error_data.push(error!(
                                    InvalidSetVariable,
                                    err.reference.clone()
                                ));
                                continue;
                            }
                        };
                        let value: Option<u64> = match args.get(1) {
                            None => None,
                            Some(ASTArgument {
                                inner: ASTArgumentData::Immediate(x),
                                ..
                            }) => Some(*x),
                            Some(err) => {
                                error_data.push(error!(
                                    InvalidSetConstant,
                                    err.reference.clone(),
                                    name.clone()
                                ));
                                None /* placeholder to allow compiler to keep going */
                            }
                        };
                        if args.len() > 2 {
                            error_data.push(error!(
                                TrailingSetData,
                                args[2].reference.clone()
                            ));
                        }
                        match namespaces.set_identifier(
                            &name,
                            &source.path,
                            IdentifierType::Variable(value),
                            instruction.reference.clone(),
                        ) {
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
            Ok(namespaces)
        } else {
            Err(error_data)
        }
    }
}
