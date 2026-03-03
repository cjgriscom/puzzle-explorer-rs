use glam::DVec3;
use std::collections::{BTreeMap, HashSet};
use std::f64::consts::PI;

pub const TAU: f64 = 2.0 * PI;
pub const R: f64 = 1.052028; // Radius of sphere
pub const DISP_R: f64 = R * 1.004; // Dist of arcs from sphere
pub const LABEL_R: f64 = R * 1.04; // Dist. of orbit labels from sphere

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub pole: DVec3,
    pub colat: f64,
    pub u: DVec3,
    pub w: DVec3,
}

#[derive(Clone, Copy, Debug)]
pub struct Arc {
    pub circ_idx: usize,
    pub s: f64,
    pub l: f64,
}

pub struct Interval {
    pub s: f64,
    pub l: f64,
}

pub fn norm_ang(a: f64) -> f64 {
    let a = a % TAU;
    if a < 0.0 { a + TAU } else { a }
}

pub fn make_circ(pole: DVec3, colat: f64) -> Circle {
    let arb = if pole.x.abs() < 0.9 {
        DVec3::new(1.0, 0.0, 0.0)
    } else {
        DVec3::new(0.0, 1.0, 0.0)
    };
    let u = pole.cross(arb).normalize();
    let w = pole.cross(u).normalize();
    Circle {
        pole: pole.normalize(),
        colat,
        u,
        w,
    }
}

#[cfg(test)]
pub fn arc_avg(c: &Circle, arc: &Arc, da0: f64, da1: f64, n_samples: usize) -> DVec3 {
    let mut sum = DVec3::ZERO;

    for i in 0..n_samples {
        let t = da0 + (da1 - da0) * (i as f64 + 0.5) / (n_samples as f64);
        let ang = norm_ang(arc.s + t);
        sum += circ_pt(c, ang);
    }
    sum / (n_samples as f64)
}

pub fn arc_integral(c: &Circle, arc: &Arc, a: f64, b: f64) -> DVec3 {
    let a = a + arc.s;
    let b = b + arc.s;
    c.pole * c.colat.cos()
        + c.colat.sin() * (c.w * (a.cos() - b.cos()) + c.u * (b.sin() - a.sin())) / (b - a)
}

pub fn circ_pt(c: &Circle, theta: f64) -> DVec3 {
    let sc = c.colat.sin();
    let cc = c.colat.cos();
    let ct = theta.cos();
    let st = theta.sin();
    (c.u * ct + c.w * st) * sc + c.pole * cc
}

pub fn pt_ang(c: &Circle, p: DVec3) -> f64 {
    let cc = c.colat.cos();
    let d = p - c.pole * cc;
    d.dot(c.w).atan2(d.dot(c.u))
}

pub fn rotate_v(v: DVec3, axis: DVec3, angle: f64) -> DVec3 {
    let c = angle.cos();
    let s = angle.sin();
    let d = v.dot(axis);
    v * c + axis.cross(v) * s + axis * d * (1.0 - c)
}

pub fn cap_range(c: &Circle, axis: DVec3, cap_colat: f64) -> Option<Interval> {
    let sc = c.colat.sin();
    let cc = c.colat.cos();
    let a_val = sc * c.u.dot(axis);
    let b_val = sc * c.w.dot(axis);
    let c_val = cc * c.pole.dot(axis);
    let amp = (a_val * a_val + b_val * b_val).sqrt();
    let cos_cap = cap_colat.cos();
    let eps = 1e-9;

    if c_val + amp < cos_cap - eps {
        return None;
    }
    if c_val - amp >= cos_cap - eps {
        return Some(Interval { s: 0.0, l: TAU });
    }

    let phi = b_val.atan2(a_val);
    let ratio = ((cos_cap - c_val) / amp).clamp(-1.0, 1.0);
    let delta = ratio.acos();
    Some(Interval {
        s: norm_ang(phi - delta),
        l: 2.0 * delta,
    })
}

