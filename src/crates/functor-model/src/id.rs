//! Opaque identifiers, slugs and user-facing artifact references.
//!
//! See `docs/design/identity-and-addressing.md` and ADR-0001.

use std::fmt;
use std::str::FromStr;
use std::sync::{Mutex, PoisonError};

use ulid::Ulid;

use crate::error::{ModelError, Result};

/// The kind of entity an identifier or canonical reference names (RFC-0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum EntityKind {
    /// An artifact.
    Artifact,
    /// A relationship.
    Relationship,
    /// A repository.
    Repository,
    /// An event.
    Event,
}

impl EntityKind {
    /// The lowercase name used in canonical references, for example `artifact`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Artifact => "artifact",
            Self::Relationship => "relationship",
            Self::Repository => "repository",
            Self::Event => "event",
        }
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

const ULID_LEN: usize = 26;

fn is_crockford_upper(b: u8) -> bool {
    matches!(b, b'0'..=b'9' | b'A'..=b'H' | b'J' | b'K' | b'M' | b'N' | b'P'..=b'T' | b'V'..=b'Z')
}

fn parse_ulid(field: &'static str, prefix: &str, input: &str) -> Result<Ulid> {
    let Some(body) = input.strip_prefix(prefix) else {
        return Err(ModelError::new(
            field,
            format!("must start with `{prefix}`"),
        ));
    };
    if body.len() != ULID_LEN || !body.bytes().all(is_crockford_upper) {
        return Err(ModelError::new(
            field,
            "must be followed by 26 uppercase Crockford base32 characters",
        ));
    }
    // 26 base32 characters carry 130 bits; the first must fit the top 3 bits of the 128-bit value,
    // otherwise parsing would silently wrap and text would not round-trip.
    if body.as_bytes()[0] > b'7' {
        return Err(ModelError::new(
            field,
            "is not a valid ULID: value too large",
        ));
    }
    Ulid::from_string(body).map_err(|_| ModelError::new(field, "is not a valid ULID"))
}

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident, $prefix:literal, $kind:ident, $field:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Ulid);

        impl $name {
            /// The text prefix of this identifier kind.
            pub const PREFIX: &'static str = $prefix;

            /// Builds an identifier from 128 random-and-time bits supplied by an id source.
            ///
            /// The bits are opaque: no accessor exposes them back.
            #[must_use]
            pub fn from_bits(bits: u128) -> Self {
                Self(Ulid::from(bits))
            }

            /// The canonical reference `functor:<entity-type>:<id>` (RFC-0005).
            #[must_use]
            pub fn canonical(&self) -> String {
                format!("functor:{}:{}", EntityKind::$kind, self)
            }

            /// Parses a canonical reference of this entity kind.
            ///
            /// # Errors
            /// Fails when the text is not a canonical reference naming this kind, or when the
            /// identifier part is malformed.
            pub fn from_canonical(input: &str) -> Result<Self> {
                let prefix = format!("functor:{}:", EntityKind::$kind);
                let Some(id) = input.strip_prefix(prefix.as_str()) else {
                    return Err(ModelError::new(
                        $field,
                        format!("must start with `{prefix}`"),
                    ));
                };
                id.parse()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }

        impl FromStr for $name {
            type Err = ModelError;

            fn from_str(input: &str) -> Result<Self> {
                parse_ulid($field, $prefix, input).map(Self)
            }
        }
    };
}

define_id!(
    /// The opaque, immutable identity of an artifact: `art_<ULID>`.
    ArtifactId, "art_", Artifact, "artifact id"
);
define_id!(
    /// The opaque, immutable identity of a relationship: `rel_<ULID>`.
    RelationshipId, "rel_", Relationship, "relationship id"
);
define_id!(
    /// The opaque, immutable identity of a repository: `repo_<ULID>`.
    RepositoryId, "repo_", Repository, "repository id"
);
define_id!(
    /// The opaque, immutable identity of an event: `evt_<ULID>`.
    EventId, "evt_", Event, "event id"
);

