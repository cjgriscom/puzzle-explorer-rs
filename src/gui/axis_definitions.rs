use std::collections::{HashMap, HashSet};

use egui::Button;
use glam::{DAffine3, DVec3};
use indexmap::IndexMap;
use puzzle_explorer_math::geometry::derive_axis_angle;

use crate::app::PuzzleApp;
use crate::gui::{
    AXIS_ANGLE_DECIMALS, AXIS_ANGLE_SPEED, AXIS_DEFINITIONS_POS, AXIS_DEFINITIONS_WIDTH,
    EULER_DECIMALS, EULER_SPEED,
};

// Axis formats

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
        manual_axis_angle: bool,
        manual_axis_angle_deg: f64,
    },
    // Copy target_axis n times (2+) around pattern_axis along the angle range
    CircularPattern {
        pattern_axis: String,
        target_axis: String,
        n: u32,
        angle_range_deg: f64,
    },
}

const VARIANT_LABELS: &[&str] = &[
    "Vector",
    "Euler",
    "Copy",
    "Cross Product",
    "Average",
    "Will's Equation",
    "Circular Pattern",
];

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
                manual_axis_angle,
                manual_axis_angle_deg,
            } => {
                let a_vec = axis_map
                    .get(a_axis)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", a_axis))?
                    .normalize();
                let perp_vec = axis_map
                    .get(perpendicular_axis)
                    .ok_or_else(|| format!("Referenced axis '{}' not found", perpendicular_axis))?
                    .normalize();
                if a_vec.cross(perp_vec).length_squared() < 1e-12 {
                    return Err("A axis and perpendicular axis are parallel".to_string());
                }
                let angle = if *manual_axis_angle {
                    manual_axis_angle_deg.to_radians()
                } else {
                    derive_axis_angle(*n_a, *n_b, *p, *q)
                        .ok_or_else(|| "Failed to derive axis angle".to_string())?
                };
                let rotation = DAffine3::from_axis_angle(perp_vec, angle);
                let b_vec = rotation.transform_vector3(a_vec).normalize();
                Ok(vec![a_vec, b_vec])
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

    /// Returns the sub-name suffixes this axis produces when it outputs multiple vectors.
    /// Empty if single-vector output (base name only).
    pub fn output_suffixes(&self) -> Vec<String> {
        match self {
            DerivedAxis::WillsEquation { .. } => vec!["A".into(), "B".into()],
            DerivedAxis::CircularPattern { n, .. } => (1..=*n).map(|i| i.to_string()).collect(),
            _ => Vec::new(),
        }
    }

    pub fn variant_index(&self) -> usize {
        match self {
            DerivedAxis::Vector { .. } => 0,
            DerivedAxis::Euler { .. } => 1,
            DerivedAxis::Copy { .. } => 2,
            DerivedAxis::CrossProduct { .. } => 3,
            DerivedAxis::Average { .. } => 4,
            DerivedAxis::WillsEquation { .. } => 5,
            DerivedAxis::CircularPattern { .. } => 6,
        }
    }

    pub fn default_for_variant(idx: usize) -> DerivedAxis {
        match idx {
            0 => DerivedAxis::Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            1 => DerivedAxis::Euler {
                pitch: 0.0,
                yaw: 0.0,
            },
            2 => DerivedAxis::Copy {
                axis: "X".to_string(),
                invert: false,
            },
            3 => DerivedAxis::CrossProduct {
                a0: "X".to_string(),
                a1: "Y".to_string(),
            },
            4 => DerivedAxis::Average {
                axes: vec!["X".to_string(), "Y".to_string()],
            },
            5 => DerivedAxis::WillsEquation {
                p: 1,
                q: 5,
                n_a: 3,
                n_b: 3,
                a_axis: "X".to_string(),
                perpendicular_axis: "Y".to_string(),
                manual_axis_angle: false,
                manual_axis_angle_deg: 0.0,
            },
            6 => DerivedAxis::CircularPattern {
                pattern_axis: "Z".to_string(),
                target_axis: "X".to_string(),
                n: 3,
                angle_range_deg: 360.0,
            },
            _ => DerivedAxis::Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        }
    }

    /// Replace all occurrences of `old_name` with `new_name` in string references.
    fn rename_references(&mut self, old_name: &str, new_name: &str) {
        match self {
            DerivedAxis::Copy { axis, .. } => {
                if axis == old_name {
                    *axis = new_name.to_string();
                }
            }
            DerivedAxis::CrossProduct { a0, a1 } => {
                if a0 == old_name {
                    *a0 = new_name.to_string();
                }
                if a1 == old_name {
                    *a1 = new_name.to_string();
                }
            }
            DerivedAxis::Average { axes } => {
                for a in axes.iter_mut() {
                    if a == old_name {
                        *a = new_name.to_string();
                    }
                }
            }
            DerivedAxis::WillsEquation {
                a_axis,
                perpendicular_axis,
                ..
            } => {
                if a_axis == old_name {
                    *a_axis = new_name.to_string();
                }
                if perpendicular_axis == old_name {
                    *perpendicular_axis = new_name.to_string();
                }
            }
            DerivedAxis::CircularPattern {
                pattern_axis,
                target_axis,
                ..
            } => {
                if pattern_axis == old_name {
                    *pattern_axis = new_name.to_string();
                }
                if target_axis == old_name {
                    *target_axis = new_name.to_string();
                }
            }
            _ => {}
        }
    }
}

