use serde::{Deserialize, Serialize};

// --- Geometry ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeometryParams {
    pub n_a: u32,
    pub n_b: u32,
    pub p: u32,
    pub q: u32,
    pub colat_a: f32,
    pub colat_b: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeometryResult {
    pub lines: Vec<PolyLine>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PolyLine {
    pub points: Vec<[f32; 3]>,
    pub is_loop: bool,
    pub color: [f32; 3],
}

// --- Orbits ---

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrbitParams {
    pub n_a: u32,
    pub n_b: u32,
    pub p: u32,
    pub q: u32,
    pub colat_a: f32,
    pub colat_b: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OrbitResult {
    pub orbit_count: usize,
    pub face_count: usize,
    pub face_positions: Vec<[f32; 3]>,
    pub face_orbit_indices: Vec<usize>,
    pub generators: Vec<Vec<Vec<Vec<usize>>>>,
}
