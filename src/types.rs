use std::collections::{BTreeSet, HashMap};

use glam::DVec3;
use serde::{Deserialize, Serialize};

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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisEntry {
    pub axis_name: String, // references an axis definition (or X/Y/Z)
    pub n: u32,
    pub colatitude_deg: f32,
    pub n_match: bool, // when true, n syncs from CosineRule definition
    pub enabled: bool, // when false, axis is skipped during geometry build
}

impl Default for AxisEntry {
    fn default() -> Self {
        Self {
            axis_name: String::new(),
            n: 3,
            colatitude_deg: 89.0,
            n_match: false,
            enabled: true,
        }
    }
}

/// Puzzle parameters window state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PuzzleParams {
    #[serde(skip)]
    pub show_axes: bool,
    pub max_iterations: u32,
    pub lock_cuts: bool,
    pub axes: Vec<AxisEntry>,
}

impl PuzzleParams {
    /// Apply imported data, preserving transient UI state
    pub fn apply_imported(&mut self, imported: &Self) {
        self.max_iterations = imported.max_iterations;
        self.lock_cuts = imported.lock_cuts;
        self.axes = imported.axes.clone();
    }
}

impl Default for PuzzleParams {
    fn default() -> Self {
        Self {
            show_axes: true,
            max_iterations: 30,
            lock_cuts: true,
            axes: Vec::new(),
        }
    }
}

/// Fudged mode settings
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FudgedModeSettings {
    pub min_piece_angle_deg: f32,
    pub min_piece_perimeter: f64,
}

impl Default for FudgedModeSettings {
    fn default() -> Self {
        Self {
            min_piece_angle_deg: 5.0,
            min_piece_perimeter: 0.05,
        }
    }
}

/// Orbit analysis window state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OrbitAnalysisState {
    #[serde(skip)]
    pub annotate_pieces: bool,
    #[serde(skip)]
    pub number_pieces: bool,
    #[serde(skip)]
    pub auto_update_orbits: bool,
    #[serde(skip)]
    pub auto_update_groups: bool,
    #[serde(skip)]
    pub orbits_stale: bool,
    #[serde(skip)]
    pub groups_stale: bool,

    pub fudged_mode: bool,
    pub fudged_mode_settings: FudgedModeSettings,
}

impl OrbitAnalysisState {
    /// Apply imported data, preserving transient UI state
    pub fn apply_imported(&mut self, imported: &Self) {
        self.fudged_mode = imported.fudged_mode;
        self.fudged_mode_settings = imported.fudged_mode_settings.clone();
    }
}

impl Default for OrbitAnalysisState {
    fn default() -> Self {
        Self {
            annotate_pieces: true,
            number_pieces: false,
            auto_update_orbits: true,
            auto_update_groups: false,
            orbits_stale: false,
            groups_stale: false,
            fudged_mode: false,
            fudged_mode_settings: FudgedModeSettings::default(),
        }
    }
}

/// Measure axis angle window state
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MeasureAxisAngleWindowState {
    pub axis_a: String,
    pub axis_b: String,
}

/// Axis definitions window state
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct AxisDefinitions {
    /// User-defined axes in insertion order
    pub definitions: Vec<AxisDefinition>,
    /// Currently visible axes
    pub visible_axes: BTreeSet<String>,

    /// Resolved vectors results (includes built-in & sub-indexed)
    #[serde(skip)]
    pub resolved: HashMap<String, Result<Vec<DVec3>, String>>,
    /// When Some, rename dialog is active: (original_name, text_buffer)
    #[serde(skip)]
    pub rename_state: Option<(String, String)>,
    /// When Some, delete confirmation is pending for this axis
    #[serde(skip)]
    pub pending_delete: Option<String>,
}

impl AxisDefinitions {
    /// Apply imported data, preserving transient UI state
    pub fn apply_imported(&mut self, imported: &Self) {
        self.definitions = imported.definitions.clone();
        self.visible_axes = imported.visible_axes.clone();
        self.resolve_all();
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AxisDefinition {
    pub name: String,
    pub axis: DerivedAxis,
}

/// Axis definition formats
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
        manual_axis_angle_deg: Option<f64>,
    },
    // Copy target_axis n times (2+) around pattern_axis along the angle range
    CircularPattern {
        pattern_axis: String,
        target_axis: String,
        n: u32,
        angle_range_deg: f64,
        invert_range: bool,
    },
}
