//! Property tests of traversal over generated graphs, checked against a brute-force oracle.

// Test support code: panicking on unexpected errors is the point.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use functor_model::{
    Artifact, ArtifactId, ArtifactRef, ArtifactType, Direction, FormatVersion, NewArtifact,
    NewRelationship, Relationship, RelationshipId, RelationshipType, RepositoryId, RepositoryInfo,
    RepositoryName, RepositorySnapshot, Slug, Timestamp, Title, Traversal, TraversalResult,
};
use functor_query::{relationships_of, traverse};
use proptest::prelude::*;

const TYPES: [RelationshipType; 3] = [
    RelationshipType::DependsOn,
    RelationshipType::References,
    RelationshipType::Extends,
];

#[derive(Debug, Clone)]
struct Graph {
    count: usize,
    /// `(source, target, type index)`, unique, with no self-loops on acyclic types.
    edges: Vec<(usize, usize, usize)>,
}

fn graph() -> impl Strategy<Value = Graph> {
    (2_usize..8).prop_flat_map(|count| {
        prop::collection::vec((0..count, 0..count, 0..TYPES.len()), 0..24).prop_map(move |raw| {
            let mut seen = BTreeSet::new();
            let edges = raw
                .into_iter()
                .filter(|&(source, target, ty)| {
                    (source != target || !TYPES[ty].is_acyclic())
                        && seen.insert((source, target, ty))
                })
                .collect();
            Graph { count, edges }
        })
    })
}

fn directions() -> impl Strategy<Value = Direction> {
    prop_oneof![
        Just(Direction::Outbound),
        Just(Direction::Inbound),
        Just(Direction::Both)
    ]
}

/// A subset of the three types as a bit mask; 0 means "all types".
fn type_filter(mask: u8) -> Vec<RelationshipType> {
    (0..TYPES.len())
        .filter(|bit| mask & (1 << bit) != 0)
        .map(|bit| TYPES[bit].clone())
        .collect()
}

fn artifact_id(index: usize) -> ArtifactId {
    // Ids run against slug order so that sorting by slug and by id disagree.
    ArtifactId::from_bits(1000 - index as u128)
}

fn relationship_id(index: usize) -> RelationshipId {
    RelationshipId::from_bits(1 + index as u128)
}

fn build(graph: &Graph) -> RepositorySnapshot {
    let now = Timestamp::parse("2026-10-12T18:00:00Z").unwrap();
    let artifacts: Vec<Artifact> = (0..graph.count)
        .map(|i| {
            let slug = format!("n{i}");
            Artifact::create(
                artifact_id(i),
                NewArtifact::new(
                    Slug::parse(&slug).unwrap(),
                    ArtifactType::document(),
                    Title::new(&slug).unwrap(),
                ),
                now,
            )
        })
        .collect();
    let relationships: Vec<Relationship> = graph
        .edges
        .iter()
        .enumerate()
        .map(|(index, &(source, target, ty))| {
            Relationship::create(
                relationship_id(index),
                NewRelationship::new(TYPES[ty].clone(), artifact_id(source), artifact_id(target)),
                now,
            )
            .unwrap()
        })
        .collect();
    let info = RepositoryInfo::new(
        RepositoryId::from_bits(1),
        RepositoryName::new("Prop").unwrap(),
        None,
        FormatVersion::CURRENT,
        now,
    );
    RepositorySnapshot::new(info, artifacts, relationships).unwrap()
}

fn traversal(
    start: usize,
    types: &[RelationshipType],
    direction: Direction,
    max_depth: Option<u32>,
) -> Traversal {
    let mut traversal = Traversal::new(ArtifactRef::parse(&format!("n{start}")).unwrap())
        .with_types(types.iter().cloned())
        .with_direction(direction);
    traversal.max_depth = max_depth;
    traversal
}

fn slug_index(slug: &str) -> usize {
    slug.strip_prefix('n').unwrap().parse().unwrap()
}

/// The directed steps a traversal may take, as `(from, to, edge index)`.
fn steps(
    graph: &Graph,
    types: &[RelationshipType],
    direction: Direction,
) -> Vec<(usize, usize, usize)> {
    let mut steps = Vec::new();
    for (index, &(source, target, ty)) in graph.edges.iter().enumerate() {
        if !types.is_empty() && !types.contains(&TYPES[ty]) {
            continue;
        }
        if matches!(direction, Direction::Outbound | Direction::Both) {
            steps.push((source, target, index));
        }
        if matches!(direction, Direction::Inbound | Direction::Both) {
            steps.push((target, source, index));
        }
    }
    steps
}

/// Shortest distances from `start` by repeated relaxation (no queue, so independent of the
/// implementation under test).
fn distances(graph: &Graph, steps: &[(usize, usize, usize)], start: usize) -> Vec<Option<u32>> {
    let mut distance: Vec<Option<u32>> = vec![None; graph.count];
    distance[start] = Some(0);
    loop {
        let mut changed = false;
        for &(from, to, _) in steps {
            if let Some(d) = distance[from]
                && distance[to].is_none_or(|current| d + 1 < current)
            {
                distance[to] = Some(d + 1);
                changed = true;
            }
        }
        if !changed {
            return distance;
        }
    }
}

