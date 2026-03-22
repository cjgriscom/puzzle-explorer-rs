//! Canonization of puzzles
//!
//! This module generates dreadnaut scripts to canonize orbit graphs
//! as well as complete puzzles.

#[macro_use]
mod dreadnaut_script;

use std::collections::HashMap;

use crate::generator::Generator;

/// Orbit canonization using Dreadnaut
///
/// An orbit is canonized by constructing a graph containing a vertex for each piece,
/// and a midpoint vertex representing the path of the piece on each rotation axis.
/// Edges connect to the midpoint based on which direction the cycle moves.
/// Both directions are considered to ensure symmetry, so in operations containing
/// cycles, each gets two midpoints - one for each direction.
///
/// Two-cycles are represented by one midpoint with both edges pointing inward since
/// their inverse operations are indistinguishable.
///
/// Isomorphic operations are filtered out before canonization.
///
/// Each operation is assigned a label and is connected to each of its midpoints to
/// regidly enforce cycle to axis groupings.
#[derive(Debug, Clone, Default)]
pub struct OrbitCanonizer {
    // Inputs
    generator_original: Vec<Vec<Vec<usize>>>,

    // Script generation
    generator_renumbered_bidirectional: Vec<Vec<Vec<usize>>>,
    n_vertices: usize,

    // Result processing
    canonical_labels: Vec<usize>,
    canonical_graph: HashMap<usize, Vec<usize>>,
}

impl OrbitCanonizer {
    pub fn new(generator: &[Vec<Vec<usize>>]) -> Self {
        OrbitCanonizer {
            generator_original: generator.to_vec(),
            ..Default::default()
        }
    }

    fn sub_generate_input_graph(&self) -> (HashMap<usize, Vec<usize>>, usize) {
        let mut n_midpoints = 0;
        let mut midpoint_partitions = Vec::new();
        let mut edges: HashMap<usize, Vec<usize>> = HashMap::new();
        for generator in &self.generator_renumbered_bidirectional {
            let mut partition = Vec::new();
            for cycle in generator {
                if cycle.len() == 2 {
                    // Swaps are directionless, so we can represent them with a single midpoint
                    // Point the two vertices inward to new midpoint
                    let mid = n_midpoints + self.n_vertices;
                    partition.push(mid);
                    n_midpoints += 1;
                    let a = cycle[0];
                    let b = cycle[1];
                    edges.entry(a).or_default().push(mid);
                    edges.entry(b).or_default().push(mid);
                } else if cycle.len() >= 2 {
                    for i in 0..cycle.len() {
                        // Point each member of the of the cycle toward the next, separated by mid
                        let mid = n_midpoints + self.n_vertices;
                        partition.push(mid);
                        n_midpoints += 1;
                        let a = cycle[i];
                        let b = cycle[(i + 1) % cycle.len()];
                        edges.entry(a).or_default().push(mid);
                        edges.entry(mid).or_default().push(b);
                    }
                }
            }
            midpoint_partitions.push(partition);
        }

        // Add generator vertices to ensure cycles remain linked to their original axes
        for (g_idx, partition) in midpoint_partitions.iter().enumerate() {
            let gen_vertex = self.n_vertices + n_midpoints + g_idx;
            for &mid in partition {
                edges.entry(gen_vertex).or_default().push(mid);
            }
        }

        (edges, n_midpoints)
    }

    /// Generate the script to send to dreadnaut
    pub fn generate_script(&mut self) -> Result<String, String> {
        let (generator_renumbered, n_vertices) = self.generator_original.renumber(0);
        self.generator_renumbered_bidirectional = generator_renumbered
            .add_inverse_operations() // add copies of each operation with cycles inverted to ensure symmetry
            .remove_isomorphic_operations(); // redundant operations will mess up the canonization
        self.n_vertices = n_vertices;
        let (edges, n_midpoints) = self.sub_generate_input_graph();

        let n_generators = self.generator_renumbered_bidirectional.len();
        let n_total = n_midpoints + self.n_vertices + n_generators;
        let mut script = String::new();
        script.push_str(&dn_linelength!(0));
        script.push_str(dn_set_level_markers!(true));
        script.push_str(dn_sparse_mode!());
        script.push_str(dn_digraph!());
        script.push_str(&dn_begin_graph!(n_total));

        for v in 0..n_total {
            script.push_str(&format!("{}:", v));
            if let Some(neighbors) = edges.get(&v) {
                for &u in neighbors {
                    script.push_str(&format!(" {}", u));
                }
            }
            if v == n_total - 1 {
                script.push_str(".\n");
            } else {
                script.push_str(";\n");
            }
        }

        script.push_str(dn_set_canonical_labeling!(true));
        script.push_str(dn_set_automorphism_output!(false));
        script.push_str(dn_execute!());
        script.push_str(&dn_print_literal!(&"LABEL:\n"));
        script.push_str(dn_canonical_label!());
        Ok(script)
    }

