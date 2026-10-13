//! Graph traversal and the relationships touching one artifact.

use std::collections::{BTreeMap, BTreeSet};

use functor_core::{FunctorError, Result};
use functor_model::{
    Artifact, ArtifactId, ArtifactRef, Direction, EntityKind, Relationship, RelationshipId,
    RelationshipType, RepositorySnapshot, Traversal, TraversalNode, TraversalResult,
};

fn follows(direction: Direction) -> Result<(bool, bool)> {
    match direction {
        Direction::Outbound => Ok((true, false)),
        Direction::Inbound => Ok((false, true)),
        Direction::Both => Ok((true, true)),
        // `Direction` is non-exhaustive; a variant this engine does not know is a bug.
        _ => Err(FunctorError::Internal(
            "unsupported traversal direction".to_owned(),
        )),
    }
}

fn wanted(types: &[RelationshipType], relationship: &Relationship) -> bool {
    types.is_empty() || types.contains(relationship.relationship_type())
}

fn artifact_of<'a>(snapshot: &'a RepositorySnapshot, id: &ArtifactId) -> Result<&'a Artifact> {
    snapshot.artifact(id).ok_or_else(|| {
        FunctorError::Internal(format!(
            "snapshot relationship endpoint {id} has no artifact"
        ))
    })
}

fn resolve<'a>(snapshot: &'a RepositorySnapshot, reference: &ArtifactRef) -> Result<&'a Artifact> {
    snapshot
        .resolve(reference)
        .ok_or_else(|| FunctorError::not_found(EntityKind::Artifact, reference.to_string()))
}

/// Breadth-first traversal from `traversal.start`.
///
/// Every reachable artifact is reported once, at its minimum depth, with the first edge that
/// reached it (edges of one level are taken in ascending relationship id). The start is never
/// reported, and cycles terminate because visited artifacts are not expanded again.
///
/// `edges` lists each edge examined from an artifact closer to the start than the depth limit,
/// once, ordered by relationship id; this includes edges that lead to artifacts already visited,
/// so cycles stay visible. `nodes` are ordered by depth, then slug, then id.
///
/// # Errors
/// `NotFound` when the start does not resolve.
pub fn traverse(snapshot: &RepositorySnapshot, traversal: &Traversal) -> Result<TraversalResult> {
    let start = resolve(snapshot, &traversal.start)?;
    let (outbound, inbound) = follows(traversal.direction)?;
    let types = traversal.relationship_types.as_slice();

    let mut visited = BTreeSet::from([*start.id()]);
    let mut frontier = vec![*start.id()];
    let mut edges: BTreeMap<RelationshipId, &Relationship> = BTreeMap::new();
    let mut nodes = Vec::new();
    let mut depth = 0_u32;

    while !frontier.is_empty() && traversal.max_depth.is_none_or(|max| depth < max) {
        depth += 1;
        let mut steps: Vec<(&Relationship, ArtifactId)> = Vec::new();
        for id in &frontier {
            if outbound {
                steps.extend(
                    snapshot
                        .outgoing(id)
                        .filter(|r| wanted(types, r))
                        .map(|r| (r, *r.target())),
                );
            }
            if inbound {
                steps.extend(
                    snapshot
                        .incoming(id)
                        .filter(|r| wanted(types, r))
                        .map(|r| (r, *r.source())),
                );
            }
        }
        steps.sort_by_key(|(relationship, _)| *relationship.id());

        let mut next = Vec::new();
        for (relationship, neighbour) in steps {
            edges.insert(*relationship.id(), relationship);
            if visited.insert(neighbour) {
                next.push(neighbour);
                nodes.push(TraversalNode {
                    artifact: artifact_of(snapshot, &neighbour)?.clone(),
                    depth,
                    via: relationship.clone(),
                });
            }
        }
        frontier = next;
    }

    nodes.sort_by(|a, b| {
        (a.depth, a.artifact.slug(), a.artifact.id()).cmp(&(
            b.depth,
            b.artifact.slug(),
            b.artifact.id(),
        ))
    });
    Ok(TraversalResult {
        start: start.clone(),
        nodes,
        edges: edges.into_values().cloned().collect(),
    })
}

