//! Property tests for the invariants listed in `docs/design/domain-model.md`.

use functor_model::{
    ArtifactId, ArtifactRef, ContentPath, EventId, RelationshipId, RelationshipType,
    RelationshipTypeName, RepositoryId, Slug, Tag, Tags, Timestamp,
};
use proptest::prelude::*;

proptest! {
    #[test]
    fn artifact_id_text_round_trips(bits in any::<u128>()) {
        let id = ArtifactId::from_bits(bits);
        let text = id.to_string();
        prop_assert_eq!(text.len(), 4 + 26);
        prop_assert_eq!(text.parse::<ArtifactId>().unwrap(), id);
        prop_assert_eq!(ArtifactId::from_canonical(&id.canonical()).unwrap(), id);
    }

    #[test]
    fn every_id_kind_round_trips_and_is_prefix_disjoint(bits in any::<u128>()) {
        let art = ArtifactId::from_bits(bits).to_string();
        let rel = RelationshipId::from_bits(bits).to_string();
        let repo = RepositoryId::from_bits(bits).to_string();
        let evt = EventId::from_bits(bits).to_string();
        prop_assert_eq!(rel.parse::<RelationshipId>().unwrap().to_string(), rel.clone());
        prop_assert_eq!(repo.parse::<RepositoryId>().unwrap().to_string(), repo.clone());
        prop_assert_eq!(evt.parse::<EventId>().unwrap().to_string(), evt.clone());
        prop_assert!(art.parse::<RelationshipId>().is_err());
        prop_assert!(rel.parse::<ArtifactId>().is_err());
        prop_assert!(repo.parse::<EventId>().is_err());
    }

    #[test]
    fn id_ordering_follows_creation_order(a in any::<u128>(), b in any::<u128>()) {
        let (ia, ib) = (ArtifactId::from_bits(a), ArtifactId::from_bits(b));
        prop_assert_eq!(a.cmp(&b), ia.cmp(&ib));
    }

    #[test]
    fn arbitrary_text_never_parses_as_an_id_unless_it_round_trips(text in ".{0,40}") {
        if let Ok(id) = text.parse::<ArtifactId>() {
            prop_assert_eq!(id.to_string(), text);
        }
    }

    #[test]
    fn near_valid_id_text_round_trips_or_is_rejected(body in "[0-9A-Z]{26}") {
        let text = format!("art_{body}");
        if let Ok(id) = text.parse::<ArtifactId>() {
            prop_assert_eq!(id.to_string(), text);
        }
    }

    #[test]
    fn valid_slugs_are_accepted_and_stable(text in "[a-z0-9]([a-z0-9]|-[a-z0-9]){0,30}") {
        let slug = Slug::parse(&text).unwrap();
        prop_assert_eq!(slug.as_str(), text.as_str());
        prop_assert_eq!(Slug::parse(&format!("{slug}")).unwrap(), slug);
    }

    #[test]
    fn no_valid_slug_parses_as_an_id_or_canonical_reference(text in ".{0,40}") {
        if Slug::parse(&text).is_ok() {
            prop_assert!(!text.starts_with("art_"));
            prop_assert!(!text.starts_with("functor:"));
            prop_assert!(matches!(ArtifactRef::parse(&text), Ok(ArtifactRef::Slug(_))));
        }
    }

    #[test]
    fn slug_parse_accepts_only_the_documented_grammar(text in ".{0,70}") {
        let expected = !text.is_empty()
            && text.len() <= 64
            && text.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !text.starts_with('-')
            && !text.ends_with('-')
            && !text.contains("--");
        prop_assert_eq!(Slug::parse(&text).is_ok(), expected);
    }

    #[test]
    fn relationship_type_text_round_trips(name in "[a-z][a-z0-9_]{0,63}") {
        let parsed = RelationshipType::parse(&name).unwrap();
        prop_assert_eq!(parsed.to_string(), name.clone());
        prop_assert_eq!(RelationshipType::parse(&parsed.to_string()).unwrap(), parsed.clone());
        // A kebab-case spelling normalizes to the same type.
        prop_assert_eq!(RelationshipType::parse(&name.replace('_', "-")).unwrap(), parsed);
    }

    #[test]
    fn built_in_names_never_become_custom(index in 0usize..4, kebab in any::<bool>()) {
        let name = ["depends_on", "references", "derived_from", "extends"][index];
        let input = if kebab { name.replace('_', "-") } else { name.to_owned() };
        let parsed = RelationshipType::parse(&input).unwrap();
        prop_assert!(parsed.is_built_in());
        prop_assert!(RelationshipTypeName::new(name).is_err());
    }

    #[test]
    fn tags_are_sorted_and_unique(names in prop::collection::vec("[a-z0-9][a-z0-9_-]{0,10}", 0..12)) {
        let tags = Tags::new(names.iter().map(|n| Tag::new(n).unwrap()));
        let listed: Vec<&str> = tags.iter().map(Tag::as_str).collect();
        let mut expected: Vec<&str> = names.iter().map(String::as_str).collect();
        expected.sort_unstable();
        expected.dedup();
        prop_assert_eq!(listed, expected);
    }

    #[test]
    fn timestamp_text_round_trips(seconds in Timestamp::MIN.unix_seconds()..=Timestamp::MAX.unix_seconds()) {
        let ts = Timestamp::from_unix_seconds(seconds).unwrap();
        let text = ts.to_string();
        prop_assert_eq!(text.len(), 20);
        prop_assert_eq!(Timestamp::parse(&text).unwrap(), ts);
        prop_assert_eq!(Timestamp::parse(&text).unwrap().unix_seconds(), seconds);
    }

    #[test]
    fn timestamp_order_matches_text_order(a in 0i64..4_000_000_000, b in 0i64..4_000_000_000) {
        let (ta, tb) = (Timestamp::from_unix_seconds(a).unwrap(), Timestamp::from_unix_seconds(b).unwrap());
        prop_assert_eq!(a.cmp(&b), ta.to_string().cmp(&tb.to_string()));
    }

    #[test]
    fn accepted_content_paths_are_always_safe(text in ".{0,40}") {
        if let Ok(path) = ContentPath::new(&text) {
            let s = path.as_str();
            prop_assert!(!s.starts_with('/'));
            prop_assert!(!s.contains('\\'));
            prop_assert!(!s.contains(':'));
            prop_assert!(s.split('/').all(|seg| !seg.is_empty() && seg != "." && seg != ".."));
        }
    }
}
