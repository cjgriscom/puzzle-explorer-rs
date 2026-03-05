use wasm_bindgen::prelude::*;

/// Recursively dispose all geometry, material, and texture resources in a group's
/// children, then clear the group. This prevents Three.js GPU memory leaks.
pub fn dispose_group_children(group: &Group) {
    let func = js_sys::Function::new_with_args(
        "group",
        r#"
        function disposeNode(node) {
            if (node.children) {
                for (let i = node.children.length - 1; i >= 0; i--) {
                    disposeNode(node.children[i]);
                }
            }
            if (node.geometry) node.geometry.dispose();
            if (node.material) {
                if (node.material.map) node.material.map.dispose();
                node.material.dispose();
            }
        }
        for (let i = group.children.length - 1; i >= 0; i--) {
            disposeNode(group.children[i]);
        }
        group.clear();
        "#,
    );
    let _ = func.call1(&JsValue::NULL, group);
}

#[wasm_bindgen]
extern "C" {
    // --- Vector3 ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type Vector3;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(x: f64, y: f64, z: f64) -> Vector3;

    #[wasm_bindgen(method, structural, getter, js_namespace = THREE)]
    pub fn x(this: &Vector3) -> f64;
    #[wasm_bindgen(method, structural, getter, js_namespace = THREE)]
    pub fn y(this: &Vector3) -> f64;
    #[wasm_bindgen(method, structural, getter, js_namespace = THREE)]
    pub fn z(this: &Vector3) -> f64;
    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn set(this: &Vector3, x: f64, y: f64, z: f64);

    // --- Quaternion ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type Quaternion;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new() -> Quaternion;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn setFromAxisAngle(this: &Quaternion, axis: &Vector3, angle: f64) -> Quaternion;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn premultiply(this: &Quaternion, q: &Quaternion) -> Quaternion;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn normalize(this: &Quaternion) -> Quaternion;

    // --- Object3D ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type Object3D;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn rotateX(this: &Object3D, angle: f64);

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn rotateY(this: &Object3D, angle: f64);

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn add(this: &Object3D, object: &Object3D);

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn remove(this: &Object3D, object: &Object3D);

    #[wasm_bindgen(method, structural, setter, js_namespace = THREE)]
    pub fn set_visible(this: &Object3D, visible: bool);

    #[wasm_bindgen(method, structural, getter, js_namespace = THREE)]
    pub fn position(this: &Object3D) -> Vector3;

    #[wasm_bindgen(method, structural, getter, js_namespace = THREE)]
    pub fn quaternion(this: &Object3D) -> Quaternion;

    #[wasm_bindgen(method, structural, getter, js_namespace = THREE)]
    pub fn scale(this: &Object3D) -> Vector3;

    // --- Scene ---
    #[wasm_bindgen(js_namespace = THREE, extends = Object3D)]
    pub type Scene;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new() -> Scene;

    // --- Group ---
    #[wasm_bindgen(js_namespace = THREE, extends = Object3D)]
    pub type Group;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new() -> Group;

    // --- PerspectiveCamera ---
    #[wasm_bindgen(js_namespace = THREE, extends = Object3D)]
    pub type PerspectiveCamera;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(fov: f64, aspect: f64, near: f64, far: f64) -> PerspectiveCamera;

    #[wasm_bindgen(method, structural, getter, js_namespace = THREE, js_name = "position")]
    pub fn cam_position(this: &PerspectiveCamera) -> Vector3;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn updateProjectionMatrix(this: &PerspectiveCamera);

    #[wasm_bindgen(method, structural, setter, js_namespace = THREE)]
    pub fn set_aspect(this: &PerspectiveCamera, aspect: f64);

    // --- WebGLRenderer ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type WebGLRenderer;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(options: &js_sys::Object) -> WebGLRenderer;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn setSize(this: &WebGLRenderer, width: f64, height: f64);

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn setPixelRatio(this: &WebGLRenderer, ratio: f64);

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn setClearColor(this: &WebGLRenderer, color: u32);

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn render(this: &WebGLRenderer, scene: &Scene, camera: &PerspectiveCamera);

    // --- BufferGeometry / BufferAttribute ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type BufferGeometry;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new() -> BufferGeometry;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn setAttribute(this: &BufferGeometry, name: &str, attribute: &BufferAttribute);

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn dispose(this: &BufferGeometry);

    #[wasm_bindgen(js_namespace = THREE)]
    pub type BufferAttribute;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(array: &js_sys::Float32Array, itemSize: i32) -> BufferAttribute;

    // --- Line Materials ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type LineBasicMaterial;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(parameters: &js_sys::Object) -> LineBasicMaterial;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn dispose(this: &LineBasicMaterial);

    // --- Texture / CanvasTexture ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type Texture;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn dispose(this: &Texture);

    #[wasm_bindgen(js_namespace = THREE, extends = Texture)]
    pub type CanvasTexture;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(canvas: &web_sys::HtmlCanvasElement) -> CanvasTexture;

    // --- SpriteMaterial ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type SpriteMaterial;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(parameters: &js_sys::Object) -> SpriteMaterial;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn dispose(this: &SpriteMaterial);

    // --- Sprite ---
    #[wasm_bindgen(js_namespace = THREE, extends = Object3D)]
    pub type Sprite;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(material: &SpriteMaterial) -> Sprite;

    // --- Line / LineLoop ---
    #[wasm_bindgen(js_namespace = THREE, extends = Object3D)]
    pub type Line;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(geometry: &BufferGeometry, material: &LineBasicMaterial) -> Line;

    #[wasm_bindgen(js_namespace = THREE, extends = Line)]
    pub type LineLoop;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(geometry: &BufferGeometry, material: &LineBasicMaterial) -> LineLoop;

    // --- SphereGeometry / Mesh ---
    #[wasm_bindgen(js_namespace = THREE)]
    pub type SphereGeometry;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(radius: f64, widthSegments: i32, heightSegments: i32) -> SphereGeometry;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn dispose(this: &SphereGeometry);

    #[wasm_bindgen(js_namespace = THREE)]
    pub type MeshBasicMaterial;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(parameters: &js_sys::Object) -> MeshBasicMaterial;

    #[wasm_bindgen(method, structural, js_namespace = THREE)]
    pub fn dispose(this: &MeshBasicMaterial);

    #[wasm_bindgen(js_namespace = THREE, extends = Object3D)]
    pub type Mesh;

    #[wasm_bindgen(constructor, js_namespace = THREE)]
    pub fn new(geometry: &SphereGeometry, material: &MeshBasicMaterial) -> Mesh;
}