// AxisDefinitions state

pub struct AxisDefinitions {
    /// User-defined axes in insertion order. Does NOT contain X, Y, Z.
    pub definitions: IndexMap<String, DerivedAxis>,
    /// Which axes are currently visible.
    pub visible: HashSet<String>,
    /// Resolved vectors (includes X, Y, Z and sub-indexed multi-vector results).
    pub resolved: HashMap<String, Result<Vec<DVec3>, String>>,
    /// When Some, a rename dialog is active: (original_name, text_buffer).
    pub rename_state: Option<(String, String)>,
    /// When Some, a delete confirmation is pending for this axis name.
    pub pending_delete: Option<String>,
}

impl Default for AxisDefinitions {
    fn default() -> Self {
        let mut result = Self {
            definitions: IndexMap::new(),
            visible: HashSet::new(),
            resolved: HashMap::new(),
            rename_state: None,
            pending_delete: None,
        };

        result.definitions.insert(
            "Trapentrix".to_string(),
            DerivedAxis::WillsEquation {
                p: 1,
                q: 5,
                n_a: 3,
                n_b: 3,
                a_axis: "X".to_string(),
                perpendicular_axis: "Y".to_string(),
                manual_axis_angle: false,
                manual_axis_angle_deg: 0.0,
            },
        );
        result
    }
}

impl AxisDefinitions {
    // ---- Resolution with cycle detection ----

