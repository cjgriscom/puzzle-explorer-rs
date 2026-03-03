use puzzle_explorer_math::geometry::{
    compute_arcs, derive_axis_angle, merge_arcs,
};
use puzzle_explorer_math::math::TAU;
use puzzle_explorer_math::orbit::compute_orbit_analysis;

use crate::puzzle::{GeometryParams, GeometryResult, OrbitParams, OrbitResult, PolyLine};
use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};

// --- Worker Messages ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WorkerMessage {
    ComputeGeometry(GeometryParams),
    ComputeOrbits(OrbitParams),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WorkerResponse {
    GeometryComputed(GeometryResult),
    OrbitsComputed(OrbitResult),
    Error(String),
}

#[wasm_bindgen]
pub fn worker_handle_msg(msg: JsValue) -> JsValue {
    console_error_panic_hook::set_once();

    let message: WorkerMessage = match serde_wasm_bindgen::from_value(msg) {
        Ok(m) => m,
        Err(e) => {
            let response = WorkerResponse::Error(format!("Deserialize error: {:?}", e));
            return serde_wasm_bindgen::to_value(&response).unwrap_or(JsValue::UNDEFINED);
        }
    };

    let response = match message {
        WorkerMessage::ComputeGeometry(params) => {
            let mut lines = Vec::new();
            if let Some(axis_angle) = params
                .axis_angle_override
                .or_else(|| derive_axis_angle(params.n_a, params.n_b, params.p, params.q))
            {
                let (circles, arcs) = compute_arcs(
                    axis_angle,
                    params.colat_a.to_radians() as f64,
                    params.colat_b.to_radians() as f64,
                    params.n_a,
                    params.n_b,
                );
                let arcs = merge_arcs(&arcs);

                for arc in &arcs {
                    let circ = &circles[arc.circ_idx];
                    let is_full = arc.l > TAU - 0.01;
                    let npts = if is_full {
                        128
                    } else {
                        std::cmp::max(16, (arc.l / TAU * 128.0).round() as usize)
                    };
                    let pts = circ.sample_arc(arc.s, arc.l, npts);
                    lines.push(PolyLine {
                        points: pts,
                        is_loop: is_full,
                    });
                }
            }
            WorkerResponse::GeometryComputed(GeometryResult { lines })
        }

        WorkerMessage::ComputeOrbits(params) => {
            match params
                .axis_angle_override
                .or_else(|| derive_axis_angle(params.n_a, params.n_b, params.p, params.q))
            {
                None => {
                    WorkerResponse::Error("No valid axis angle for these parameters".to_string())
                }
                Some(axis_angle) => {
                    let colat_a = params.colat_a.to_radians() as f64;
                    let colat_b = params.colat_b.to_radians() as f64;

                    let (circles, arcs) =
                        compute_arcs(axis_angle, colat_a, colat_b, params.n_a, params.n_b);
                    let arcs = merge_arcs(&arcs);

                    let analysis = match compute_orbit_analysis(
                        &circles, &arcs, params.n_a, params.n_b, axis_angle, colat_a, colat_b,
                    ) {
                        Ok(a) => a,
                        Err(e) => {
                            let r = WorkerResponse::Error(e);
                            return serde_wasm_bindgen::to_value(&r).unwrap_or(JsValue::UNDEFINED);
                        }
                    };

                    let face_positions: Vec<[f32; 3]> = analysis
                        .face_positions
                        .iter()
                        .map(|p| [p.x as f32, p.y as f32, p.z as f32])
                        .collect();

                    let n_faces = face_positions.len();
                    let mut face_orbit_indices = vec![0usize; n_faces];
                    for (oi, members) in analysis.orbits.iter().enumerate() {
                        for &fi in members {
                            face_orbit_indices[fi] = oi;
                        }
                    }

                    WorkerResponse::OrbitsComputed(OrbitResult {
                        orbit_count: analysis.orbits.len(),
                        face_count: n_faces,
                        face_positions,
                        face_orbit_indices,
                        generators: analysis.generators,
                    })
                }
            }
        }
    };

    serde_wasm_bindgen::to_value(&response).unwrap()
}
