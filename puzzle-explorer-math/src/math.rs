pub use std::f64::consts::PI;
pub const TAU: f64 = 2.0 * PI;

pub fn norm_ang(a: f64) -> f64 {
    let a = a % TAU;
    if a < 0.0 { a + TAU } else { a }
}