    /// Resolve all axes. Performs topological sort; cycle participants get an error.
    pub fn resolve_all(&mut self) {
        let mut axis_map: HashMap<String, DVec3> = HashMap::new();
        // Builtins
        axis_map.insert("X".to_string(), DVec3::X);
        axis_map.insert("Y".to_string(), DVec3::Y);
        axis_map.insert("Z".to_string(), DVec3::Z);

        self.resolved.clear();

        // Topological sort via Kahn's algorithm
        let names: Vec<String> = self.definitions.keys().cloned().collect();
        let builtins: HashSet<String> = axis_map.iter().map(|s| s.0.to_string()).collect();

        // Build in-degree map (only counting edges within user-defined axes)
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for name in &names {
            in_degree.insert(name.clone(), 0);
        }

        // For each axis, count how many UNIQUE base deps are user-defined
        for (name, axis) in &self.definitions {
            let mut base_deps: HashSet<String> = HashSet::new();
            for dep in axis.dependencies() {
                // TODO Strip sub-index suffix... ideally this should resolve to the base in a better way
                let base = strip_sub_index(&dep);
                if !builtins.contains(&base) && self.definitions.contains_key(&base) {
                    base_deps.insert(base);
                }
            }
            in_degree.insert(name.clone(), base_deps.len());
        }

        // Seed the queue with axes that have zero in-degree
        let mut queue: Vec<String> = Vec::new();
        for (name, deg) in &in_degree {
            if *deg == 0 {
                queue.push(name.clone());
            }
        }

        let mut resolved_order: Vec<String> = Vec::new();
        while let Some(name) = queue.pop() {
            resolved_order.push(name.clone());
            // Decrease in-degree of axes that depend on `name`
            for (other_name, other_axis) in &self.definitions {
                if in_degree.get(other_name).copied().unwrap_or(0) == 0 {
                    continue; // already processed or will be
                }
                let deps = other_axis.dependencies();
                let depends_on_name = deps.iter().any(|d| {
                    let base = strip_sub_index(d);
                    base == name
                });
                if depends_on_name {
                    let deg = in_degree.get_mut(other_name).unwrap();
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push(other_name.clone());
                    }
                }
            }
        }

        // Any axis not in resolved_order is part of a cycle
        let resolved_set: HashSet<&String> = resolved_order.iter().collect();
        for name in &names {
            if !resolved_set.contains(name) {
                self.resolved
                    .insert(name.clone(), Err("Circular axis reference".to_string()));
            }
        }