/// Every relationship touching an artifact, ordered by relationship id.
///
/// `Outbound` keeps the relationships the artifact is the source of, `Inbound` those it is the
/// target of, and `Both` all of them; a self-referencing relationship appears once. An empty
/// `types` keeps every type.
///
/// # Errors
/// `NotFound` when the artifact does not resolve.
pub fn relationships_of(
    snapshot: &RepositorySnapshot,
    reference: &ArtifactRef,
    types: &[RelationshipType],
    direction: Direction,
) -> Result<(Artifact, Vec<Relationship>)> {
    let artifact = resolve(snapshot, reference)?;
    let (outbound, inbound) = follows(direction)?;

    let mut found: BTreeMap<RelationshipId, &Relationship> = BTreeMap::new();
    if outbound {
        for relationship in snapshot.outgoing(artifact.id()) {
            found.insert(*relationship.id(), relationship);
        }
    }
    if inbound {
        for relationship in snapshot.incoming(artifact.id()) {
            found.insert(*relationship.id(), relationship);
        }
    }
    let relationships = found
        .into_values()
        .filter(|relationship| wanted(types, relationship))
        .cloned()
        .collect();
    Ok((artifact.clone(), relationships))
}

#[cfg(test)]
mod tests {
    use functor_core::ErrorKind;
    use functor_testkit::RepoFixture;

    use super::*;

    fn reference(text: &str) -> ArtifactRef {
        ArtifactRef::parse(text).unwrap()
    }

    fn run(
        fixture: &RepoFixture,
        start: &str,
        types: &[&str],
        direction: Direction,
        max_depth: Option<u32>,
    ) -> TraversalResult {
        let mut traversal = Traversal::new(reference(start))
            .with_types(types.iter().map(|t| RelationshipType::parse(t).unwrap()))
            .with_direction(direction);
        traversal.max_depth = max_depth;
        traverse(&fixture.build(), &traversal).unwrap()
    }

    /// `(slug, depth)` of every reported node.
    fn reached(result: &TraversalResult) -> Vec<(&str, u32)> {
        result
            .nodes
            .iter()
            .map(|n| (n.artifact.slug().as_str(), n.depth))
            .collect()
    }

    /// `source -> target` labels of the examined edges, by relationship id.
    fn edge_labels(fixture: &RepoFixture, result: &TraversalResult) -> Vec<String> {
        let label = |id: &ArtifactId| {
            fixture
                .artifacts()
                .iter()
                .find(|a| a.id() == id)
                .unwrap()
                .slug()
                .to_string()
        };
        result
            .edges
            .iter()
            .map(|r| format!("{}>{}", label(r.source()), label(r.target())))
            .collect()
    }

    /// `d` depends on `c` depends on `b` depends on `a`.
    fn chain() -> RepoFixture {
        RepoFixture::new()
            .artifact("a")
            .artifact("b")
            .artifact("c")
            .artifact("d")
            .relation("b", "depends_on", "a")
            .relation("c", "depends_on", "b")
            .relation("d", "depends_on", "c")
    }

    #[test]
    fn a_chain_is_followed_in_both_directions() {
        let fixture = chain();
        let out = run(&fixture, "d", &[], Direction::Outbound, None);
        assert_eq!(reached(&out), [("c", 1), ("b", 2), ("a", 3)]);
        assert_eq!(out.start.slug().as_str(), "d");

        let inbound = run(&fixture, "a", &[], Direction::Inbound, None);
        assert_eq!(reached(&inbound), [("b", 1), ("c", 2), ("d", 3)]);

        // Nothing leaves `a` outbound and nothing arrives at `d`.
        assert_eq!(run(&fixture, "a", &[], Direction::Outbound, None).nodes, []);
        assert_eq!(run(&fixture, "d", &[], Direction::Inbound, None).nodes, []);
    }

    #[test]
    fn depth_limits_cut_the_closure() {
        let fixture = chain();
        let at = |depth| run(&fixture, "d", &[], Direction::Outbound, depth);
        assert_eq!(reached(&at(Some(0))), []);
        assert_eq!(at(Some(0)).edges, []);
        assert_eq!(reached(&at(Some(1))), [("c", 1)]);
        assert_eq!(reached(&at(Some(2))), [("c", 1), ("b", 2)]);
        assert_eq!(reached(&at(Some(3))), [("c", 1), ("b", 2), ("a", 3)]);
        assert_eq!(reached(&at(Some(99))), reached(&at(None)));
    }

