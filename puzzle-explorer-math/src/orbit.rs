use crate::math::TAU;
use crate::geometry::{rotate_v};
use crate::polygon::get_poly_centroids;
use crate::circle::{Circle, Arc};
use glam::DVec3;
use std::collections::HashSet;

// --- Orbit Analysis ---

pub struct OrbitAnalysis {
    pub face_positions: Vec<DVec3>,
    pub orbits: Vec<Vec<usize>>,
    pub generators: Vec<Vec<Vec<Vec<usize>>>>,
}

pub fn compute_orbit_analysis(
    circles: &[Circle],
    arcs: &[Arc],
    n_a: u32,
    n_b: u32,
    axis_angle_rad: f64,
    colat_a: f64,
    colat_b: f64,
) -> Result<OrbitAnalysis, String> {
    let faces = get_poly_centroids(circles, arcs)?;
    let n_faces = faces.len();

    if n_faces == 0 {
        return Ok(OrbitAnalysis {
            face_positions: vec![],
            orbits: vec![],
            generators: vec![],
        });
    }

    let axis_a = DVec3::new(0.0, 0.0, 1.0);
    let axis_b = DVec3::new(axis_angle_rad.sin(), 0.0, axis_angle_rad.cos());

    let base_pos: Vec<DVec3> = faces.iter().map(|f| f.center).collect();

    /* // For ignoring orbits and displaying debug points
    let orbits_all: Vec<Vec<usize>> = (0..n_faces).map(|i| vec![i]).collect();

    if true {
        return Ok(OrbitAnalysis {
            face_positions: base_pos,
            orbits: orbits_all,
            generators: vec![],
        });
    } */

    struct Move {
        name: &'static str,
        axis: DVec3,
        angle: f64,
        colat: f64,
    }
    let moves = [
        Move {
            name: "A",
            axis: axis_a,
            angle: TAU / n_a as f64,
            colat: colat_a,
        },
        Move {
            name: "Ai",
            axis: axis_a,
            angle: -TAU / n_a as f64,
            colat: colat_a,
        },
        Move {
            name: "B",
            axis: axis_b,
            angle: TAU / n_b as f64,
            colat: colat_b,
        },
        Move {
            name: "Bi",
            axis: axis_b,
            angle: -TAU / n_b as f64,
            colat: colat_b,
        },
    ];

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
    let mut perm_a: Vec<usize> = (0..n_faces).collect();
    let mut perm_b: Vec<usize> = (0..n_faces).collect();

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
                if m.name == "A" {
                    perm_a[i] = idx;
                }
                if m.name == "B" {
                    perm_b[i] = idx;
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

    for members in orbits.iter() {
        if members.len() == 1 {
            generators.push(vec![]);
        } else {
            let gen_a = perm_to_0_indexed_cycles(&perm_a, members);
            let gen_b = perm_to_0_indexed_cycles(&perm_b, members);
            let mut gens_for_orbit = Vec::new();
            if !gen_a.is_empty() {
                if gen_a.iter().any(|c| c.len() != n_a as usize) {
                    return Err(format!(
                        "Orbit Cycle Length mismatch: expected cycle length of {} for move A.",
                        n_a
                    ));
                }
                gens_for_orbit.push(gen_a);
            }
            if !gen_b.is_empty() {
                if gen_b.iter().any(|c| c.len() != n_b as usize) {
                    return Err(format!(
                        "Orbit Cycle Length mismatch: expected cycle length of {} for move B.",
                        n_b
                    ));
                }
                gens_for_orbit.push(gen_b);
            }
            generators.push(gens_for_orbit);
        }
    }

    Ok(OrbitAnalysis {
        face_positions: base_pos,
        orbits,
        generators,
    })
}
