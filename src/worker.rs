use puzzle_explorer_math::geometry::{compute_arcs, merge_arcs};
use puzzle_explorer_math::math::TAU;

use puzzle_explorer_math::orbit::{OrbitAnalysisInput, compute_orbit_analysis};
use puzzle_explorer_math::polygon::PolygonOptions;

use wasm_bindgen::prelude::*;

use serde::{Deserialize, Serialize};

use crate::app::{AxisDef, cvt_axis_defs};
use crate::types::FudgedModeSettings;

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
        WorkerMessage::ComputeGeometry {
            axes,
            max_iterations_cap,
        } => {
            let mut lines = Vec::new();
            let axes = cvt_axis_defs(&axes);
            if !axes.is_empty() {
                let (circles, arcs) =
                    compute_arcs(&axes, max_iterations_cap.map(|cap| cap as usize));
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

        WorkerMessage::ComputeOrbits {
            axes,
            max_iterations_cap,
            fudged_mode_settings,
        } => {
            let axes = cvt_axis_defs(&axes);
            if axes.is_empty() {
                WorkerResponse::Error("No axes defined".to_string())
            } else {
                let (circles, arcs) =
                    compute_arcs(&axes, max_iterations_cap.map(|cap| cap as usize));
                let arcs = merge_arcs(&arcs);

                let analysis = match compute_orbit_analysis(OrbitAnalysisInput {
                    circles: &circles,
                    arcs: &arcs,
                    axes: &axes,
                    options: match fudged_mode_settings {
                        Some(settings) => PolygonOptions::FudgedMode {
                            min_piece_angle_rad: Some(settings.min_piece_angle_deg.to_radians()),
                            min_piece_perimeter: settings.min_piece_perimeter,
                        },
                        None => PolygonOptions::Default,
                    },
                }) {
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
                let mut face_orbit_indices = vec![None; n_faces];
                for (oi, members) in analysis.orbits.iter().enumerate() {
                    for &fi in members {
                        face_orbit_indices[fi] = Some(oi);
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
    };

    serde_wasm_bindgen::to_value(&response).unwrap()
}

// --- Worker Messages ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WorkerMessage {
    ComputeGeometry {
        axes: Vec<Option<AxisDef>>,
        max_iterations_cap: Option<u32>,
    },
    ComputeOrbits {
        axes: Vec<Option<AxisDef>>,
        max_iterations_cap: Option<u32>,
        fudged_mode_settings: Option<FudgedModeSettings>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum WorkerResponse {
    GeometryComputed(GeometryResult),
    OrbitsComputed(OrbitResult),
    Error(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeometryResult {
    pub lines: Vec<PolyLine>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrbitResult {
    pub orbit_count: usize,
    pub face_count: usize,
    pub face_positions: Vec<[f32; 3]>,
    pub face_orbit_indices: Vec<Option<usize>>,
    pub generators: Vec<Vec<Vec<Vec<usize>>>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolyLine {
    pub points: Vec<[f32; 3]>,
    pub is_loop: bool,
}
