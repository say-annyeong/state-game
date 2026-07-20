use std::ops::Bound;
use crate::Namespace;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version([u64; 8]);

pub struct ModificationMetadata {
    pub namespace: Namespace,
    pub version: Version,
    pub api_version: Version,
    pub content_hash: Hash,

    pub dependencies: Vec<Dependency>,
    pub interactions: Vec<Interaction>,
}


pub struct Interaction {
    pub namespace: Namespace,
    pub kind: InteractionKind,
}


pub enum InteractionKind {
    /// This mod and the target mod modify overlapping functionality.
    /// They may work together depending on load order or patches.
    Conflict,

    /// This mod cannot function together with the target mod.
    Incompatible,
}

pub struct Dependency {
    pub namespace: Namespace,
    pub version: VersionRequirement,
    pub kind: DependencyKind,
}

pub enum DependencyKind {
    /// Mod cannot work without this.
    Required,

    /// Mod works without this but enables extra features.
    Optional,
}

pub enum VersionRequirement {
    Any,

    Exact(Version),

    AtLeast(Version),

    Compatible(Version),

    Range {
        min: Bound<Version>,
        max: Bound<Version>,
    },
}

pub enum Hash {
    Sha256([u8;32]),
    Blake3([u8;32]),
}