pub fn isect_iv(a: &Interval, b: &Interval) -> Vec<Interval> {
    fn segs(iv: &Interval) -> Vec<(f64, f64)> {
        let e = iv.s + iv.l;
        if e <= TAU + 1e-10 {
            vec![(iv.s, e.min(TAU))]
        } else {
            vec![(iv.s, TAU), (0.0, e - TAU)]
        }
    }
    let sa = segs(a);
    let sb = segs(b);
    let mut res = Vec::new();
    for (as_start, as_end) in &sa {
        for (bs, be) in &sb {
            let lo = as_start.max(*bs);
            let hi = as_end.min(*be);
            if hi > lo + 1e-10 {
                res.push(Interval { s: lo, l: hi - lo });
            }
        }
    }
    res
}

pub fn subtract_iv(base: &Interval, removals: &[Interval]) -> Vec<Interval> {
    if base.l < 1e-10 {
        return vec![];
    }
    let bl = base.l;
    let mut segs: Vec<(f64, f64)> = Vec::new();

    for r in removals {
        let rs = norm_ang(r.s - base.s);
        let rl = r.l;
        if rs < bl {
            segs.push((rs, (rs + rl).min(bl)));
        }
        if rs + rl > TAU {
            let we = rs + rl - TAU;
            if we > 0.0 {
                segs.push((0.0, we.min(bl)));
            }
        }
        if rs > bl && rs + rl > TAU {
            let we2 = rs + rl - TAU;
            if we2 > 0.0 {
                segs.push((0.0, we2.min(bl)));
            }
        }
    }

    segs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for seg in segs {
        if let Some(last) = merged.last_mut() {
            if seg.0 <= last.1 + 1e-10 {
                last.1 = last.1.max(seg.1);
            } else {
                merged.push(seg);
            }
        } else {
            merged.push(seg);
        }
    }

    let mut res = Vec::new();
    let mut pos = 0.0;
    for (start, end) in merged {
        if start > pos + 1e-10 {
            res.push(Interval {
                s: norm_ang(pos + base.s),
                l: start - pos,
            });
        }
        pos = end;
    }
    if bl > pos + 1e-10 {
        res.push(Interval {
            s: norm_ang(pos + base.s),
            l: bl - pos,
        });
    }
    res
}

pub fn map_arc_to_rotated(
    src_c: &Circle,
    dst_c: &Circle,
    iv: &Interval,
    axis: DVec3,
    angle: f64,
) -> Interval {
    let r0 = rotate_v(circ_pt(src_c, iv.s), axis, angle).normalize();
    Interval {
        s: norm_ang(pt_ang(dst_c, r0)),
        l: iv.l,
    }
}

pub fn same_circle(c1: &Circle, c2: &Circle) -> bool {
    c1.pole.dot(c2.pole) > 1.0 - 1e-6 && (c1.colat - c2.colat).abs() < 1e-6
}

pub fn find_circ(list: &[Circle], circ: &Circle) -> Option<usize> {
    list.iter().position(|c| same_circle(c, circ))
}

pub fn derive_axis_angle(n_a: u32, n_b: u32, p: u32, q: u32) -> Option<f64> {
    let c_a = (PI / n_a as f64).cos();
    let s_a = (PI / n_a as f64).sin();
    let c_b = (PI / n_b as f64).cos();
    let s_b = (PI / n_b as f64).sin();
    let c_g = (PI * p as f64 / q as f64).cos();
    let denom = s_a * s_b;
    if denom.abs() < 1e-12 {
        return None;
    }
    let cos_t = (c_a * c_b - c_g) / denom;
    if !(-1.0 - 1e-9..=1.0 + 1e-9).contains(&cos_t) {
        return None;
    }
    Some(cos_t.clamp(-1.0, 1.0).acos())
}

