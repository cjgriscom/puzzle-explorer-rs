//! Built-in puzzle examples

pub const EXAMPLES: &[Example] = &[
    Example::new("Trapentrix", include_bytes!("examples/Trapentrix.yml")),
    Example::new(
        "Photonic Crystal",
        include_bytes!("examples/Photonic Crystal.yml"),
    ),
    Example::new("Biaxe", include_bytes!("examples/Biaxe.yml")),
    Example::new("Undectrix", include_bytes!("examples/Undectrix.yml")),
    Example::new("Megaminx", include_bytes!("examples/Megaminx.yml")),
    Example::new(
        "Skyglobe Ultimate",
        include_bytes!("examples/Skyglobe Ultimate.yml"),
    ),
];

pub struct Example {
    pub name: &'static str,
    pub yaml: &'static [u8],
}

impl Example {
    const fn new(name: &'static str, yaml: &'static [u8]) -> Self {
        Self { name, yaml }
    }

    pub fn to_yaml(&self) -> Option<String> {
        String::from_utf8(self.yaml.to_vec()).ok()
    }
}

pub fn default_example() -> &'static Example {
    &EXAMPLES[1]
}