/// A monotonic source of identifier bits.
///
/// Within one source, successive values strictly increase, even within the same millisecond, so
/// ids sort in creation order. `functor-core` wraps this in its `IdGenerator`.
pub struct MonotonicUlids {
    inner: Mutex<ulid::Generator>,
}

impl fmt::Debug for MonotonicUlids {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonotonicUlids").finish_non_exhaustive()
    }
}

impl MonotonicUlids {
    /// Creates a source seeded from the system clock and entropy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ulid::Generator::new()),
        }
    }

    /// Returns the next identifier bits.
    #[must_use]
    pub fn next_bits(&self) -> u128 {
        let mut generator = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        // Overflow of the 80-bit random part within one millisecond is astronomically unlikely;
        // a fresh ULID is a safe fallback.
        generator.generate().unwrap_or_else(|_| Ulid::new()).into()
    }
}

impl Default for MonotonicUlids {
    fn default() -> Self {
        Self::new()
    }
}

/// A human-friendly, renamable alias for an artifact, unique within a repository.
///
/// Grammar: 1 to 64 characters of lowercase ASCII letters, digits and hyphens; no leading or
/// trailing hyphen; no consecutive hyphens.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Slug(String);

impl Slug {
    /// The maximum length of a slug.
    pub const MAX_LEN: usize = 64;

    /// Validates and wraps a slug.
    ///
    /// # Errors
    /// Fails when the text violates the slug grammar.
    pub fn parse(input: &str) -> Result<Self> {
        const SHAPE: &str =
            "lowercase letters, digits and single hyphens, not starting or ending with a hyphen";
        if input.is_empty() {
            return Err(ModelError::new("slug", "must not be empty"));
        }
        if input.len() > Self::MAX_LEN {
            return Err(ModelError::new(
                "slug",
                format!("must be at most {} characters", Self::MAX_LEN),
            ));
        }
        let valid_chars = input
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid_chars || input.starts_with('-') || input.ends_with('-') || input.contains("--") {
            return Err(ModelError::new("slug", format!("must be {SHAPE}")));
        }
        Ok(Self(input.to_owned()))
    }

    /// The slug text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Slug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Slug {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::parse(input)
    }
}

impl AsRef<str> for Slug {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// What a user can type to name an artifact: an id, a canonical reference or a slug.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ArtifactRef {
    /// An artifact identity.
    Id(ArtifactId),
    /// A slug alias.
    Slug(Slug),
}

impl ArtifactRef {
    /// Parses canonical references (`functor:artifact:art_...`), ids (`art_...`) and slugs.
    ///
    /// Text that looks like an id or a canonical reference but is malformed is an error; it is
    /// never retried as a slug. Because slugs cannot contain `_` or `:`, the forms are disjoint.
    ///
    /// # Errors
    /// Fails when the text is a malformed canonical reference or id, or violates the slug grammar.
    pub fn parse(input: &str) -> Result<Self> {
        if input.starts_with("functor:") {
            ArtifactId::from_canonical(input).map(Self::Id)
        } else if input.starts_with(ArtifactId::PREFIX) {
            input.parse().map(Self::Id)
        } else {
            Slug::parse(input).map(Self::Slug)
        }
    }
}

impl fmt::Display for ArtifactRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => id.fmt(f),
            Self::Slug(slug) => slug.fmt(f),
        }
    }
}

impl FromStr for ArtifactRef {
    type Err = ModelError;

    fn from_str(input: &str) -> Result<Self> {
        Self::parse(input)
    }
}

impl From<ArtifactId> for ArtifactRef {
    fn from(id: ArtifactId) -> Self {
        Self::Id(id)
    }
}