    #[test]
    fn edges_cover_only_what_the_depth_limit_examined() {
        let fixture = chain();
        let at = |depth| {
            let result = run(&fixture, "d", &[], Direction::Outbound, depth);
            edge_labels(&fixture, &result)
        };
        assert_eq!(at(Some(1)), ["d>c"]);
        assert_eq!(at(Some(2)), ["c>b", "d>c"]);
        assert_eq!(at(None), ["b>a", "c>b", "d>c"]);
    }

    #[test]
    fn via_is_the_edge_that_reached_the_node() {
        let fixture = chain();
        let result = run(&fixture, "d", &[], Direction::Outbound, None);
        let via: Vec<(String, String)> = result
            .nodes
            .iter()
            .map(|n| {
                let label = |id: &ArtifactId| {
                    fixture
                        .artifacts()
                        .iter()
                        .find(|a| a.id() == id)
                        .unwrap()
                        .slug()
                        .to_string()
                };
                (label(n.via.source()), label(n.via.target()))
            })
            .collect();
        assert_eq!(
            via,
            [
                ("d".into(), "c".into()),
                ("c".into(), "b".into()),
                ("b".into(), "a".into())
            ]
        );
    }

    /// `top` depends on `left` and `right`, both depend on `base`.
    fn diamond() -> RepoFixture {
        RepoFixture::new()
            .artifact("top")
            .artifact("left")
            .artifact("right")
            .artifact("base")
            .relation("top", "depends_on", "left")
            .relation("top", "depends_on", "right")
            .relation("left", "depends_on", "base")
            .relation("right", "depends_on", "base")
    }

    #[test]
    fn a_diamond_reports_the_shared_node_once_at_its_minimum_depth() {
        let fixture = diamond();
        let result = run(&fixture, "top", &[], Direction::Outbound, None);
        assert_eq!(reached(&result), [("left", 1), ("right", 1), ("base", 2)]);
        // Both edges into `base` are examined, so the diamond's shape stays visible.
        assert_eq!(
            edge_labels(&fixture, &result),
            ["top>left", "top>right", "left>base", "right>base"]
        );
    }

    #[test]
    fn via_is_the_lowest_relationship_id_among_the_edges_of_a_level() {
        let fixture = diamond();
        let result = run(&fixture, "top", &[], Direction::Outbound, None);
        let base = result
            .nodes
            .iter()
            .find(|n| n.artifact.slug().as_str() == "base")
            .unwrap();
        let left = fixture.id_of("left").unwrap();
        assert_eq!(*base.via.source(), left, "left>base was created first");
    }

    #[test]
    fn the_shortest_path_wins_when_a_longer_one_exists() {
        // a -> b -> c and a -> c: c is at depth 1, not 2.
        let fixture = RepoFixture::new()
            .artifact("a")
            .artifact("b")
            .artifact("c")
            .relation("a", "depends_on", "b")
            .relation("b", "depends_on", "c")
            .relation("a", "depends_on", "c");
        let result = run(&fixture, "a", &[], Direction::Outbound, None);
        assert_eq!(reached(&result), [("b", 1), ("c", 1)]);
        assert_eq!(edge_labels(&fixture, &result), ["a>b", "b>c", "a>c"]);
    }

    #[test]
    fn cycles_terminate_and_never_report_the_start() {
        let fixture = RepoFixture::new()
            .artifact("a")
            .artifact("b")
            .artifact("c")
            .relation("a", "depends_on", "b")
            .relation("b", "depends_on", "c")
            .relation("c", "depends_on", "a");
        let result = run(&fixture, "a", &[], Direction::Outbound, None);
        assert_eq!(reached(&result), [("b", 1), ("c", 2)]);
        assert_eq!(edge_labels(&fixture, &result), ["a>b", "b>c", "c>a"]);

        let inbound = run(&fixture, "a", &[], Direction::Inbound, None);
        assert_eq!(reached(&inbound), [("c", 1), ("b", 2)]);
    }

