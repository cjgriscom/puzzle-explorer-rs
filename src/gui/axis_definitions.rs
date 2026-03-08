use std::collections::HashMap;

use glam::{DAffine3, DVec3};
use puzzle_explorer_math::geometry::derive_axis_angle;

use crate::app::PuzzleApp;

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
    // Takes the cross product of the normalized references
    CrossProduct {
        a0: String,
        a1: String,
    },
    // Normalizes and averages the referenced axes
    Average {
        axes: Vec<String>,
    },
    // Will's biaxe equation (puzzle-explorer-math::geometry::derive_axis_angle)
    //   returns empty if a_axis and dir_axis
    WillsEquation {
        p: u32,
        q: u32,
        n_a: u32,
        n_b: u32,
        a_axis: String,             // Axis A
        perpendicular_axis: String, // axis A is copied and rotated around this axis by the derived angle
    },
    // Copy target_axis n times (2+) around pattern_axis along the angle range
    CircularPattern {
        pattern_axis: String,
        target_axis: String,
        n: u32,
        angle_range_deg: f64,
    },
}

impl DerivedAxis {
    pub fn resultant_vectors(
        &self,
        axis_map: &HashMap<String, DVec3>,
    ) -> Result<Vec<DVec3>, String> {
        match self {
            DerivedAxis::Vector { x, y, z } => {
                if *x == 0.0 && *y == 0.0 && *z == 0.0 {
                    Err("Can not derive an axis from the zero vector".to_string())
                } else {
                    Ok(vec![DVec3::new(*x, *y, *z)])
                }
            }
            DerivedAxis::Euler { pitch, yaw } => {
                let pitch_rad = pitch.to_radians();
                let yaw_rad = yaw.to_radians();
                let x = pitch_rad.cos() * yaw_rad.sin();
                let y = pitch_rad.sin();
                let z = pitch_rad.cos() * yaw_rad.cos();
                let v = DVec3::new(x, y, z);
                if v.length_squared() < 1e-12 {
                    Err("Euler angles resulted in a zero vector".to_string())
                } else {
                    Ok(vec![v.normalize()])
                }
            }
            DerivedAxis::Copy { axis, invert } => {
                let v = axis_map
                    .get(axis)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", axis))?;
                Ok(vec![if *invert { -*v } else { *v }])
            }
            DerivedAxis::CrossProduct { a0, a1 } => {
                let v0 = axis_map
                    .get(a0)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", a0))?
                    .normalize();
                let v1 = axis_map
                    .get(a1)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", a1))?
                    .normalize();
                let cross = v0.cross(v1);
                if cross.length_squared() < 1e-12 {
                    Err("Axes are parallel".to_string())
                } else {
                    Ok(vec![cross.normalize()])
                }
            }
            DerivedAxis::Average { axes } => {
                if axes.is_empty() {
                    return Err("Average requires at least one axis".to_string());
                }
                let mut sum = DVec3::ZERO;
                for name in axes {
                    let v = axis_map
                        .get(name)
                        .ok_or_else(|| format!("Referenced axis '{}' not found", name))?;
                    sum += v.normalize();
                }
                if sum.length_squared() < 1e-12 {
                    Err("Average resulted in a zero vector".to_string())
                } else {
                    Ok(vec![sum.normalize()])
                }
            }
            DerivedAxis::WillsEquation {
                p,
                q,
                n_a,
                n_b,
                a_axis,
                perpendicular_axis,
            } => {
                let a_vec = axis_map
                    .get(a_axis)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", a_axis))?
                    .normalize();
                let perp_vec = axis_map
                    .get(perpendicular_axis)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", perpendicular_axis))?
                    .normalize();
                let angle = derive_axis_angle(*n_a, *n_b, *p, *q)
                    .ok_or_else(|| "Failed to derive axis angle".to_string())?;
                let rotation = DAffine3::from_axis_angle(perp_vec, angle);
                let result = rotation.transform_vector3(a_vec);
                Ok(vec![result.normalize()])
            }
            DerivedAxis::CircularPattern {
                pattern_axis,
                target_axis,
                n,
                angle_range_deg,
            } => {
                if *n < 2 {
                    return Err("Expected n >= 2".to_string());
                }
                if *angle_range_deg < 0.0 || *angle_range_deg > 360.0 {
                    return Err("Expected angle range in [0, 360]".to_string());
                }
                let pattern_vec = axis_map
                    .get(pattern_axis)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", pattern_axis))?
                    .normalize();
                let target_vec = axis_map
                    .get(target_axis)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", target_axis))?;
                let angle_range_rad = angle_range_deg.to_radians();
                let mut results = Vec::with_capacity(*n as usize);
                for i in 0..*n {
                    let angle = angle_range_rad * i as f64 / *n as f64;
                    let rotation = DAffine3::from_axis_angle(pattern_vec, angle);
                    results.push(rotation.transform_vector3(*target_vec).normalize());
                }
                Ok(results)
            }
        }
    }

    pub fn dependencies(&self) -> Vec<String> {
        match self {
            DerivedAxis::Vector { .. } => Vec::new(),
            DerivedAxis::Euler { .. } => Vec::new(),
            DerivedAxis::Copy { axis, .. } => {
                vec![axis.clone()]
            }
            DerivedAxis::CrossProduct { a0, a1 } => {
                vec![a0.clone(), a1.clone()]
            }
            DerivedAxis::Average { axes } => axes.clone(),
            DerivedAxis::WillsEquation {
                a_axis,
                perpendicular_axis,
                ..
            } => {
                vec![a_axis.clone(), perpendicular_axis.clone()]
            }
            DerivedAxis::CircularPattern {
                pattern_axis,
                target_axis,
                ..
            } => {
                vec![pattern_axis.clone(), target_axis.clone()]
            }
        }
    }
}

pub fn build_axis_definitions_window(app: &mut PuzzleApp, ctx: &egui::Context) {}