impl From<Slug> for ArtifactRef {
    fn from(slug: Slug) -> Self {
        Self::Slug(slug)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "art_01J9Z3K8M2Q7X4V6B5N1R0T8WD";

    #[test]
    fn id_text_round_trips() {
        let id: ArtifactId = ID.parse().unwrap();
        assert_eq!(id.to_string(), ID);
    }

    #[test]
    fn id_rejects_bad_text() {
        for bad in [
            "",
            "art_",
            "rel_01J9Z3K8M2Q7X4V6B5N1R0T8WD",
            "art_01j9z3k8m2q7x4v6b5n1r0t8wd",
            "art_01J9Z3K8M2Q7X4V6B5N1R0T8W",
            "art_01J9Z3K8M2Q7X4V6B5N1R0T8WDD",
            "art_01J9Z3K8M2Q7X4V6B5N1R0T8WI",
            "art_81J9Z3K8M2Q7X4V6B5N1R0T8WD",
            " art_01J9Z3K8M2Q7X4V6B5N1R0T8WD",
        ] {
            assert!(
                bad.parse::<ArtifactId>().is_err(),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn each_id_kind_has_its_prefix() {
        let bits = 0x0123_4567_89AB_CDEF_0123_4567_89AB_CDEF_u128;
        assert!(ArtifactId::from_bits(bits).to_string().starts_with("art_"));
        assert!(
            RelationshipId::from_bits(bits)
                .to_string()
                .starts_with("rel_")
        );
        assert!(
            RepositoryId::from_bits(bits)
                .to_string()
                .starts_with("repo_")
        );
        assert!(EventId::from_bits(bits).to_string().starts_with("evt_"));
    }

    #[test]
    fn id_kinds_do_not_parse_each_other() {
        let rel = RelationshipId::from_bits(42).to_string();
        assert!(rel.parse::<ArtifactId>().is_err());
        assert!(rel.parse::<RelationshipId>().is_ok());
    }

    #[test]
    fn canonical_reference_round_trips() {
        let id: ArtifactId = ID.parse().unwrap();
        let canonical = id.canonical();
        assert_eq!(canonical, format!("functor:artifact:{ID}"));
        assert_eq!(ArtifactId::from_canonical(&canonical).unwrap(), id);
        assert!(ArtifactId::from_canonical(&format!("functor:relationship:{ID}")).is_err());
        assert!(ArtifactId::from_canonical(ID).is_err());
    }

    #[test]
    fn monotonic_source_strictly_increases() {
        let source = MonotonicUlids::new();
        let mut last = source.next_bits();
        for _ in 0..1000 {
            let next = source.next_bits();
            assert!(next > last);
            last = next;
        }
    }

    #[test]
    fn slug_accepts_valid_grammar() {
        for good in ["a", "0", "rfc-0000", "a-b-c", &"a".repeat(64)] {
            assert!(Slug::parse(good).is_ok(), "{good:?} should be accepted");
        }
    }

    #[test]
    fn slug_rejects_invalid_grammar() {
        for bad in [
            "",
            "-a",
            "a-",
            "a--b",
            "A",
            "a_b",
            "a b",
            "a.b",
            "é",
            &"a".repeat(65),
        ] {
            assert!(Slug::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn artifact_ref_forms() {
        assert!(matches!(ArtifactRef::parse(ID), Ok(ArtifactRef::Id(_))));
        assert!(matches!(
            ArtifactRef::parse(&format!("functor:artifact:{ID}")),
            Ok(ArtifactRef::Id(_))
        ));
        assert!(matches!(
            ArtifactRef::parse("rfc-0000"),
            Ok(ArtifactRef::Slug(_))
        ));
    }

    #[test]
    fn malformed_id_is_not_retried_as_slug() {
        assert!(ArtifactRef::parse("art_xyz").is_err());
        assert!(ArtifactRef::parse("functor:artifact:xyz").is_err());
        assert!(ArtifactRef::parse("functor:relationship:rel_x").is_err());
        assert!(ArtifactRef::parse("rel_01J9Z3K8M2Q7X4V6B5N1R0T8WD").is_err());
    }

    #[test]
    fn artifact_ref_displays_its_input_form() {
        assert_eq!(
            ArtifactRef::parse("rfc-0000").unwrap().to_string(),
            "rfc-0000"
        );
        assert_eq!(ArtifactRef::parse(ID).unwrap().to_string(), ID);
    }
}