    #[test]
    fn a_self_loop_is_an_edge_but_not_a_node() {
        let fixture = RepoFixture::new()
            .artifact("a")
            .artifact("b")
            .relation("a", "references", "a")
            .relation("a", "references", "b");
        let result = run(&fixture, "a", &[], Direction::Outbound, None);
        assert_eq!(reached(&result), [("b", 1)]);
        assert_eq!(edge_labels(&fixture, &result), ["a>a", "a>b"]);
        let both = run(&fixture, "a", &[], Direction::Both, None);
        assert_eq!(reached(&both), [("b", 1)]);
        assert_eq!(edge_labels(&fixture, &both), ["a>a", "a>b"]);
    }

    #[test]
    fn disconnected_artifacts_are_not_reached() {
        let fixture = RepoFixture::new()
            .artifact("a")
            .artifact("b")
            .artifact("island")
            .relation("a", "depends_on", "b");
        let result = run(&fixture, "a", &[], Direction::Both, None);
        assert_eq!(reached(&result), [("b", 1)]);
        let lonely = run(&fixture, "island", &[], Direction::Both, None);
        assert_eq!(lonely.nodes, []);
        assert_eq!(lonely.edges, []);
    }

    #[test]
    fn relationship_types_limit_what_is_followed() {
        let fixture = RepoFixture::new()
            .artifact("a")
            .artifact("b")
            .artifact("c")
            .artifact("d")
            .relation("a", "depends_on", "b")
            .relation("a", "references", "c")
            .relation("c", "depends_on", "d");
        let all = run(&fixture, "a", &[], Direction::Outbound, None);
        assert_eq!(reached(&all), [("b", 1), ("c", 1), ("d", 2)]);

        let depends = run(&fixture, "a", &["depends_on"], Direction::Outbound, None);
        assert_eq!(reached(&depends), [("b", 1)]);
        assert_eq!(edge_labels(&fixture, &depends), ["a>b"]);

        let references = run(&fixture, "a", &["references"], Direction::Outbound, None);
        assert_eq!(reached(&references), [("c", 1)]);

        let several = run(
            &fixture,
            "a",
            &["depends-on", "references"],
            Direction::Outbound,
            None,
        );
        assert_eq!(reached(&several), [("b", 1), ("c", 1), ("d", 2)]);

        let none = run(&fixture, "a", &["extends"], Direction::Outbound, None);
        assert_eq!(none.nodes, []);
    }

    #[test]
    fn both_directions_expand_from_the_middle_of_a_chain() {
        let fixture = chain();
        let result = run(&fixture, "b", &[], Direction::Both, None);
        assert_eq!(reached(&result), [("a", 1), ("c", 1), ("d", 2)]);
        let limited = run(&fixture, "b", &[], Direction::Both, Some(1));
        assert_eq!(reached(&limited), [("a", 1), ("c", 1)]);
        assert_eq!(edge_labels(&fixture, &limited), ["b>a", "c>b"]);
    }

    #[test]
    fn nodes_are_ordered_by_depth_then_slug_not_by_discovery() {
        let fixture = RepoFixture::new()
            .artifact("root")
            .artifact("zebra")
            .artifact("apple")
            .artifact("mango")
            .artifact("deep-b")
            .artifact("deep-a")
            .relation("root", "depends_on", "zebra")
            .relation("root", "depends_on", "apple")
            .relation("root", "depends_on", "mango")
            .relation("zebra", "depends_on", "deep-b")
            .relation("apple", "depends_on", "deep-a");
        let result = run(&fixture, "root", &[], Direction::Outbound, None);
        assert_eq!(
            reached(&result),
            [
                ("apple", 1),
                ("mango", 1),
                ("zebra", 1),
                ("deep-a", 2),
                ("deep-b", 2)
            ]
        );
    }

