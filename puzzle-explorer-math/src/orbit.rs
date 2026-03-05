use crate::circle::{Arc, Circle};
use crate::geometry::rotate_v;
use crate::math::TAU;
use crate::polygon::{PolygonOptions, get_poly_centroids};
use glam::DVec3;
use std::collections::HashSet;

// --- Orbit Analysis ---

pub struct OrbitAnalysis {
    pub degenerate_faces: HashSet<usize>,
    pub face_positions: Vec<DVec3>,
    pub orbits: Vec<Vec<usize>>,
    pub generators: Vec<Vec<Vec<Vec<usize>>>>,
}

/// Each axis entry is (direction_unit_vec, colat_radians, rotational_symmetry_n).
pub struct OrbitAnalysisInput<'a> {
    pub circles: &'a [Circle],
    pub arcs: &'a [Arc],
    pub axes: &'a [(DVec3, f64, u32)],
    pub options: PolygonOptions,
}

pub fn compute_orbit_analysis(input: OrbitAnalysisInput<'_>) -> Result<OrbitAnalysis, String> {
    let OrbitAnalysisInput {
        circles,
        arcs,
        axes,
        options,
    } = input;
    let faces = get_poly_centroids(circles, arcs, options)?;
    let n_faces = faces.len();
    let n_axes = axes.len();

    let fudged_mode = matches!(options, PolygonOptions::FudgedMode { .. });

    if n_faces == 0 {
        return Ok(OrbitAnalysis {
            degenerate_faces: HashSet::new(),
            face_positions: vec![],
            orbits: vec![],
            generators: vec![],
        });
    }

    // Build moves: for each axis, a forward and inverse rotation
    struct Move {
        axis_idx: usize,
        is_forward: bool,
        axis: DVec3,
        angle: f64,
        colat: f64,
    }
    let mut moves = Vec::new();
    for (i, &(axis, colat, n)) in axes.iter().enumerate() {
        moves.push(Move {
            axis_idx: i,
            is_forward: true,
            axis,
            angle: TAU / n as f64,
            colat,
        });
        moves.push(Move {
            axis_idx: i,
            is_forward: false,
            axis,
            angle: -TAU / n as f64,
            colat,
        });
    }

    let base_pos: Vec<DVec3> = faces.iter().map(|f| f.center).collect();

    let find_match = |p_rot: DVec3| -> Option<usize> {
        let mut best_d = f64::MAX;
        let mut best_idx = None;
        for (i, bp) in base_pos.iter().enumerate() {
            let d = p_rot.distance(*bp);
            if d < best_d {
                best_d = d;
                best_idx = Some(i);
            }
        }
        if best_d < 0.4 { best_idx } else { None }
    };

    let mut adj: Vec<Vec<usize>> = vec![vec![]; n_faces];
    // One permutation per axis (forward rotation only)
    let mut perms: Vec<Vec<usize>> = (0..n_axes).map(|_| (0..n_faces).collect()).collect();

    for m in &moves {
        let cos_colat = m.colat.cos();
        for i in 0..n_faces {
            let p0 = base_pos[i];
            let dot = p0.normalize().dot(m.axis);
            let p_rot = if dot > cos_colat + 1e-4 {
                rotate_v(p0, m.axis, m.angle)
            } else {
                p0
            };
            if let Some(idx) = find_match(p_rot) {
                if !adj[i].contains(&idx) {
                    adj[i].push(idx);
                }
                if !adj[idx].contains(&i) {
                    adj[idx].push(i);
                }
                if m.is_forward {
                    perms[m.axis_idx][i] = idx;
                }
            }
        }
    }

    // BFS connected components
    let mut visited = vec![false; n_faces];
    let mut orbits: Vec<Vec<usize>> = Vec::new();
    for i in 0..n_faces {
        if visited[i] {
            continue;
        }
        let mut queue = vec![i];
        visited[i] = true;
        let mut members = Vec::new();
        while let Some(u) = queue.pop() {
            members.push(u);
            for &v in &adj[u] {
                if !visited[v] {
                    visited[v] = true;
                    queue.push(v);
                }
            }
        }
        members.sort();
        orbits.push(members);
    }

    let perm_to_0_indexed_cycles = |perm: &[usize], subset: &[usize]| -> Vec<Vec<usize>> {
        let mut in_set = std::collections::HashMap::new();
        for (i, &v) in subset.iter().enumerate() {
            in_set.insert(v, i);
        }
        let mut seen = HashSet::new();
        let mut cycles = Vec::new();
        for &start in subset {
            if seen.contains(&start) {
                continue;
            }
            let mut cycle = Vec::new();
            let mut cur = start;
            while !seen.contains(&cur) && in_set.contains_key(&cur) {
                seen.insert(cur);
                cycle.push(in_set[&cur]);
                cur = perm[cur];
            }
            if cycle.len() > 1 {
                cycles.push(cycle);
            }
        }
        cycles
    };

    let mut generators = Vec::new();
    let mut orbits_final = Vec::new();
    let mut degenerate_faces = HashSet::new();

    for members in orbits.iter() {
        if members.len() == 1 {
            generators.push(vec![]);
            orbits_final.push(members.clone());
        } else {
            // Build cycle generators for each axis
            let axis_gens: Vec<Vec<Vec<usize>>> = (0..n_axes)
                .map(|ai| perm_to_0_indexed_cycles(&perms[ai], members))
                .collect();

            // Check cycle length mismatches
            let mut any_mismatch = false;
            for (ai, axis_gen) in axis_gens.iter().enumerate() {
                let expected_n = axes[ai].2 as usize;
                let mismatch = axis_gen.iter().any(|c| c.len() != expected_n);
                if mismatch && !fudged_mode {
                    return Err(format!(
                        "Orbit Cycle Length mismatch: expected cycle length of {} for axis {}.",
                        expected_n, ai
                    ));
                }
                if mismatch {
                    any_mismatch = true;
                }
            }

            if fudged_mode && any_mismatch {
                for &m in members {
                    degenerate_faces.insert(m);
                }
                continue;
            }

            // TODO verify this does not negatively impact fudged mode.  this mainly helps ensure orbits don't
            //  accidently merge and send monstrosities to GAP
            /*let int_scale_factor = 100f32; // Cvt to int to sort and set tolerance.
            let mut face_perimeters = Vec::new();
            for &m in members {
                face_perimeters.push((faces[m].perimeter * int_scale_factor) as i32);
            }

            face_perimeters.sort();
            if face_perimeters[0] != face_perimeters[face_perimeters.len() - 1] {
                for &m in members {
                    degenerate_faces.insert(m);
                }
                continue;
            }*/

            if fudged_mode {
                orbits_final.push(members.clone());
                generators.push(axis_gens);
            } else {
                let mut gens_for_orbit = Vec::new();
                for axis_gen in axis_gens {
                    if !axis_gen.is_empty() {
                        gens_for_orbit.push(axis_gen);
                    }
                }
                generators.push(gens_for_orbit);
                orbits_final.push(members.clone());
            }
        }
    }

    Ok(OrbitAnalysis {
        degenerate_faces,
        face_positions: base_pos,
        orbits: orbits_final,
        generators,
    })
}
