use crate::math::{PI, TAU, norm_ang};
use crate::circle::{Circle, Arc};
use glam::DVec3;
use std::collections::HashSet;

// --- Polygon / Face Detection ---

pub struct Face {
    pub center: DVec3,
}

struct GraphEdge {
    to: usize,
    vec_dir: DVec3,
    pair_id: usize,
    angle: f64,
    arc_idx: usize,
}

struct GraphNode {
    pos: DVec3,
    edges: Vec<GraphEdge>,
}

fn find_or_create_node(nodes: &mut Vec<GraphNode>, v: DVec3) -> usize {
    for (i, n) in nodes.iter().enumerate() {
        if n.pos.distance(v) < 1e-4 {
            return i;
        }
    }
    nodes.push(GraphNode {
        pos: v,
        edges: Vec::new(),
    });
    nodes.len() - 1
}

pub fn get_poly_centroids(circles: &[Circle], arcs: &[Arc]) -> Result<Vec<Face>, String> {
    // Step 1: Find intersection cuts for each arc
    let mut cuts: Vec<Vec<f64>> = arcs.iter().map(|a| vec![0.0, a.l]).collect();

    for i in 0..arcs.len() {
        for j in (i + 1)..arcs.len() {
            let c1 = &circles[arcs[i].circ_idx];
            let c2 = &circles[arcs[j].circ_idx];
            let pts = c1.intersect(c2);
            for p in &pts {
                let ang1 = c1.pt_ang(*p);
                let da1 = norm_ang(ang1 - arcs[i].s);
                if da1 <= arcs[i].l + 1e-5 {
                    cuts[i].push(da1);
                }
                let ang2 = c2.pt_ang(*p);
                let da2 = norm_ang(ang2 - arcs[j].s);
                if da2 <= arcs[j].l + 1e-5 {
                    cuts[j].push(da2);
                }
            }
        }
    }

    // Step 2: Build graph - edges between consecutive intersection nodes along each arc
    let mut nodes: Vec<GraphNode> = Vec::new();
    let mut edge_pair_id = 0usize;

    for i in 0..arcs.len() {
        let c = &circles[arcs[i].circ_idx];
        cuts[i].sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut unique = vec![cuts[i][0]];
        for k in 1..cuts[i].len() {
            if cuts[i][k] > *unique.last().unwrap() + 1e-5 {
                unique.push(cuts[i][k]);
            }
        }

        for k in 0..unique.len().saturating_sub(1) {
            let s = unique[k];
            let e = unique[k + 1];
            let p1 = c.circ_pt(arcs[i].s + s);
            let p2 = c.circ_pt(arcs[i].s + e);
            let idx1 = find_or_create_node(&mut nodes, p1);
            let idx2 = find_or_create_node(&mut nodes, p2);
            if idx1 == idx2 {
                continue;
            }

            let ang_s = arcs[i].s + s;
            let ang_e = arcs[i].s + e;
            let tan_s = (c.w * ang_s.cos() - c.u * ang_s.sin()).normalize();
            let tan_e = -(c.w * ang_e.cos() - c.u * ang_e.sin()).normalize();

            let pid = edge_pair_id;
            edge_pair_id += 1;
            nodes[idx1].edges.push(GraphEdge {
                to: idx2,
                vec_dir: tan_s,
                pair_id: pid,
                angle: 0.0,
                arc_idx: i,
            });
            nodes[idx2].edges.push(GraphEdge {
                to: idx1,
                vec_dir: tan_e,
                pair_id: pid,
                angle: 0.0,
                arc_idx: i,
            });
        }
    }

    // Step 2b: Remove degree-2 nodes (pass-through nodes that don't represent
    // real intersections). These arise when an intersection point lies on one
    // arc but the edges on the other arc collapse due to tolerance.
    // Note: this is probably no longer needed due to integral formula in step 4
    loop {
        let mut merged_any = false;
        for ni in 0..nodes.len() {
            if nodes[ni].edges.len() != 2 {
                continue;
            }
            // This node has exactly 2 edges; merge them
            let e0_to = nodes[ni].edges[0].to;
            let e0_pid = nodes[ni].edges[0].pair_id;
            let e1_to = nodes[ni].edges[1].to;
            let e1_pid = nodes[ni].edges[1].pair_id;

            if e0_to == e1_to {
                // Both edges go to the same node - just remove both
                nodes[ni].edges.clear();
                nodes[e0_to]
                    .edges
                    .retain(|e| e.pair_id != e0_pid && e.pair_id != e1_pid);
                edge_pair_id -= 2;
                merged_any = true;
                continue;
            }

            // Propagate arc_idx for finding midpoint later
            let arc_idx = nodes[ni].edges[0].arc_idx;

            // Find the back-edges at the two neighbor nodes and record their vec_dir
            let a_vec = nodes[e0_to]
                .edges
                .iter()
                .find(|e| e.pair_id == e0_pid)
                .map(|e| e.vec_dir);
            let b_vec = nodes[e1_to]
                .edges
                .iter()
                .find(|e| e.pair_id == e1_pid)
                .map(|e| e.vec_dir);

            if a_vec.is_none() || b_vec.is_none() {
                continue;
            }

            // Remove old edges from neighbors
            nodes[e0_to].edges.retain(|e| e.pair_id != e0_pid);
            nodes[e1_to].edges.retain(|e| e.pair_id != e1_pid);

            // Clear the degree-2 node
            nodes[ni].edges.clear();

            // Add new merged edge between the two neighbors
            // The tangent directions at each neighbor are preserved from the
            // original edges pointing toward ni (which is the same direction
            // as pointing along the arc through ni toward the other neighbor).
            let new_pid = edge_pair_id;
            edge_pair_id += 1;
            nodes[e0_to].edges.push(GraphEdge {
                to: e1_to,
                vec_dir: a_vec.unwrap(),
                pair_id: new_pid,
                angle: 0.0,
                arc_idx,
            });
            nodes[e1_to].edges.push(GraphEdge {
                to: e0_to,
                vec_dir: b_vec.unwrap(),
                pair_id: new_pid,
                angle: 0.0,
                arc_idx,
            });

            // Net: removed 2 edges (e0_pid, e1_pid), added 1 (new_pid)
            // edge_pair_id was already incremented; adjust for the 2 removed
            // (pair_id is just an ID, not a count, so we don't need to adjust)

            merged_any = true;
        }
        if !merged_any {
            break;
        }
    }

    // Recompute edge_pair_id for Euler formula check
    let mut actual_edges = 0;
    for node in &nodes {
        actual_edges += node.edges.len();
    }
    edge_pair_id = actual_edges / 2;

    // Step 3: Sort edges at each node by angle around the sphere normal
    for node in &mut nodes {
        if node.edges.is_empty() {
            continue;
        }
        let normal = node.pos.normalize();
        let ref_vec = node.edges[0].vec_dir;
        let ref_perp = normal.cross(ref_vec);
        for edge in &mut node.edges {
            edge.angle = edge.vec_dir.dot(ref_perp).atan2(edge.vec_dir.dot(ref_vec));
        }
        node.edges
            .sort_by(|a, b| a.angle.partial_cmp(&b.angle).unwrap());
    }

    // Step 4: Walk faces by following "next edge" (turn right) at each node
    let mut faces: Vec<Face> = Vec::new();
    let mut visited: HashSet<(usize, usize)> = HashSet::new();
    let mut skipped_faces = 0;

    for i in 0..nodes.len() {
        for j in 0..nodes[i].edges.len() {
            if visited.contains(&(i, j)) {
                continue;
            }

            let mut path = Vec::new();
            let mut curr = i;
            let mut curr_edge_idx = j;
            let mut fail = false;
            let mut steps = 0;

            loop {
                visited.insert((curr, curr_edge_idx));
                path.push((curr, curr_edge_idx));

                let to = nodes[curr].edges[curr_edge_idx].to;
                let pid = nodes[curr].edges[curr_edge_idx].pair_id;

                let back_idx = nodes[to].edges.iter().position(|e| e.pair_id == pid);
                if back_idx.is_none() {
                    fail = true;
                    break;
                }
                let back_idx = back_idx.unwrap();

                let out_idx = (back_idx + 1) % nodes[to].edges.len();
                curr = to;
                curr_edge_idx = out_idx;
                steps += 1;

                if (curr == i && curr_edge_idx == j) || steps >= 200 {
                    break;
                }
            }

            let old_method = false;

            if !fail && curr == i && curr_edge_idx == j && path.len() >= 2 {
                let mut perimeter = 0.0;
                let mut sum = DVec3::ZERO;

                for k in 0..path.len() {
                    let p1 = nodes[path[k].0].pos;
                    let p2 = nodes[path[(k + 1) % path.len()].0].pos;

                    // Rough perimeter estimate for eliminating tiny faces
                    perimeter += p1.distance(p2);

                    if old_method {
                        sum += (p1 + p2).normalize() * p1.angle_between(p2);
                    } else {
                        let edge = &nodes[path[k].0].edges[path[k].1];
                        let arc_idx = edge.arc_idx;
                        let arc = &arcs[arc_idx];
                        let c = &circles[arc.circ_idx];

                        // Get circle angles of p1 and p2
                        let ang1 = c.pt_ang(p1);
                        let ang2 = c.pt_ang(p2);
                        let mut da1 = norm_ang(ang1 - arc.s);
                        let mut da2 = norm_ang(ang2 - arc.s);

                        // Ensure shorter path between angles
                        if (da2 - da1).abs() > PI {
                            if da1 < da2 {
                                da1 += TAU;
                            } else {
                                da2 += TAU;
                            }
                        }

                        // Integral of arc between p1 and p2
                        let v = c.arc_integral(arc, da1, da2);
                        let angle = norm_ang(da2 - da1).min(norm_ang(da1 - da2));
                        sum += v * angle;
                    }
                }

                // Filter out tiny faces
                if perimeter > 0.02 {
                    // Check for near-duplicates
                    // TODO figure out why this happens in the first place
                    //      4, 4, 1/4, 62°
                    let mut has_duplicate = false;
                    for f in &faces {
                        if f.center.distance(sum.normalize()) < 1e-4 {
                            has_duplicate = true;
                            break;
                        }
                    }
                    if !has_duplicate {
                        faces.push(Face {
                            center: sum.normalize(),
                        });
                    } else {
                        skipped_faces += 1;
                    }
                } else {
                    skipped_faces += 1;
                }
            }
        }
    }

    let v = nodes.iter().filter(|n| !n.edges.is_empty()).count();
    let e = edge_pair_id;
    let f = faces.len() + skipped_faces;
    if v == 0 && f == 0 && e == 0 {
        return Err("Polygon detection failed - no intersections found".to_string());
    } else if v + f != e + 2 {
        return Err(format!(
            "Polygon detection failed - Euler's formula mismatch: V={} E={} F={} (expected V-E+F=2)",
            v, e, f
        ));
    }

    Ok(faces)
}

