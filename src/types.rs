use std::collections::{HashMap, HashSet};

use glam::DVec3;

/// Closable window states
#[derive(Clone, Debug, PartialEq)]
pub struct WindowState {
    pub show_controls: bool,
    pub show_measure_axis_angle: bool,
    pub show_gap_console: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            show_controls: false,
            show_measure_axis_angle: false,
            show_gap_console: true,
        }
    }
}
/// Axis entry in puzzle params
#[derive(Clone, Debug, PartialEq)]
pub struct AxisEntry {
    pub axis_name: String, // references an axis definition (or X/Y/Z)
    pub n: u32,
    pub colat: f32,
    pub n_match: bool, // when true, n syncs from CosineRule definition
    pub enabled: bool, // when false, axis is skipped during geometry build
}

impl Default for AxisEntry {
    fn default() -> Self {
        Self {
            axis_name: String::new(),
            n: 3,
            colat: 109.5,
            n_match: false,
            enabled: true,
        }
    }
}

/// Puzzle parameters window state
#[derive(Clone, Debug, PartialEq)]
pub struct PuzzleParams {
    pub max_iterations: u32,
    pub lock_cuts: bool,
    pub show_axes: bool,
    pub axes: Vec<AxisEntry>,
}

impl Default for PuzzleParams {
    fn default() -> Self {
        Self {
            max_iterations: 30,
            lock_cuts: true,
            show_axes: true,
            axes: vec![
                AxisEntry {
                    axis_name: "Trapentrix_A".to_string(),
                    n: 3,
                    colat: 109.5,
                    n_match: true,
                    enabled: true,
                },
                AxisEntry {
                    axis_name: "Trapentrix_B".to_string(),
                    n: 3,
                    colat: 109.5,
                    n_match: true,
                    enabled: true,
                },
            ],
        }
    }
}

/// Orbit analysis window state
#[derive(Clone, Debug, PartialEq)]
pub struct OrbitAnalysisState {
    pub annotate_pieces: bool,
    pub number_pieces: bool,
    pub fudged_mode: bool,
    pub min_piece_angle_deg: f32,
    pub min_piece_perimeter: f64,
    pub auto_update_orbits: bool,
    pub auto_update_groups: bool,
    pub orbits_stale: bool,
    pub groups_stale: bool,
}

impl Default for OrbitAnalysisState {
    fn default() -> Self {
        Self {
            annotate_pieces: true,
            number_pieces: false,
            fudged_mode: false,
            min_piece_angle_deg: 5.0,
            min_piece_perimeter: 0.02,
            auto_update_orbits: false,
            auto_update_groups: false,
            orbits_stale: false,
            groups_stale: false,
        }
    }
}

/// Measure axis angle window state
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MeasureAxisAngleWindowState {
    pub axis_a: String,
    pub axis_b: String,
}

pub struct AxisDefinitions {
    /// User-defined axes in insertion order
    pub definitions: Vec<(String, DerivedAxis)>,
    /// Currently visible axes
    pub visible: HashSet<String>,
    /// Visibility of the builtin X, Y, Z reference axes.
    pub show_builtin: [bool; 3],
    /// Resolved vectors results (includes built-in & sub-indexed)
    pub resolved: HashMap<String, Result<Vec<DVec3>, String>>,
    /// When Some, rename dialog is active: (original_name, text_buffer)
    pub rename_state: Option<(String, String)>,
    /// When Some, delete confirmation is pending for this axis
    pub pending_delete: Option<String>,
}

impl Default for AxisDefinitions {
    fn default() -> Self {
        let mut result = Self {
            definitions: Vec::new(),
            visible: HashSet::new(),
            show_builtin: [false; 3],
            resolved: HashMap::new(),
            rename_state: None,
            pending_delete: None,
        };

        result.definitions.push((
            "Trapentrix".to_string(),
            DerivedAxis::CosineRule {
                p: 1,
                q: 5,
                n_a: 3,
                n_b: 3,
                a_axis: "X".to_string(),
                perpendicular_axis: "Y".to_string(),
                manual_axis_angle: false,
                manual_axis_angle_deg: 0.0,
            },
        ));
        result
    }
}

// --- Axis formats ---
#[derive(Clone, Debug)]
pub enum DerivedAxis {
    // Raw vector definition
    Vector {
        x: f64,
        y: f64,
        z: f64,
    },
    // Defines a normalized vector based on pitch and yaw
    Euler {
        pitch: f64,
        yaw: f64,
    },
    // Makes a copy of an axis for renaming or inverting it
    Copy {
        axis: String,
        invert: bool,
    },
    // Cross product of the normalized references
    CrossProduct {
        a0: String,
        a1: String,
    },
    // Normalizes and averages the referenced axes
    Average {
        axes: Vec<String>,
    },
    // Spherical law of cosines (puzzle-explorer-math::geometry::derive_axis_angle)
    CosineRule {
        p: u32,
        q: u32,
        n_a: u32,
        n_b: u32,
        a_axis: String,             // Axis A
        perpendicular_axis: String, // axis A is copied and rotated around this axis by the derived angle
        manual_axis_angle: bool,
        manual_axis_angle_deg: f64,
    },
    // Copy target_axis n times (2+) around pattern_axis along the angle range
    CircularPattern {
        pattern_axis: String,
        target_axis: String,
        n: u32,
        angle_range_deg: f64,
        invert: bool,
    },
}
