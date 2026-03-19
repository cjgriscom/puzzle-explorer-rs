use glam::DVec3;

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

impl Circle {
    pub fn new(pole: DVec3, colat: f64) -> Self {
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

    pub fn circ_pt(&self, theta: f64) -> DVec3 {
        let sc = self.colat.sin();
        let cc = self.colat.cos();
        let ct = theta.cos();
        let st = theta.sin();
        (self.u * ct + self.w * st) * sc + self.pole * cc
    }

    pub fn pt_ang(&self, p: DVec3) -> f64 {
        let cc = self.colat.cos();
        let d = p - self.pole * cc;
        d.dot(self.w).atan2(d.dot(self.u))
    }

    pub fn sample_arc(&self, start: f64, length: f64, npts: usize) -> Vec<[f32; 3]> {
        let mut pts = Vec::with_capacity(npts + 1);
        for i in 0..=npts {
            let theta = start + length * (i as f64) / (npts as f64);
            let p = self.circ_pt(theta);
            pts.push([(p.x) as f32, (p.y) as f32, (p.z) as f32]);
        }
        pts
    }

    // Arc sampling

    pub fn arc_pt_at_ang(&self, arc: &Arc, da: f64) -> DVec3 {
        let ang = crate::math::norm_ang(arc.s + da);
        self.circ_pt(ang)
    }

    #[cfg(test)]
    pub fn arc_avg(&self, arc: &Arc, da0: f64, da1: f64, n_samples: usize) -> DVec3 {
        let mut sum = DVec3::ZERO;

        for i in 0..n_samples {
            let t = da0 + (da1 - da0) * (i as f64 + 0.5) / (n_samples as f64);
            let ang = crate::math::norm_ang(arc.s + t);
            sum += self.circ_pt(ang);
        }
        sum / (n_samples as f64)
    }

    pub fn arc_integral(&self, arc: &Arc, a: f64, b: f64) -> DVec3 {
        let a = a + arc.s;
        let b = b + arc.s;
        self.pole * self.colat.cos()
            + self.colat.sin() * (self.w * (a.cos() - b.cos()) + self.u * (b.sin() - a.sin()))
                / (b - a)
    }

    // Intersection

    pub fn intersect(&self, c2: &Circle) -> Vec<DVec3> {
        let n1 = self.pole;
        let n2 = c2.pole;
        let d1 = self.colat.cos();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{PI, TAU};

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
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
            let avg = c_simple.arc_avg(&arc_full, *da0, *da1, 10000);
            let integral = c_simple.arc_integral(&arc_full, *da0, *da1);
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