        // Resolve in topological order
        for name in &resolved_order {
            let axis = match self.definitions.get(name) {
                Some(a) => a,
                None => continue,
            };
            let result = axis.resultant_vectors(&axis_map);
            if let Ok(vecs) = &result {
                if vecs.len() == 1 {
                    axis_map.insert(name.clone(), vecs[0]);
                } else {
                    // Multi-vector: use _A/_B for WillsEquation, _1/_2/... otherwise
                    let is_wills = matches!(axis, DerivedAxis::WillsEquation { .. });
                    if is_wills && vecs.len() == 2 {
                        axis_map.insert(format!("{}_A", name), vecs[0]);
                        axis_map.insert(format!("{}_B", name), vecs[1]);
                    } else {
                        for (i, v) in vecs.iter().enumerate() {
                            axis_map.insert(format!("{}_{}", name, i + 1), *v);
                        }
                    }
                }
            }
            self.resolved.insert(name.clone(), result);
        }
    }

    /// Returns all axis names available for reference in dropdowns.
    /// Includes builtins X, Y, Z plus user-defined names (or sub-indexed names for multi-vector).
    pub fn available_axis_names(&self) -> Vec<String> {
        let mut names = vec!["X".to_string(), "Y".to_string(), "Z".to_string()];
        for (name, axis) in &self.definitions {
            match self.resolved.get(name) {
                Some(Ok(vecs)) if vecs.len() > 1 => {
                    let is_wills = matches!(axis, DerivedAxis::WillsEquation { .. });
                    if is_wills && vecs.len() == 2 {
                        names.push(format!("{}_A", name));
                        names.push(format!("{}_B", name));
                    } else {
                        for i in 0..vecs.len() {
                            names.push(format!("{}_{}", name, i + 1));
                        }
                    }
                }
                _ => {
                    names.push(name.clone());
                }
            }
        }
        names
    }

    /// For a given axis name referencing a WillsEquation definition,
    /// return the matching n value (n_a for _A, n_b for _B).
    pub fn get_wills_n_for_axis(&self, axis_name: &str) -> Option<u32> {
        if let Some(pos) = axis_name.rfind('_') {
            let base = &axis_name[..pos];
            let suffix = &axis_name[pos + 1..];
            if let Some(DerivedAxis::WillsEquation { n_a, n_b, .. }) = self.definitions.get(base) {
                return match suffix {
                    "A" => Some(*n_a),
                    "B" => Some(*n_b),
                    _ => None,
                };
            }
        }
        None
    }

    /// Look up a single resolved vector by name, handling sub-indexed names.
    /// Returns Some(vec) for valid single-vector names (including "Foo_1" sub-indices).
    pub fn get_resolved_vector(&self, name: &str) -> Option<glam::DVec3> {
        // Direct lookup: if the base name resolves to exactly 1 vector
        if let Some(Ok(vecs)) = self.resolved.get(name)
            && vecs.len() == 1
        {
            return Some(vecs[0]);
        }
        // Sub-indexed lookup: check for _A/_B (WillsEquation) or _N (numeric)
        if let Some(pos) = name.rfind('_') {
            let base = &name[..pos];
            let suffix = &name[pos + 1..];
            if let Some(Ok(vecs)) = self.resolved.get(base) {
                // _A / _B for WillsEquation (2-vector)
                if vecs.len() == 2
                    && let Some(def) = self.definitions.get(base)
                    && matches!(def, DerivedAxis::WillsEquation { .. })
                {
                    match suffix {
                        "A" => return Some(vecs[0]),
                        "B" => return Some(vecs[1]),
                        _ => {}
                    }
                }
                // Numeric sub-index
                if let Ok(idx_1based) = suffix.parse::<usize>()
                    && idx_1based >= 1
                    && idx_1based <= vecs.len()
                {
                    return Some(vecs[idx_1based - 1]);
                }
            }
        }
        None
    }

    /// Collect all resolved vectors from visible axis definitions for grey indicator rendering.
    pub fn get_visible_vectors(&self) -> Vec<glam::DVec3> {
        let mut vecs = Vec::new();
        for (name, _axis) in &self.definitions {
            if !self.visible.contains(name) {
                continue;
            }
            if let Some(Ok(result_vecs)) = self.resolved.get(name) {
                for v in result_vecs {
                    vecs.push(*v);
                }
            }
        }
        vecs
    }

    /// Generate a unique name at (max N)+1 like "Axis 1"
    pub fn generate_name(&self) -> String {
        let mut max_n = 0u32;
        for name in self.definitions.keys() {
            if let Some(rest) = name.strip_prefix("Axis ")
                && let Ok(n) = rest.parse::<u32>()
            {
                max_n = max_n.max(n);
            }
        }
        format!("Axis {}", max_n + 1)
    }

    /// Prevent collision by appending a suffix automatically
    fn make_unique_name(&self, desired: &str) -> String {
        let builtins: HashSet<&str> = ["X", "Y", "Z"].into();
        if !builtins.contains(desired) && !self.definitions.contains_key(desired) {
            return desired.to_string();
        }
        let mut i = 2u32;
        loop {
            let candidate = format!("{} {}", desired, i);
            if !builtins.contains(candidate.as_str()) && !self.definitions.contains_key(&candidate)
            {
                return candidate;
            }
            i += 1;
        }
    }

    /// Find all axes that reference `target_name` in their dependencies
    fn find_dependents(&self, target_name: &str) -> Vec<String> {
        let mut result = Vec::new();
        for (name, axis) in &self.definitions {
            if name == target_name {
                continue;
            }
            let deps = axis.dependencies();
            if deps.iter().any(|d| {
                let base = strip_sub_index(d);
                base == target_name
            }) {
                result.push(name.clone());
            }
        }
        result
    }

    /// Rename, updating key and all references in other axes
    fn do_rename(&mut self, old_name: &str, new_name: &str) {
        if old_name == new_name {
            return;
        }
        // Rename key in-place preserving order by rebuilding the map
        let mut new_defs = IndexMap::new();
        for (k, v) in self.definitions.drain(..) {
            if k == old_name {
                new_defs.insert(new_name.to_string(), v);
            } else {
                new_defs.insert(k, v);
            }
        }
        self.definitions = new_defs;
        // Update visibility set
        if self.visible.remove(old_name) {
            self.visible.insert(new_name.to_string());
        }
        // Update references in all other axes
        // Get suffixes of the renamed axis to also rename sub-indexed references
        let suffixes = self
            .definitions
            .get(new_name)
            .map(|a| a.output_suffixes())
            .unwrap_or_default();
        let keys: Vec<String> = self.definitions.keys().cloned().collect();
        for key in keys {
            if let Some(axis) = self.definitions.get_mut(&key) {
                axis.rename_references(old_name, new_name);
                for suffix in &suffixes {
                    let old_sub = format!("{}_{}", old_name, suffix);
                    let new_sub = format!("{}_{}", new_name, suffix);
                    axis.rename_references(&old_sub, &new_sub);
                }
            }
        }
    }

    /// Delete an axis by name.
    fn do_delete(&mut self, name: &str) {
        self.definitions.shift_remove(name);
        self.visible.remove(name);
    }
}

