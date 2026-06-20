use crate::Namespace;

pub type Version = [u64; 8];

pub struct ModificationMetadata {
    pub namespace: Namespace,
    pub version: Version,
    pub application_programming_interface_version: Version,
    pub content_hash: u64,
    pub dependency: Vec<Dependency>,
}

pub enum DependencyKind {
    /// A required dependency.
    /// The system cannot function correctly without this dependency.
    /// Absence typically prevents loading or disables core functionality.
    Hard,

    /// An optional dependency.
    /// The system works without it, but may enable additional features if present.
    /// Safe to ignore if unavailable.
    Soft,

    /// A dependency that is not required but is preferred.
    /// Its presence improves performance, usability, or feature completeness.
    /// The system may prioritize configurations including this dependency.
    Recommended,

    /// A partially conflicting dependency.
    /// The system can run with both present, but behavior may be degraded,
    /// unstable, or partially incompatible.
    /// Usage together is allowed but discouraged; warnings may be issued.
    Conflict,

    /// A strictly incompatible dependency.
    /// The system must not allow this dependency combination.
    /// Presence of both typically prevents loading or triggers a hard error.
    Incompatible,
}


pub struct Dependency {
    pub kind: DependencyKind,
    pub namespace: Namespace,
    pub version: VersionRequirement,
}

pub enum VersionRequirement {
    Any,
    Exact(Version),
    Minimum(Version),
    Range {
        min: Version,
        max: Version,
    },
}