pub fn compute_arcs(
    axis_angle_rad: f64,
    colat_a: f64,
    colat_b: f64,
    n_a: u32,
    n_b: u32,
) -> (Vec<Circle>, Vec<Arc>) {
    let axis_a = DVec3::new(0.0, 0.0, 1.0);
    let axis_b = DVec3::new(axis_angle_rad.sin(), 0.0, axis_angle_rad.cos());
    let cut_axes = [axis_a, axis_b];
    let rot_orders = [n_a, n_b];
    let colats = [colat_a, colat_b];

    let mut circles = Vec::new();
    let mut covered: Vec<Vec<Interval>> = Vec::new();
    let mut disp_arcs = Vec::new();

    circles.push(make_circ(axis_a, colat_a));
    covered.push(vec![Interval { s: 0.0, l: TAU }]);
    disp_arcs.push(Arc {
        circ_idx: 0,
        s: 0.0,
        l: TAU,
    });

    circles.push(make_circ(axis_b, colat_b));
    covered.push(vec![Interval { s: 0.0, l: TAU }]);
    disp_arcs.push(Arc {
        circ_idx: 1,
        s: 0.0,
        l: TAU,
    });

    let mut step_start = 0;
    for _ in 0..100 {
        let before = disp_arcs.len();
        let mut bailout = false;

        for mi in 0..2 {
            if bailout {
                break;
            }
            let axis = cut_axes[mi];
            let n = rot_orders[mi];
            let cap_colat = colats[mi];

            for ai in step_start..before {
                if bailout {
                    break;
                }
                let arc = disp_arcs[ai];
                let src_c = circles[arc.circ_idx];
                let cr = match cap_range(&src_c, axis, cap_colat) {
                    Some(v) => v,
                    None => continue,
                };
                let clipped = isect_iv(&Interval { s: arc.s, l: arc.l }, &cr);
                if clipped.is_empty() {
                    continue;
                }

                for k in 1..n {
                    if bailout {
                        break;
                    }
                    let rot_ang = k as f64 * TAU / n as f64;
                    let rot_pole = rotate_v(src_c.pole, axis, rot_ang).normalize();
                    let rot_c = make_circ(rot_pole, src_c.colat);
                    let rci = match find_circ(&circles, &rot_c) {
                        Some(idx) => idx,
                        None => {
                            let idx = circles.len();
                            circles.push(rot_c);
                            covered.push(Vec::new());
                            idx
                        }
                    };
                    let dst_c = circles[rci];
                    for clip in &clipped {
                        let rot_iv = map_arc_to_rotated(&src_c, &dst_c, clip, axis, rot_ang);
                        let remaining = subtract_iv(&rot_iv, &covered[rci]);
                        for r in remaining {
                            if r.l > 1e-6 {
                                disp_arcs.push(Arc {
                                    circ_idx: rci,
                                    s: r.s,
                                    l: r.l,
                                });
                                covered[rci].push(Interval { s: r.s, l: r.l });
                                if disp_arcs.len() - before > 1000 {
                                    bailout = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        if disp_arcs.len() == before {
            break;
        }
        step_start = before;
    }
    (circles, disp_arcs)
}

pub fn merge_arcs(arcs: &[Arc]) -> Vec<Arc> {
    let mut by_circ: BTreeMap<usize, Vec<(f64, f64)>> = BTreeMap::new();
    for a in arcs {
        by_circ.entry(a.circ_idx).or_default().push((a.s, a.l));
    }
    let mut merged = Vec::new();
    for (ci, mut segs) in by_circ {
        segs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut result: Vec<(f64, f64)> = vec![segs[0]];
        for seg in segs.iter().skip(1) {
            let (prev_s, prev_l) = *result.last().unwrap();
            let end = norm_ang(prev_s + prev_l);
            let gap = norm_ang(seg.0 - end);
            if gap < 1e-4 {
                result.last_mut().unwrap().1 += gap + seg.1;
            } else {
                result.push(*seg);
            }
        }
        if result.len() > 1 {
            let (last_s, last_l) = result.last().unwrap();
            let (first_s, first_l) = result[0];
            let end = norm_ang(last_s + last_l);
            let gap = norm_ang(first_s - end);
            if gap < 1e-4 {
                result[0] = (*last_s, *last_l + gap + first_l);
                result.pop();
            }
        }
        for (s, l) in result {
            merged.push(Arc {
                circ_idx: ci,
                s,
                l: l.min(TAU),
            });
        }
    }
    merged
}

pub fn sample_arc(circ: &Circle, start: f64, length: f64, npts: usize) -> Vec<[f32; 3]> {
    let mut pts = Vec::with_capacity(npts + 1);
    for i in 0..=npts {
        let theta = start + length * (i as f64) / (npts as f64);
        let p = circ_pt(circ, theta);
        pts.push([
            (p.x * DISP_R) as f32,
            (p.y * DISP_R) as f32,
            (p.z * DISP_R) as f32,
        ]);
    }
    pts
}

// --- Polygon / Face Detection ---

pub fn intersect_circles(c1: &Circle, c2: &Circle) -> Vec<DVec3> {
    let n1 = c1.pole;
    let n2 = c2.pole;
    let d1 = c1.colat.cos();
    let d2 = c2.colat.cos();
    let dot = n1.dot(n2);
    if dot.abs() > 1.0 - 1e-6 {
        return vec![];
    }
    let det = 1.0 - dot * dot;
    let ca = (d1 - dot * d2) / det;
    let cb = (d2 - dot * d1) / det;
    let x0 = n1 * ca + n2 * cb;
    if x0.length_squared() > 1.0 {
        return vec![];
    }
    let l_dir = n1.cross(n2);
    let t = ((1.0 - x0.length_squared()) / l_dir.length_squared()).sqrt();
    vec![x0 + l_dir * t, x0 - l_dir * t]
}

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
            let pts = intersect_circles(c1, c2);
            for p in &pts {
                let ang1 = pt_ang(c1, *p);
                let da1 = norm_ang(ang1 - arcs[i].s);
                if da1 <= arcs[i].l + 1e-5 {
                    cuts[i].push(da1);
                }
                let ang2 = pt_ang(c2, *p);
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
            let p1 = circ_pt(c, arcs[i].s + s);
            let p2 = circ_pt(c, arcs[i].s + e);
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
    let mut faces = Vec::new();
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
                        let ang1 = pt_ang(c, p1);
                        let ang2 = pt_ang(c, p2);
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
                        let v = arc_integral(c, arc, da1, da2);
                        let angle = norm_ang(da2 - da1).min(norm_ang(da1 - da2));
                        sum += v * angle;
                    }
                }

                // Filter out tiny faces
                if perimeter > 0.02 {
                    faces.push(Face {
                        center: sum.normalize() * LABEL_R,
                    });
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_arc_integral_vs_avg() {
        // Test that integral formula closely
        //   matches sampling algo with high number of samples

        let pole = DVec3::new(0.0, 0.0, 1.0);
        let u = DVec3::new(1.0, 0.0, 0.0);
        let w = DVec3::new(0.0, 1.0, 0.0);
        let c_simple = Circle {
            pole,
            colat: PI / 3.0,
            u,
            w,
        };
        let arc_full = Arc {
            circ_idx: 0,
            s: 0.0,
            l: TAU,
        };

        let simple_cases: Vec<(f64, f64, &str)> = vec![
            (0.5, 1.5, "normal short arc"),
            (0.1, TAU - 0.1, "wrap-around across 0 (short path is ~0.2)"),
            (5.5, 0.5, "da0 > da1, wraps forward"),
            (TAU - 0.3, 0.3, "symmetric wrap"),
            (3.0, 3.5, "arc near PI"),
            (0.0, PI - 0.01, "nearly half-circle"),
        ];

        for (da0, da1, label) in &simple_cases {
            let avg = arc_avg(&c_simple, &arc_full, *da0, *da1, 10000);
            let integral = arc_integral(&c_simple, &arc_full, *da0, *da1);
            let dist = avg.distance(integral);
            assert!(
                dist < 0.01,
                "arc_integral != arc_avg for case '{}': dist={}, avg={:?}, integral={:?}",
                label,
                dist,
                avg,
                integral
            );
        }
    }
}