fn node_map(result: &TraversalResult) -> BTreeMap<usize, u32> {
    result
        .nodes
        .iter()
        .map(|n| (slug_index(n.artifact.slug().as_str()), n.depth))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn traversal_matches_a_brute_force_oracle(
        graph in graph(),
        start in 0_usize..8,
        mask in 0_u8..8,
        direction in directions(),
        max_depth in prop::option::of(0_u32..6),
    ) {
        let start = start % graph.count;
        let types = type_filter(mask);
        let snapshot = build(&graph);
        let result = traverse(&snapshot, &traversal(start, &types, direction, max_depth)).unwrap();

        let steps = steps(&graph, &types, direction);
        let distance = distances(&graph, &steps, start);
        let limit = max_depth.unwrap_or(u32::MAX);

        prop_assert_eq!(slug_index(result.start.slug().as_str()), start);
        // Each node is reported once and never the start...
        let reported = node_map(&result);
        prop_assert_eq!(reported.len(), result.nodes.len());
        prop_assert!(!reported.contains_key(&start));
        // ...exactly when it is within the limit, at its minimum depth.
        let expected: BTreeMap<usize, u32> = distance
            .iter()
            .enumerate()
            .filter_map(|(node, d)| d.map(|d| (node, d)))
            .filter(|&(node, d)| node != start && d >= 1 && d <= limit)
            .collect();
        prop_assert_eq!(&reported, &expected);

        // Order: depth, then slug, then id.
        let keys: Vec<(u32, String, ArtifactId)> = result
            .nodes
            .iter()
            .map(|n| (n.depth, n.artifact.slug().to_string(), *n.artifact.id()))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort();
        prop_assert_eq!(keys, sorted);

        // `via` is the lowest-id step into the node from a node one level closer.
        for node in &result.nodes {
            let target = slug_index(node.artifact.slug().as_str());
            let first = steps
                .iter()
                .filter(|&&(from, to, _)| to == target && distance[from] == Some(node.depth - 1))
                .map(|&(_, _, index)| index)
                .min();
            prop_assert_eq!(first.map(relationship_id), Some(*node.via.id()));
        }

        // Edges: unique, ordered by id, and exactly those leaving a node closer than the limit.
        let ids: Vec<RelationshipId> = result.edges.iter().map(|r| *r.id()).collect();
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        sorted_ids.dedup();
        prop_assert_eq!(&ids, &sorted_ids);
        let expected_edges: BTreeSet<RelationshipId> = steps
            .iter()
            .filter(|&&(from, _, _)| distance[from].is_some_and(|d| d < limit))
            .map(|&(_, _, index)| relationship_id(index))
            .collect();
        prop_assert_eq!(ids.into_iter().collect::<BTreeSet<_>>(), expected_edges);
    }

    #[test]
    fn a_deeper_limit_only_adds_nodes(
        graph in graph(),
        start in 0_usize..8,
        mask in 0_u8..8,
        direction in directions(),
        depth in 0_u32..5,
    ) {
        let start = start % graph.count;
        let types = type_filter(mask);
        let snapshot = build(&graph);
        let run = |max_depth| {
            node_map(&traverse(&snapshot, &traversal(start, &types, direction, max_depth)).unwrap())
        };
        let (shallow, deeper, unlimited) = (run(Some(depth)), run(Some(depth + 1)), run(None));
        for (node, d) in &shallow {
            prop_assert_eq!(deeper.get(node), Some(d));
        }
        for (node, d) in &deeper {
            prop_assert_eq!(unlimited.get(node), Some(d));
        }
    }

    #[test]
    fn inbound_of_the_outbound_neighbours_includes_the_start(
        graph in graph(),
        start in 0_usize..8,
        mask in 0_u8..8,
    ) {
        let start = start % graph.count;
        let types = type_filter(mask);
        let snapshot = build(&graph);
        let outbound = traverse(&snapshot, &traversal(start, &types, Direction::Outbound, Some(1))).unwrap();
        for node in &outbound.nodes {
            let neighbour = slug_index(node.artifact.slug().as_str());
            let back = traverse(&snapshot, &traversal(neighbour, &types, Direction::Inbound, Some(1))).unwrap();
            prop_assert!(node_map(&back).contains_key(&start));
        }
    }

    #[test]
    fn reachability_is_symmetric_between_outbound_and_inbound(
        graph in graph(),
        a in 0_usize..8,
        b in 0_usize..8,
        mask in 0_u8..8,
    ) {
        let (a, b) = (a % graph.count, b % graph.count);
        prop_assume!(a != b);
        let types = type_filter(mask);
        let snapshot = build(&graph);
        let forward = node_map(&traverse(&snapshot, &traversal(a, &types, Direction::Outbound, None)).unwrap());
        let backward = node_map(&traverse(&snapshot, &traversal(b, &types, Direction::Inbound, None)).unwrap());
        prop_assert_eq!(forward.get(&b), backward.get(&a));
    }

    #[test]
    fn relationships_of_matches_filtering_the_edge_list(
        graph in graph(),
        artifact in 0_usize..8,
        mask in 0_u8..8,
        direction in directions(),
    ) {
        let artifact = artifact % graph.count;
        let types = type_filter(mask);
        let snapshot = build(&graph);
        let (found, relationships) = relationships_of(
            &snapshot,
            &ArtifactRef::parse(&format!("n{artifact}")).unwrap(),
            &types,
            direction,
        )
        .unwrap();
        prop_assert_eq!(slug_index(found.slug().as_str()), artifact);

        let expected: Vec<RelationshipId> = graph
            .edges
            .iter()
            .enumerate()
            .filter(|&(_, &(source, target, ty))| {
                (types.is_empty() || types.contains(&TYPES[ty]))
                    && match direction {
                        Direction::Outbound => source == artifact,
                        Direction::Inbound => target == artifact,
                        _ => source == artifact || target == artifact,
                    }
            })
            .map(|(index, _)| relationship_id(index))
            .collect();
        let actual: Vec<RelationshipId> = relationships.iter().map(|r| *r.id()).collect();
        prop_assert_eq!(actual, expected);
    }
}
