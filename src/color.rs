// --- Orbit Colors (direct from JS/CSS version) ---

#[rustfmt::skip]
pub const ORBIT_COLORS: &[(&str, [f32; 3])] = &[
    ("Red",             [0.86, 0.08, 0.24]),
    ("Green",           [0.13, 0.55, 0.13]),
    ("Blue",            [0.20, 0.40, 1.00]),
    ("Orange",          [1.00, 0.55, 0.00]),
    ("Purple",          [0.58, 0.00, 0.83]),
    ("Sky Blue",        [0.00, 0.75, 1.00]),
    ("Gold",            [1.00, 0.90, 0.10]),
    ("Light Pink",      [1.00, 0.46, 0.73]),
    ("Yellow-Green",    [0.60, 0.80, 0.20]),
    ("Coral",           [0.94, 0.50, 0.50]),
    ("Teal",            [0.00, 0.50, 0.50]),
    ("Brown",           [0.55, 0.27, 0.07]),
    ("Magenta",         [1.00, 0.08, 0.68]),
    ("Lavender",        [0.83, 0.66, 0.92]),
    ("Turquoise",       [0.30, 0.90, 0.85]),
    ("Indigo",          [0.40, 0.00, 0.90]),
    ("Tan",             [0.85, 0.75, 0.60]),
];

pub const SINGLETON_COLOR: (&str, [f32; 3]) = ("Gray", [0.41, 0.41, 0.41]);

pub fn axis_color(idx: usize) -> [f32; 3] {
    ORBIT_COLORS[idx % ORBIT_COLORS.len()].1
}

pub fn color32(c: &[f32; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
    )
}

pub fn color_to_hex(c: &[f32; 3]) -> u32 {
    let r = (c[0] * 255.0) as u32;
    let g = (c[1] * 255.0) as u32;
    let b = (c[2] * 255.0) as u32;
    (r << 16) | (g << 8) | b
}

pub const ARC_COLOR: u32 = 0xeeeeee;
pub const SPHERE_COLOR: u32 = 0x1a1a1a;

pub const BUILTIN_X_COLOR: u32 = 0xFF8080;
pub const BUILTIN_Y_COLOR: u32 = 0x80FF80;
pub const BUILTIN_Z_COLOR: u32 = 0x8080FF;

// https://stackoverflow.com/questions/3942878/how-to-decide-font-color-in-white-or-black-depending-on-background-color/3943023#3943023
pub fn get_contrast_color(r: u8, g: u8, b: u8) -> String {
    // Calculate relative luminance
    let luminance = (0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) / 255.0;

    // Return black or white based on luminance
    if luminance > 0.5 {
        "#000000".to_string()
    } else {
        "#ffffff".to_string()
    }
}