    /// Parse dreadnaut output
    pub fn process_script_result(&mut self, s: &str) -> Result<(), String> {
        let after_label = s.find("LABEL:").ok_or("LABEL: not found in output")? + "LABEL:".len();

        let rest = &s[after_label..];
        let mut label = Vec::new();
        let mut edges: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut found_label = false;

        for line in rest.split('\n') {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if !found_label {
                if line.contains(':') {
                    continue;
                }
                let nums: Vec<usize> = line
                    .split_whitespace()
                    .filter_map(|w| w.parse().ok())
                    .collect();
                if !nums.is_empty() {
                    label = nums;
                    found_label = true;
                }
                continue;
            }

            if let Some(colon_pos) = line.find(':') {
                let lhs = line[..colon_pos].trim();
                let rhs = line[colon_pos + 1..].trim().trim_end_matches(';');
                if let Ok(vertex) = lhs.parse::<usize>() {
                    let neighbors: Vec<usize> = rhs
                        .split_whitespace()
                        .filter_map(|w| w.parse().ok())
                        .collect();
                    edges.insert(vertex, neighbors);
                }
            }
        }

        if label.is_empty() {
            return Err("could not parse canonical label".to_string());
        }

        self.canonical_labels = label;
        self.canonical_graph = edges;

        Ok(())
    }

    pub fn get_relabeling(&self) -> Vec<usize> {
        self.canonical_labels.clone()
    }

    pub fn get_canonical_graph(&self) -> HashMap<usize, Vec<usize>> {
        self.canonical_graph.clone()
    }

    pub fn get_canonical_graph_as_string(&self) -> String {
        let mut s = String::new();

        /*
        // DEBUG print original graph
        s.push_str("ORIGINAL GRAPH:\n");
        let (graph, _, _) = self.sub_generate_input_graph();
        let mut keys = graph.keys().cloned().collect::<Vec<_>>();
        keys.sort_unstable();
        for key in keys {
            s.push_str(&key.to_string());
            s.push(':');
            let mut neighbors = graph[&key].clone();
            neighbors.sort_unstable();
            for (i, n) in neighbors.iter().enumerate() {
                if i > 0 {
                    s.push(' ');
                }
                s.push_str(&n.to_string());
            }
            s.push('\n');
        }

        // DEBUG print midpoint partitions
        s.push_str("MIDPOINT PARTITIONS:\n");
        for partition in &self.midpoint_partitions {
            for (i, &midpoint) in partition.iter().enumerate() {
                if i > 0 {
                    s.push(' ');
                }
                s.push_str(&midpoint.to_string());
            }
            s.push('\n');
        }
        s.push('\n');*/

        // print canonical graph
        let mut keys = self.canonical_graph.keys().cloned().collect::<Vec<_>>();
        keys.sort_unstable();
        for key in keys {
            s.push_str(&key.to_string());
            s.push(':');
            let mut neighbors = self.canonical_graph[&key].clone();
            neighbors.sort_unstable();
            for (i, n) in neighbors.iter().enumerate() {
                if i > 0 {
                    s.push(' ');
                }
                s.push_str(&n.to_string());
            }
            s.push('\n');
        }
        s
    }

    pub fn get_hash(&self) -> String {
        create_hash("o1", &self.get_canonical_graph_as_string())
    }
}

/// Produce a deterministic hash of the canonically labelled graph:
/// `[(format_code) (8 hex) (8 hex) (8 hex)]`
fn create_hash(format_code: &str, input: &str) -> String {
    let hash = blake3::hash(input.as_bytes());
    let h = hash.as_slice();
    format!(
        "[{} {:08x} {:08x} {:08x}]",
        format_code,
        u32::from_be_bytes([h[0], h[1], h[2], h[3]]),
        u32::from_be_bytes([h[4], h[5], h[6], h[7]]),
        u32::from_be_bytes([h[8], h[9], h[10], h[11]]),
    )
}

// Quick tests with dreadnaut commandline
// They will not run on the build server
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator;
    use std::io::Write;
    use std::process::{Command, Stdio};

    fn canonize(gap: &str) -> String {
        let g = generator::parse_gap_string(gap).unwrap();
        let mut c = OrbitCanonizer::new(&g);
        let mut script = c.generate_script().unwrap();
        script.push_str("q\n");
        let mut child = Command::new("dreadnaut")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("dreadnaut not found on PATH");
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(script.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        let s = String::from_utf8_lossy(&out.stdout);
        c.process_script_result(&s).unwrap();
        c.get_canonical_graph_as_string()
    }

    #[test]
    fn test_magic_cogra_flip_vertex_order() {
        let g1 = "[(1,2,5,4)(8,9,12,11),(3,4,8,7)(5,6,10,9),(4,5,9,8)]";
        let g2 = "[(1,2,5,4)(8,9,12,11),(3,4,8,7)(5,6,10,9),(8,9,5,4)]";
        assert_eq!(
            canonize(g1),
            canonize(g2),
            "canon string must not depend on cycle rotation when graphs are isomorphic"
        );
    }
}
