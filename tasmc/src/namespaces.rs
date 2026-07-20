use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use crate::libs::Source;
use crate::parser::{ASTArgument, ASTInstruction, ASTOpcode};

#[derive(Clone, Debug)]
pub struct Namespaces {
    pub private_identifiers: HashMap<PathBuf, HashMap<String, NamespaceIdentifier>>,
    pub public_identifiers: HashMap<String, NamespaceIdentifier>,
}

#[derive(Clone, Debug)]
pub struct NamespaceIdentifier {
    pub id: IdentifierHash,
    pub name: String,
    pub idtype: IdentifierType,
}

#[derive(Clone, Debug)]
pub enum IdentifierType {
    Tag,
    Variable(Option<u64>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdentifierHash(pub u64);

impl Namespaces {
    fn is_private(name: &str) -> bool {
        name.len() <= 0 || name.chars().nth(0).unwrap() == '_'
    }

    fn is_public(name: &str) -> bool {
        !Self::is_private(name)
    }

    // todo: there is a small chance of identifier collision.
    // since self is borrowed as mutable, the hashing function
    // can simply be replaced by an instance-local counter.
    pub fn hash_private(&mut self, path: &Path, name: &str) -> IdentifierHash {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        name.hash(&mut hasher);
        IdentifierHash(hasher.finish())
    }

    // -||-
    pub fn hash_public(&mut self, name: &str) -> IdentifierHash {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        IdentifierHash(hasher.finish())
    }

    // todo: big problem - this isn't type checked!!! identifiers may be
    // corrupted/exchanged under the hood if they are used incorrectly.
    pub fn get_identifier(
        &self,
        name: &str,
        path: Option<&Path>,
    ) -> Result<NamespaceIdentifier, ()> {
        // private identifier resolution.
        if Self::is_private(name)
            && let Some(path) = path
        {
            if let Some(identifiers) = self.private_identifiers.get(path)
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
        Err(())
    }

    // fails if identifier is already set.
    fn set_identifier(
        &mut self,
        name: &str,
        path: &Path,
        idtype: IdentifierType,
    ) -> Result<(), ()> {
        if Self::is_private(name) {
            let id = self.hash_private(path, name);
            let identifiers = self
                .private_identifiers
                .entry(path.to_owned())
                .or_insert(HashMap::new());
            if identifiers.contains_key(name) {
                eprintln!("! variabel redan definerad");
                return Err(());
            }
            identifiers.insert(
                name.to_owned(),
                NamespaceIdentifier {
                    id,
                    name: name.to_owned(),
                    idtype,
                },
            );
            Ok(())
        } else {
            if self.public_identifiers.contains_key(name) {
                eprintln!("! variabel redan definerad");
                return Err(());
            }
            let id = self.hash_public(name);
            self.public_identifiers.insert(
                name.to_owned(),
                NamespaceIdentifier {
                    id,
                    name: name.to_owned(),
                    idtype,
                },
            );
            Ok(())
        }
    }
}

impl TryFrom<&Vec<Source>> for Namespaces {
    type Error = ();
    fn try_from(sources: &Vec<Source>) -> Result<Self, Self::Error> {
        let mut namespaces = Namespaces {
            private_identifiers: HashMap::new(),
            public_identifiers: HashMap::new(),
        };

        for source in sources {
            for instruction in &source.ast.instructions {
                match instruction.clone() {
                    ASTInstruction::Tag(x) => {
                        namespaces.set_identifier(&x, &source.path, IdentifierType::Tag)?;
                    }
                    ASTInstruction::VMStep(opcode, args) => {
                        match opcode {
                            ASTOpcode::Set => (),
                            _ => continue,
                        }
                        if let Some(arg1) = args.get(0)
                            && let ASTArgument::Variable(name) = arg1
                        {
                            let value: Option<u64> = match args.get(1) {
                                None => None,
                                Some(ASTArgument::Immediate(x)) => Some(*x),
                                _ => {
                                    eprintln!("! ogiltigt initieringsvärde för `set`");
                                    return Err(());
                                }
                            };
                            if args.len() > 2 {
                                eprintln!("! överflödig data efter `set`");
                                return Err(());
                            }
                            namespaces.set_identifier(
                                &name,
                                &source.path,
                                IdentifierType::Variable(value),
                            )?;
                        } else {
                            eprintln!("! `set` måste efterföljas av en giltig identifierare");
                            return Err(());
                        }
                    }
                }
            }
        }

        Ok(namespaces)
    }
}
