use super::Identifier; // Defined in mod.rs

#[derive(Debug, PartialEq, Clone)]
pub enum ImportDeclaration {
    ImportModule {
        path: String, // The module path (e.g., "math", "./utils")
        alias: Option<Identifier>, // Optional alias (e.g., `as lmn`)
    },
    ImportSymbols {
        source: ImportSource, // Source can be a file or the special 'std' module
        symbols: Vec<(Identifier, Option<Identifier>)>, // Original name and optional alias
    },
}

/// Represents the source of an import (either a file path or the standard library).
#[derive(Debug, PartialEq, Clone)]
pub enum ImportSource {
    File(String), // For from "path" import ...
    Std,          // For from std import ...
}