#[cfg(test)]
mod tests {
    use super::*;
	use crate::geometry::{derive_axis_angle, compute_arcs, merge_arcs};

    fn get_poly_centroids_for(
        n_a: u32,
        n_b: u32,
        p: u32,
        q: u32,
        colat_a: f64,
        colat_b: f64,
    ) -> Result<Vec<Face>, String> {
        let axis_angle = derive_axis_angle(n_a, n_b, p, q).expect("Failed to derive axis angle");
        let (circles, arcs) = compute_arcs(axis_angle, colat_a, colat_b, n_a, n_b);
        let merged_arcs = merge_arcs(&arcs);

        /*println!(
            "Circles: {}, Arcs: {}, Merged Arcs: {}",
            circles.len(),
            arcs.len(),
            merged_arcs.len()
        );*/

        get_poly_centroids(&circles, &merged_arcs)
    }

    #[test]
    fn test_poly_centroids_case_0() {
        match get_poly_centroids_for(3, 2, 1, 4, 120.0f64.to_radians(), 120.0f64.to_radians()) {
            Ok(_faces) => {}
            Err(e) => {
                panic!("{}", e);
                // Perfect boundary case where one piece disappears and another appears
                // must merge points and ensure edges are connected
            }
        }
    }

    #[test]
    fn test_poly_centroids_case_1() {
        // Bugged case 03/01/2026 fixed with step 2b & integral formula

        // Result A has 58 pieces
        let result_a =
            get_poly_centroids_for(3, 2, 1, 3, 125.1f64.to_radians(), 125.1f64.to_radians());

        // Result B should have faces, pseudo-subset of result A
        //   there was a bug where one face coordinate was off significantly
        let result_b =
            get_poly_centroids_for(3, 2, 1, 3, 125.3f64.to_radians(), 125.3f64.to_radians());

        match (result_a, result_b) {
            (Ok(faces_a), Ok(faces_b)) => {
                println!("Found {} faces in A", faces_a.len());

                println!("Found {} faces in B", faces_b.len());

                let mut matches = Vec::new();

                // Loop thru B faces and find a nearly matching point in A
                for (j, face_b) in faces_b.iter().enumerate() {
                    let mut best_idx = None;
                    let mut best_d = f64::INFINITY;
                    for (i, face_a) in faces_a.iter().enumerate() {
                        let d = face_a.center.distance(face_b.center);
                        if d < best_d {
                            best_d = d;
                            best_idx = Some(i);
                        }
                    }
                    if let Some(idx) = best_idx {
                        // 4 points don't exist, filter them out
                        if best_d < 0.25 {
                            matches.push((j + 1, idx + 1, best_d));
                        }
                    }
                }

                matches.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
                for (j, idx, d) in matches {
                    // Assert d < 0.01
                    assert!(
                        d < 0.01,
                        "Face {} in B does not match face {} in A with D={}",
                        j,
                        idx,
                        d
                    );
                }
            }
            (Err(e), _) | (_, Err(e)) => {
                panic!("Polygon detection failed: {}", e);
            }
        }
    }

}
