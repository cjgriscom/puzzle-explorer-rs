use serde::{Deserialize, Serialize};

// --- Axis Definition ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AxisDef {
    pub colat: f32,          // colatitude in degrees
    pub direction: [f64; 3], // unit direction vector
    pub n: u32,              // rotational symmetry order
}

// --- Geometry ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeometryParams {
    pub axes: Vec<AxisDef>,
    pub max_iterations_cap: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeometryResult {
    pub lines: Vec<PolyLine>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolyLine {
    pub points: Vec<[f32; 3]>,
    pub is_loop: bool,
}

// --- Orbits ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrbitParams {
    pub axes: Vec<AxisDef>,
    pub max_iterations_cap: Option<u32>,
    pub fudged_mode: bool,
    pub min_piece_angle_deg: f32,
    pub min_piece_perimeter: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrbitResult {
    pub orbit_count: usize,
    pub face_count: usize,
    pub face_positions: Vec<[f32; 3]>,
    pub face_orbit_indices: Vec<Option<usize>>,
    pub generators: Vec<Vec<Vec<Vec<usize>>>>,
}