/// Strip a sub-index suffix (e.g. "Trapentrix_A" -> "Trapentrix", "X" -> "X", "Pattern_1" -> "Pattern")
fn strip_sub_index(name: &str) -> String {
    if let Some(idx) = name.rfind('_') {
        let suffix = &name[idx + 1..];
        // Strip numeric sub-indices (_1, _2, ...) and WillsEquation labels (_A, _B)
        if suffix.parse::<u32>().is_ok() || suffix == "A" || suffix == "B" {
            return name[..idx].to_string();
        }
    }
    name.to_string()
}

pub fn build_axis_definitions_window(app: &mut PuzzleApp, ctx: &egui::Context) {
    let mut changed = false;

    // Handle delete confirmation dialog
    if let Some(name) = app.axis_defs.pending_delete.clone() {
        let dependents = app.axis_defs.find_dependents(&name);
        if dependents.is_empty() {
            // No dependents, delete immediately
            app.axis_defs.do_delete(&name);
            app.axis_defs.pending_delete = None;
            changed = true;
        } else {
            let mut open = true;
            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .show(ctx, |ui| {
                    let others = dependents.len().saturating_sub(1);
                    let msg = if others > 0 {
                        format!(
                            "This definition is referenced by {} and {} other(s). Continue deleting?",
                            dependents[0], others
                        )
                    } else {
                        format!(
                            "This definition is referenced by {}. Continue deleting?",
                            dependents[0]
                        )
                    };
                    ui.label(&msg);
                    ui.horizontal(|ui| {
                        if ui.button("Delete").clicked() {
                            app.axis_defs.do_delete(&name);
                            app.axis_defs.pending_delete = None;
                            changed = true;
                        }
                        if ui.button("Cancel").clicked() {
                            app.axis_defs.pending_delete = None;
                        }
                    });
                });
            if !open {
                app.axis_defs.pending_delete = None;
            }
        }
    }

    egui::Window::new("Axis Definitions")
        .default_pos(AXIS_DEFINITIONS_POS)
        .default_width(AXIS_DEFINITIONS_WIDTH)
        .show(ctx, |ui| {
            // Toolbar
            ui.horizontal(|ui| {
                if ui.button("Hide All").clicked() && !app.axis_defs.visible.is_empty() {
                    app.axis_defs.visible.clear();
                    changed = true;
                }

                if ui.button("Show All").clicked() {
                    for name in app.axis_defs.definitions.keys() {
                        if app.axis_defs.visible.insert(name.clone()) {
                            changed = true;
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("+ New Axis").clicked() {
                        let name = app.axis_defs.generate_name();
                        app.axis_defs
                            .definitions
                            .insert(name.clone(), DerivedAxis::default_for_variant(0));
                        app.axis_defs.visible.insert(name);
                        changed = true;
                    }
                });
            });

            // Snapshot keys for iteration (avoids borrow issues)
            let keys: Vec<String> = app.axis_defs.definitions.keys().cloned().collect();
            let available = app.axis_defs.available_axis_names();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for name in &keys {
                    ui.separator();

                    let result = app.axis_defs.resolved.get(name);
                    let is_err = matches!(result, Some(Err(_)));
                    let err_text = match result {
                        Some(Err(e)) => e.clone(),
                        _ => String::new(),
                    };

                    // Build header with colored name
                    let header_color = if is_err {
                        egui::Color32::from_rgb(255, 80, 80)
                    } else {
                        egui::Color32::from_rgb(100, 255, 100)
                    };

                    let header_text = egui::RichText::new(name).color(header_color);

                    // Use a horizontal layout to put buttons on the right
                    let id = ui.make_persistent_id(format!("axis_def_{}", name));
                    let mut state =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            ui.ctx(),
                            id,
                            true,
                        );

                    // Get variant index before mutable borrow for the combo box
                    let current_variant_idx = app
                        .axis_defs
                        .definitions
                        .get(name)
                        .map(|a| a.variant_index())
                        .unwrap_or(0);

                    let _header_resp = ui.horizontal(|ui| {
                        state.show_toggle_button(ui, egui::collapsing_header::paint_default_icon);

                        let label_resp = ui.label(header_text);
                        if is_err {
                            label_resp.on_hover_text(&err_text);
                        }

                        // Push buttons to the right
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // Delete button
                            if ui.add(Button::new("🗑")).clicked() {
                                app.axis_defs.pending_delete = Some(name.clone());
                            }

                            // Rename button
                            if ui.add(Button::new("✏")).clicked() {
                                app.axis_defs.rename_state = Some((name.clone(), name.clone()));
                            }

                            // Visible toggle
                            let is_visible = app.axis_defs.visible.contains(name);
                            if ui
                                .add(Button::new("👁").selected(is_visible).frame(true))
                                .clicked()
                            {
                                if !is_visible {
                                    app.axis_defs.visible.insert(name.clone());
                                } else {
                                    app.axis_defs.visible.remove(name);
                                }
                                changed = true;
                            }

                            // Type dropdown
                            let mut new_variant_idx = current_variant_idx;
                            egui::ComboBox::from_id_salt(format!("type_{}", name))
                                .selected_text(VARIANT_LABELS[current_variant_idx])
                                .show_ui(ui, |ui| {
                                    for (i, label) in VARIANT_LABELS.iter().enumerate() {
                                        if ui
                                            .selectable_label(i == current_variant_idx, *label)
                                            .clicked()
                                            && i != current_variant_idx
                                        {
                                            new_variant_idx = i;
                                        }
                                    }
                                });
                            if new_variant_idx != current_variant_idx
                                && let Some(axis) = app.axis_defs.definitions.get_mut(name)
                            {
                                *axis = DerivedAxis::default_for_variant(new_variant_idx);
                                changed = true;
                            }
                        });
                    });

                    // Rename inline editor
                    if let Some((ref rename_target, _)) = app.axis_defs.rename_state
                        && rename_target == name
                    {
                        let mut do_rename = false;
                        let mut cancel = false;
                        ui.horizontal(|ui| {
                            if let Some((_, ref mut buf)) = app.axis_defs.rename_state {
                                ui.text_edit_singleline(buf);
                                if ui.button("OK").clicked() {
                                    do_rename = true;
                                }
                                if ui.button("Cancel").clicked() {
                                    cancel = true;
                                }
                            }
                        });
                        if do_rename
                            && let Some((old, new_desired)) = app.axis_defs.rename_state.take()
                        {
                            let new_trimmed = new_desired.trim().to_string();
                            if !new_trimmed.is_empty() && new_trimmed != old {
                                let new_name = app.axis_defs.make_unique_name(&new_trimmed);
                                app.axis_defs.do_rename(&old, &new_name);
                                // Bubble rename into puzzle params
                                let suffixes = app
                                    .axis_defs
                                    .definitions
                                    .get(&new_name)
                                    .map(|a| a.output_suffixes())
                                    .unwrap_or_default();
                                rename_axis_in_puzzle_params(
                                    &mut app.params,
                                    &old,
                                    &new_name,
                                    &suffixes,
                                );
                                changed = true;
                            }
                        }
                        if cancel {
                            app.axis_defs.rename_state = None;
                        }
                    }

                    // Collapsing body
                    state.show_body_unindented(ui, |ui| {
                        let axis = match app.axis_defs.definitions.get_mut(name) {
                            Some(a) => a,
                            None => return,
                        };

                        // Per-variant controls
                        match axis {
                            DerivedAxis::Vector { x, y, z } => {
                                ui.horizontal(|ui| {
                                    ui.label("x:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(x)
                                                .speed(EULER_SPEED)
                                                .fixed_decimals(EULER_DECIMALS),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("y:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(y)
                                                .speed(EULER_SPEED)
                                                .fixed_decimals(EULER_DECIMALS),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("z:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(z)
                                                .speed(EULER_SPEED)
                                                .fixed_decimals(EULER_DECIMALS),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::Euler { pitch, yaw } => {
                                ui.horizontal(|ui| {
                                    ui.label("Pitch:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(pitch)
                                                .range(0.0..=180.0)
                                                .speed(AXIS_ANGLE_SPEED)
                                                .fixed_decimals(AXIS_ANGLE_DECIMALS)
                                                .suffix("°"),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("Yaw:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(yaw)
                                                .range(-180.0..=180.0)
                                                .speed(AXIS_ANGLE_SPEED)
                                                .fixed_decimals(AXIS_ANGLE_DECIMALS)
                                                .suffix("°"),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::Copy { axis, invert } => {
                                ui.horizontal(|ui| {
                                    ui.label("Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("copy_axis_{}", name),
                                        axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                    if ui.checkbox(invert, "Invert").changed() {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::CrossProduct { a0, a1 } => {
                                ui.horizontal(|ui| {
                                    ui.label("A:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("cross_a0_{}", name),
                                        a0,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                    ui.label("B:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("cross_a1_{}", name),
                                        a1,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::Average { axes } => {
                                let mut to_remove = None;
                                for (i, a) in axes.iter_mut().enumerate() {
                                    ui.horizontal(|ui| {
                                        if axis_combo_box(
                                            ui,
                                            &format!("avg_{}_{}", name, i),
                                            a,
                                            &available,
                                        ) {
                                            changed = true;
                                        }
                                        if ui.small_button("🗑").clicked() {
                                            to_remove = Some(i);
                                        }
                                    });
                                }
                                if let Some(idx) = to_remove {
                                    axes.remove(idx);
                                    changed = true;
                                }
                                if ui.small_button("+ Add Axis").clicked() {
                                    axes.push("X".to_string());
                                    changed = true;
                                }
                            }
                            DerivedAxis::WillsEquation {
                                p,
                                q,
                                n_a,
                                n_b,
                                a_axis,
                                perpendicular_axis,
                                manual_axis_angle,
                                manual_axis_angle_deg,
                            } => {
                                // Manual axis angle toggle
                                ui.horizontal(|ui| {
                                    ui.label("Axis Angle:");
                                    if *manual_axis_angle {
                                        ui.horizontal(|ui| {
                                            if ui
                                                .add(
                                                    egui::DragValue::new(manual_axis_angle_deg)
                                                        .range(0.0..=180.0)
                                                        .speed(AXIS_ANGLE_SPEED)
                                                        .fixed_decimals(AXIS_ANGLE_DECIMALS)
                                                        .suffix("°"),
                                                )
                                                .changed()
                                            {
                                                changed = true;
                                            }
                                        });
                                    } else {
                                        // Show derived angle
                                        if let Some(ang) = derive_axis_angle(*n_a, *n_b, *p, *q) {
                                            ui.label(format!("{:.4}°", ang.to_degrees()));
                                        }
                                    }
                                    ui.separator();
                                    if ui.add(crate::gui::toggle(manual_axis_angle)).changed() {
                                        // When toggling on, populate from current p/q
                                        if *manual_axis_angle
                                            && let Some(ang) = derive_axis_angle(*n_a, *n_b, *p, *q)
                                        {
                                            *manual_axis_angle_deg =
                                                (ang.to_degrees() * 10000.0).round() / 10000.0;
                                        }
                                        changed = true;
                                    }
                                    ui.label("Manual Override");
                                });

                                ui.horizontal(|ui| {
                                    ui.label("nA:");
                                    if ui
                                        .add(egui::DragValue::new(n_a).range(2..=8).speed(0.05))
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("nB:");
                                    if ui
                                        .add(egui::DragValue::new(n_b).range(2..=8).speed(0.05))
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    if !*manual_axis_angle {
                                        ui.label("p:");
                                        if ui
                                            .add(egui::DragValue::new(p).range(1..=20).speed(0.02))
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                        ui.label("q:");
                                        if ui
                                            .add(egui::DragValue::new(q).range(2..=30).speed(0.02))
                                            .changed()
                                        {
                                            changed = true;
                                        }
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("A Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("wills_a_{}", name),
                                        a_axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                    ui.label("Perp Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("wills_perp_{}", name),
                                        perpendicular_axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                });
                            }
                            DerivedAxis::CircularPattern {
                                pattern_axis,
                                target_axis,
                                n,
                                angle_range_deg,
                            } => {
                                ui.horizontal(|ui| {
                                    ui.label("Pattern Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("circ_pat_{}", name),
                                        pattern_axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                    ui.label("Target Axis:");
                                    if axis_combo_box(
                                        ui,
                                        &format!("circ_tgt_{}", name),
                                        target_axis,
                                        &available,
                                    ) {
                                        changed = true;
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("n:");
                                    if ui
                                        .add(egui::DragValue::new(n).range(2..=100).speed(0.05))
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                    ui.label("Range:");
                                    if ui
                                        .add(
                                            egui::DragValue::new(angle_range_deg)
                                                .range(0.0..=360.0)
                                                .speed(0.5)
                                                .fixed_decimals(1)
                                                .suffix("°"),
                                        )
                                        .changed()
                                    {
                                        changed = true;
                                    }
                                });
                            }
                        }
                    });
                }
            });
        });

    if changed {
        app.axis_defs.resolve_all();
        // Bubble changes to puzzle params
        app.spawn_geometry_worker();
        if let Some(three) = &app.three {
            let axes = app.build_axes();
            let def_vecs = app.axis_defs.get_visible_vectors();
            three.update_axis_indicators(&axes, app.params.show_axes, &def_vecs);
        }
    }
}

/// Helper: axis reference combo box. Returns true if the value changed.
fn axis_combo_box(
    ui: &mut egui::Ui,
    id_salt: &str,
    selected: &mut String,
    available: &[String],
) -> bool {
    let mut changed = false;
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for name in available {
                if ui.selectable_value(selected, name.clone(), name).changed() {
                    changed = true;
                }
            }
        });
    changed
}

/// Rename all occurrences of `old_name` (and sub-indexed variants) in puzzle params.
fn rename_axis_in_puzzle_params(
    params: &mut crate::gui::PuzzleParams,
    old_name: &str,
    new_name: &str,
    suffixes: &[String],
) {
    for entry in &mut params.axes {
        if entry.axis_name == old_name {
            entry.axis_name = new_name.to_string();
        }
        for suffix in suffixes {
            let old_sub = format!("{}_{}", old_name, suffix);
            if entry.axis_name == old_sub {
                entry.axis_name = format!("{}_{}", new_name, suffix);
            }
        }
    }
}
