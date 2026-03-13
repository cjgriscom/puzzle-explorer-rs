use crate::types::{AxisDefinition, AxisDefinitions, DerivedAxis};
use std::collections::{HashMap, HashSet};

use glam::{DAffine3, DVec3};
use puzzle_explorer_math::geometry::derive_axis_angle;

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
                    Err("Unexpected zero vector".to_string())
                } else {
                    Ok(vec![v.normalize()])
                }
            }
            DerivedAxis::Copy { axis, invert } => {
                let v = resolve_norm_reference(axis_map, axis)?;
                Ok(vec![if *invert { -v } else { v }])
            }
            DerivedAxis::CrossProduct { a0, a1 } => {
                let v0 = resolve_norm_reference(axis_map, a0)?;
                let v1 = resolve_norm_reference(axis_map, a1)?;
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
                    sum += resolve_norm_reference(axis_map, name)?;
                }
                if sum.length_squared() < 1e-12 {
                    Err("Average resulted in a zero vector".to_string())
                } else {
                    Ok(vec![sum.normalize()])
                }
            }
            DerivedAxis::CosineRule {
                p,
                q,
                n_a,
                n_b,
                a_axis,
                perpendicular_axis,
                manual_axis_angle_deg: manual_axis_angle,
            } => {
                let a_vec = resolve_norm_reference(axis_map, a_axis)?;
                let perp_vec = resolve_norm_reference(axis_map, perpendicular_axis)?;
                if a_vec.cross(perp_vec).length_squared() < 1e-12 {
                    return Err("A axis and perpendicular axis are parallel".to_string());
                }
                let angle = match manual_axis_angle {
                    Some(manual_axis_angle_deg) => manual_axis_angle_deg.to_radians(),
                    None => derive_axis_angle(*n_a, *n_b, *p, *q)
                        .ok_or_else(|| "Failed to derive axis angle".to_string())?,
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
                invert_range: invert,
            } => {
                if *n < 2 {
                    return Err("Expected n >= 2".to_string());
                }
                if *angle_range_deg < 0.0 || *angle_range_deg > 360.0 {
                    return Err("Expected angle range in [0, 360]".to_string());
                }
                let pattern_vec = resolve_norm_reference(axis_map, pattern_axis)?;
                let target_vec = resolve_norm_reference(axis_map, target_axis)?;
                let sign = if *invert { -1.0 } else { 1.0 };
                let angle_range_rad = angle_range_deg.to_radians() * sign;
                let mut results = Vec::with_capacity(*n as usize);
                for i in 0..*n {
                    let angle = angle_range_rad * i as f64 / *n as f64;
                    let rotation = DAffine3::from_axis_angle(pattern_vec, angle);
                    results.push(rotation.transform_vector3(target_vec).normalize());
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
            DerivedAxis::CosineRule {
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
            DerivedAxis::CosineRule { .. } => vec!["A".into(), "B".into()],
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
            DerivedAxis::CosineRule { .. } => 5,
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
            5 => DerivedAxis::CosineRule {
                p: 1,
                q: 5,
                n_a: 3,
                n_b: 3,
                a_axis: "X".to_string(),
                perpendicular_axis: "Y".to_string(),
                manual_axis_angle_deg: None,
            },
            6 => DerivedAxis::CircularPattern {
                pattern_axis: "Z".to_string(),
                target_axis: "X".to_string(),
                n: 3,
                angle_range_deg: 360.0,
                invert_range: false,
            },
            _ => DerivedAxis::Vector {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        }
    }

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
            DerivedAxis::CosineRule {
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

fn resolve_norm_reference(axis_map: &HashMap<String, DVec3>, axis: &str) -> Result<DVec3, String> {
    Ok(axis_map
        .get(axis)
        .ok_or_else(|| format!("Referenced axis '{}' not found", axis))?
        .normalize())
}

impl AxisDefinitions {
    pub fn get_definition(&self, name: &str) -> Option<&DerivedAxis> {
        self.definitions
            .iter()
            .find(|d| d.name == name)
            .map(|d| &d.axis)
    }

    pub fn get_definition_mut(&mut self, name: &str) -> Option<&mut DerivedAxis> {
        self.definitions
            .iter_mut()
            .find(|d| d.name == name)
            .map(|d| &mut d.axis)
    }

    pub fn contains_definition(&self, name: &str) -> bool {
        self.definitions.iter().any(|d| d.name == name)
    }

    pub fn definitions_keys(&self) -> Vec<String> {
        self.definitions.iter().map(|d| d.name.clone()).collect()
    }

    /// Resolve axes with cycle detection
    pub fn resolve_all(&mut self) {
        let mut axis_map: HashMap<String, DVec3> = HashMap::new();
        axis_map.insert("X".to_string(), DVec3::X);
        axis_map.insert("Y".to_string(), DVec3::Y);
        axis_map.insert("Z".to_string(), DVec3::Z);

        self.resolved.clear();

        // Topological sort via Kahn's algorithm
        let names: Vec<String> = self.definitions_keys();
        let builtins: HashSet<String> = axis_map.iter().map(|s| s.0.to_string()).collect();

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for name in &names {
            in_degree.insert(name.clone(), 0);
        }

        for d in &self.definitions {
            let mut base_deps: HashSet<String> = HashSet::new();
            for dep in d.axis.dependencies() {
                // TODO Strip sub-index suffix... ideally this should resolve to the base in a better way
                let base = strip_sub_index(&dep);
                if !builtins.contains(&base) && self.contains_definition(&base) {
                    base_deps.insert(base);
                }
            }
            in_degree.insert(d.name.clone(), base_deps.len());
        }

        let mut queue: Vec<String> = Vec::new();
        for (name, deg) in &in_degree {
            if *deg == 0 {
                queue.push(name.clone());
            }
        }

        let mut resolved_order: Vec<String> = Vec::new();
        while let Some(name) = queue.pop() {
            resolved_order.push(name.clone());
            for other_d in &self.definitions {
                if in_degree.get(&other_d.name).copied().unwrap_or(0) == 0 {
                    continue;
                }
                let deps = other_d.axis.dependencies();
                let depends_on_name = deps.iter().any(|d| {
                    let base = strip_sub_index(d);
                    base == name
                });
                if depends_on_name {
                    let deg = in_degree.get_mut(&other_d.name).unwrap();
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push(other_d.name.clone());
                    }
                }
            }
        }

        let resolved_set: HashSet<&String> = resolved_order.iter().collect();
        for name in &names {
            if !resolved_set.contains(name) {
                self.resolved
                    .insert(name.clone(), Err("Circular axis reference".to_string()));
            }
        }

        for name in &resolved_order {
            let axis = match self.get_definition(name) {
                Some(a) => a,
                None => continue,
            };
            let result = axis.resultant_vectors(&axis_map);
            if let Ok(vecs) = &result {
                if vecs.len() == 1 {
                    axis_map.insert(name.clone(), vecs[0]);
                } else {
                    let suffixes = axis.output_suffixes();
                    for (i, suffix) in suffixes.iter().enumerate() {
                        axis_map.insert(format!("{}_{}", name, suffix), vecs[i]);
                    }
                }
            }
            self.resolved.insert(name.clone(), result);
        }
    }

    /// Returns all axis names available for reference in dropdown lists
    pub fn available_axis_names(&self) -> Vec<String> {
        let mut names = vec!["X".to_string(), "Y".to_string(), "Z".to_string()];
        for d in &self.definitions {
            match self.resolved.get(&d.name) {
                Some(Ok(vecs)) if vecs.len() > 1 => {
                    for suffix in d.axis.output_suffixes() {
                        names.push(format!("{}_{}", d.name, suffix));
                    }
                }
                _ => {
                    names.push(d.name.clone());
                }
            }
        }
        names
    }

    /// For a given axis name referencing a CosineRule definition,
    /// return the matching n value (n_a for _A, n_b for _B)
    pub fn get_cosine_rule_n_for_axis(&self, axis_name: &str) -> Option<u32> {
        if let Some(pos) = axis_name.rfind('_') {
            let base = &axis_name[..pos];
            let suffix = &axis_name[pos + 1..];
            if let Some(DerivedAxis::CosineRule { n_a, n_b, .. }) = self.get_definition(base) {
                return match suffix {
                    "A" => Some(*n_a),
                    "B" => Some(*n_b),
                    _ => None,
                };
            }
        }
        None
    }

    /// Look up a single resolved vector by name, handling builtins and
    /// sub-indexed names
    pub fn get_resolved_vector(&self, name: &str) -> Option<DVec3> {
        if name.is_empty() {
            return None;
        }
        match name {
            "X" => return Some(DVec3::X),
            "Y" => return Some(DVec3::Y),
            "Z" => return Some(DVec3::Z),
            _ => {}
        };
        if let Some(Ok(vecs)) = self.resolved.get(name)
            && vecs.len() == 1
        {
            return Some(vecs[0]);
        }
        if let Some(pos) = name.rfind('_') {
            let base = &name[..pos];
            let suffix = &name[pos + 1..];
            if let Some(Ok(vecs)) = self.resolved.get(base)
                && let Some(def) = self.get_definition(base)
            {
                for (i, s) in def.output_suffixes().iter().enumerate() {
                    if suffix == s {
                        return Some(vecs[i]);
                    }
                }
            }
        }
        None
    }

    /// Collect all resolved vectors from visible axis definitions for
    /// rendering construction axes
    pub fn get_visible_vectors(&self) -> Vec<DVec3> {
        let mut vecs = Vec::new();
        for d in &self.definitions {
            if !self.visible_axes.contains(&d.name) {
                continue;
            }
            if let Some(Ok(result_vecs)) = self.resolved.get(&d.name) {
                for v in result_vecs {
                    vecs.push(*v);
                }
            }
        }
        vecs
    }

    /// Collect visible builtin axis indicators as (vector, color_hex) pairs
    pub fn get_visible_builtin_axes(&self) -> Vec<(DVec3, u32)> {
        use crate::color::{BUILTIN_X_COLOR, BUILTIN_Y_COLOR, BUILTIN_Z_COLOR};
        let mut result = Vec::new();
        if self.visible_axes.contains("X") {
            result.push((DVec3::X, BUILTIN_X_COLOR));
        }
        if self.visible_axes.contains("Y") {
            result.push((DVec3::Y, BUILTIN_Y_COLOR));
        }
        if self.visible_axes.contains("Z") {
            result.push((DVec3::Z, BUILTIN_Z_COLOR));
        }
        result
    }

    /// Generate a unique name at (max N)+1 like "Axis 1"
    pub fn generate_name(&self) -> String {
        let mut max_n = 0u32;
        for name in self.definitions_keys() {
            if let Some(rest) = name.strip_prefix("Axis ")
                && let Ok(n) = rest.parse::<u32>()
            {
                max_n = max_n.max(n);
            }
        }
        format!("Axis {}", max_n + 1)
    }

    pub(crate) fn get_builtin_axis_names(&self) -> Vec<String> {
        vec!["X".to_string(), "Y".to_string(), "Z".to_string()]
    }

    pub(crate) fn make_unique_name(&self, desired: &str) -> String {
        let builtins: HashSet<String> = self.get_builtin_axis_names().into_iter().collect();
        if !builtins.contains(desired) && !self.contains_definition(desired) {
            return desired.to_string();
        }
        let mut i = 2u32;
        loop {
            let candidate = format!("{} {}", desired, i);
            if !builtins.contains(candidate.as_str()) && !self.contains_definition(&candidate) {
                return candidate;
            }
            i += 1;
        }
    }

    pub(crate) fn find_dependents(&self, target_name: &str) -> Vec<String> {
        let mut result = Vec::new();
        for d in &self.definitions {
            if d.name == target_name {
                continue;
            }
            let deps = d.axis.dependencies();
            if deps.iter().any(|d| {
                let base = strip_sub_index(d);
                base == target_name
            }) {
                result.push(d.name.clone());
            }
        }
        result
    }

    pub(crate) fn do_rename(&mut self, old_name: &str, new_name: &str) {
        if old_name == new_name {
            return;
        }
        let mut new_defs = Vec::new();
        for d in self.definitions.drain(..) {
            if d.name == old_name {
                new_defs.push(AxisDefinition {
                    name: new_name.to_string(),
                    axis: d.axis.clone(),
                });
            } else {
                new_defs.push(d);
            }
        }
        self.definitions = new_defs;
        if self.visible_axes.remove(old_name) {
            self.visible_axes.insert(new_name.to_string());
        }
        let suffixes = self
            .get_definition(new_name)
            .map(|a| a.output_suffixes())
            .unwrap_or_default();
        for key in self.definitions_keys() {
            if let Some(axis) = self.get_definition_mut(&key) {
                axis.rename_references(old_name, new_name);
                for suffix in &suffixes {
                    let old_sub = format!("{}_{}", old_name, suffix);
                    let new_sub = format!("{}_{}", new_name, suffix);
                    axis.rename_references(&old_sub, &new_sub);
                }
            }
        }
    }

    pub(crate) fn delete(&mut self, name: &str) {
        self.definitions.retain(|d| d.name != name);
        self.visible_axes.remove(name);
    }
}

/// Strip a sub-index suffix
/// (e.g. "Trapentrix_A" -> "Trapentrix", "X" -> "X", "Pattern_1" -> "Pattern")
/// TODO don't like this
fn strip_sub_index(name: &str) -> String {
    if let Some(idx) = name.rfind('_') {
        let suffix = &name[idx + 1..];
        if suffix.parse::<u32>().is_ok() || suffix == "A" || suffix == "B" {
            return name[..idx].to_string();
        }
    }
    name.to_string()
}
