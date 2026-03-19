//! Canonization of puzzles
//!
//! This module interfaces with dreadnaut to canonize orbit graphs
//! as well as complete puzzles.

use std::collections::{HashMap, HashSet};

/// Construct a Dreadnaut script to get the canon hash for an orbit graph
pub fn orbit_graph_hash_script(combined_gen: &[Vec<Vec<usize>>], n_vertices: usize) -> String {
    // Ported from DreadnautInterface.java in GroupExplorer

    let mut adj: Vec<std::collections::BTreeSet<usize>> =
        vec![std::collections::BTreeSet::new(); n_vertices];
    for generator in combined_gen {
        for cycle in generator {
            if cycle.len() < 2 {
                continue;
            }
            for i in 0..cycle.len() {
                let u = cycle[i];
                let v = cycle[(i + 1) % cycle.len()];
                adj[u].insert(v);
            }
        }
    }

    let mut script = String::new();
    script.push_str("l=0\n-m\n");
    script.push_str("Ad\nd\n");
    script.push_str(&format!("n={} g\n", n_vertices));
    (0..n_vertices).for_each(|i| {
        script.push_str(&format!("{}:", i));
        let neigh = &adj[i];
        if !neigh.is_empty() {
            for &j in neigh {
                script.push_str(&format!(" {}", j));
            }
        }
        if i == n_vertices - 1 {
            script.push_str(".\n");
        } else {
            script.push_str(";\n");
        }
    });
    script.push_str("c -a\nx\nz\n");
    script
}

/// Renumber a generator to 0-indexed with no missing labels
/// Returns the renumbered generator and the number of vertices
pub fn renumber_generator_for_dreadnaut(
    gen_raw: &[Vec<Vec<usize>>],
) -> (Vec<Vec<Vec<usize>>>, usize) {
    let mut seen = HashSet::new();
    let mut labels = Vec::new();
    for generator in gen_raw {
        for cycle in generator {
            for &vertex in cycle {
                if seen.insert(vertex) {
                    labels.push(vertex);
                }
            }
        }
    }

    let label_to_index: HashMap<usize, usize> = labels
        .iter()
        .copied()
        .enumerate()
        .map(|(index, label)| (label, index))
        .collect();

    let gen_renumbered: Vec<Vec<Vec<usize>>> = gen_raw
        .iter()
        .map(|generator| {
            generator
                .iter()
                .map(|cycle| cycle.iter().map(|vertex| label_to_index[vertex]).collect())
                .collect()
        })
        .collect();

    (gen_renumbered, labels.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn test_renumber_generator_for_dreadnaut() {
        let gen_raw = vec![vec![vec![1, 9, 3], vec![4, 3, 6]]];
        let (gen_renumbered, num_vertices) = renumber_generator_for_dreadnaut(&gen_raw);
        assert_eq!(gen_renumbered, vec![vec![vec![0, 1, 2], vec![3, 2, 4]]]);
        assert_eq!(num_vertices, 5);
    }
}