    #[test]
    fn the_start_resolves_by_slug_id_or_canonical_reference_only_when_it_exists() {
        let fixture = chain();
        let snapshot = fixture.build();
        let id = fixture.id_of("d").unwrap();
        for text in ["d".to_owned(), id.to_string(), id.canonical()] {
            let traversal = Traversal::new(reference(&text));
            let result = traverse(&snapshot, &traversal).unwrap();
            assert_eq!(result.start.slug().as_str(), "d");
        }
        let missing = traverse(&snapshot, &Traversal::new(reference("nope"))).unwrap_err();
        assert_eq!(missing.kind(), ErrorKind::NotFound);
        let unknown_id = ArtifactId::from_bits(9999);
        let missing = traverse(&snapshot, &Traversal::new(unknown_id.into())).unwrap_err();
        assert_eq!(missing.kind(), ErrorKind::NotFound);
    }

    #[test]
    fn traversal_is_deterministic() {
        let fixture = diamond();
        let first = run(&fixture, "top", &[], Direction::Both, None);
        let second = run(&fixture, "top", &[], Direction::Both, None);
        assert_eq!(first, second);
    }

    // ---- relationships_of ----

    fn touching(
        fixture: &RepoFixture,
        slug: &str,
        types: &[&str],
        direction: Direction,
    ) -> Vec<String> {
        let types: Vec<RelationshipType> = types
            .iter()
            .map(|t| RelationshipType::parse(t).unwrap())
            .collect();
        let (artifact, relationships) =
            relationships_of(&fixture.build(), &reference(slug), &types, direction).unwrap();
        assert_eq!(artifact.slug().as_str(), slug);
        let label = |id: &ArtifactId| {
            fixture
                .artifacts()
                .iter()
                .find(|a| a.id() == id)
                .unwrap()
                .slug()
                .to_string()
        };
        relationships
            .iter()
            .map(|r| {
                format!(
                    "{}>{}:{}",
                    label(r.source()),
                    label(r.target()),
                    r.relationship_type()
                )
            })
            .collect()
    }

    fn mixed() -> RepoFixture {
        RepoFixture::new()
            .artifact("a")
            .artifact("b")
            .artifact("c")
            .relation("b", "depends_on", "a")
            .relation("a", "references", "c")
            .relation("c", "depends_on", "a")
            .relation("a", "references", "a")
            .relation("b", "references", "c")
    }

    #[test]
    fn relationships_of_returns_both_directions_ordered_by_id() {
        let fixture = mixed();
        assert_eq!(
            touching(&fixture, "a", &[], Direction::Both),
            [
                "b>a:depends_on",
                "a>c:references",
                "c>a:depends_on",
                "a>a:references"
            ]
        );
    }

    #[test]
    fn relationships_of_filters_by_direction() {
        let fixture = mixed();
        assert_eq!(
            touching(&fixture, "a", &[], Direction::Outbound),
            ["a>c:references", "a>a:references"]
        );
        assert_eq!(
            touching(&fixture, "a", &[], Direction::Inbound),
            ["b>a:depends_on", "c>a:depends_on", "a>a:references"]
        );
    }

    #[test]
    fn relationships_of_filters_by_type() {
        let fixture = mixed();
        assert_eq!(
            touching(&fixture, "a", &["depends_on"], Direction::Both),
            ["b>a:depends_on", "c>a:depends_on"]
        );
        assert_eq!(
            touching(
                &fixture,
                "a",
                &["depends_on", "references"],
                Direction::Outbound
            ),
            ["a>c:references", "a>a:references"]
        );
        assert_eq!(
            touching(&fixture, "a", &["extends"], Direction::Both),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_self_loop_is_listed_once_even_for_both_directions() {
        let fixture = mixed();
        let all = touching(&fixture, "a", &[], Direction::Both);
        assert_eq!(all.iter().filter(|l| l.starts_with("a>a")).count(), 1);
    }

    #[test]
    fn relationships_of_an_unconnected_artifact_is_empty_and_an_unknown_one_is_not_found() {
        let fixture = RepoFixture::new()
            .artifact("a")
            .artifact("lonely")
            .relation("a", "references", "a");
        assert_eq!(
            touching(&fixture, "lonely", &[], Direction::Both),
            Vec::<String>::new()
        );
        let error = relationships_of(&fixture.build(), &reference("nope"), &[], Direction::Both)
            .unwrap_err();
        assert_eq!(error.kind(), ErrorKind::NotFound);
    }
}
