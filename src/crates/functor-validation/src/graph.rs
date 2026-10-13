//! Strongly connected components, for cycle detection (`V0011`).

/// The strongly connected components of a directed graph with `count` nodes, found with an
/// iterative form of Tarjan's algorithm (no recursion, so long chains cannot overflow the stack).
///
/// `adjacency[n]` lists the successors of node `n`. Components are returned in the order they
/// complete; the nodes inside one component are in no particular order.
pub(crate) fn components(count: usize, adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    const UNVISITED: usize = usize::MAX;
    let mut index = vec![UNVISITED; count];
    let mut low = vec![0_usize; count];
    let mut on_stack = vec![false; count];
    let mut stack: Vec<usize> = Vec::new();
    let mut next_index = 0_usize;
    let mut found = Vec::new();

    for root in 0..count {
        if index[root] != UNVISITED {
            continue;
        }
        index[root] = next_index;
        low[root] = next_index;
        next_index += 1;
        stack.push(root);
        on_stack[root] = true;
        // Each entry is a node and the position of its next successor to examine.
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];

        while let Some(&(node, child)) = work.last() {
            if let Some(&next) = adjacency[node].get(child) {
                if let Some(top) = work.last_mut() {
                    top.1 += 1;
                }
                if index[next] == UNVISITED {
                    index[next] = next_index;
                    low[next] = next_index;
                    next_index += 1;
                    stack.push(next);
                    on_stack[next] = true;
                    work.push((next, 0));
                } else if on_stack[next] {
                    low[node] = low[node].min(index[next]);
                }
            } else {
                work.pop();
                if let Some(&(parent, _)) = work.last() {
                    low[parent] = low[parent].min(low[node]);
                }
                if low[node] == index[node] {
                    let mut component = Vec::new();
                    while let Some(member) = stack.pop() {
                        on_stack[member] = false;
                        component.push(member);
                        if member == node {
                            break;
                        }
                    }
                    found.push(component);
                }
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    fn sorted(mut components: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
        for component in &mut components {
            component.sort_unstable();
        }
        components.sort();
        components
    }

    #[test]
    fn an_empty_graph_has_no_components() {
        assert_eq!(components(0, &[]), Vec::<Vec<usize>>::new());
    }

    #[test]
    fn a_chain_has_only_singleton_components() {
        let adjacency = vec![vec![1], vec![2], vec![]];
        assert_eq!(sorted(components(3, &adjacency)), [[0], [1], [2]]);
    }

    #[test]
    fn a_two_node_cycle_is_one_component() {
        let adjacency = vec![vec![1], vec![0], vec![]];
        assert_eq!(sorted(components(3, &adjacency)), [vec![0, 1], vec![2]]);
    }

    #[test]
    fn two_disjoint_cycles_are_two_components() {
        let adjacency = vec![vec![1], vec![0], vec![3], vec![4], vec![2]];
        assert_eq!(
            sorted(components(5, &adjacency)),
            [vec![0, 1], vec![2, 3, 4]]
        );
    }

    #[test]
    fn a_cycle_with_a_tail_and_a_head_keeps_them_out() {
        // 0 -> 1 -> 2 -> 3 -> 1, 3 -> 4
        let adjacency = vec![vec![1], vec![2], vec![3], vec![1, 4], vec![]];
        assert_eq!(
            sorted(components(5, &adjacency)),
            [vec![0], vec![1, 2, 3], vec![4]]
        );
    }

    #[test]
    fn a_self_loop_is_a_singleton_component() {
        assert_eq!(sorted(components(1, &[vec![0]])), [[0]]);
    }

    #[test]
    fn a_very_long_ring_does_not_overflow_the_stack() {
        let count = 200_000;
        let adjacency: Vec<Vec<usize>> = (0..count).map(|n| vec![(n + 1) % count]).collect();
        let found = components(count, &adjacency);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].len(), count);
    }

    fn reachable(adjacency: &[Vec<usize>], from: usize) -> Vec<bool> {
        let mut seen = vec![false; adjacency.len()];
        let mut todo = vec![from];
        while let Some(node) = todo.pop() {
            if std::mem::replace(&mut seen[node], true) {
                continue;
            }
            todo.extend(&adjacency[node]);
        }
        seen
    }

    proptest! {
        /// Two nodes share a component exactly when each reaches the other.
        #[test]
        fn components_are_the_mutually_reachable_classes(
            edges in prop::collection::vec((0_usize..8, 0_usize..8), 0..30),
        ) {
            let count = 8;
            let mut adjacency = vec![Vec::new(); count];
            for (from, to) in edges {
                adjacency[from].push(to);
            }
            let found = components(count, &adjacency);

            let mut owner = vec![usize::MAX; count];
            for (id, component) in found.iter().enumerate() {
                for &node in component {
                    prop_assert_eq!(owner[node], usize::MAX, "node in two components");
                    owner[node] = id;
                }
            }
            prop_assert!(owner.iter().all(|&o| o != usize::MAX), "every node is in a component");

            let reach: Vec<Vec<bool>> = (0..count).map(|n| reachable(&adjacency, n)).collect();
            for a in 0..count {
                for b in 0..count {
                    let mutual = reach[a][b] && reach[b][a];
                    prop_assert_eq!(owner[a] == owner[b], mutual, "{} and {}", a, b);
                }
            }
        }
    }
}
