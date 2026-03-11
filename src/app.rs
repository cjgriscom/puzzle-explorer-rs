use egui::Visuals;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, MessageEvent, Worker, WorkerOptions, window};

use puzzle_explorer_math::circle::Circle;
use puzzle_explorer_math::math::TAU;

use crate::color::{ORBIT_COLORS, SINGLETON_COLOR, color_to_hex};
use crate::dreadnaut::DreadnautManager;
use crate::gap::GapManager;
use crate::gui::axis_definitions::AxisDefinitions;
use crate::gui::{OrbitAnalysisState, PuzzleParams};
use crate::input::{CameraInputState, handle_camera_input};
use crate::puzzle::{AxisDef, GeometryParams, GeometryResult, OrbitParams, OrbitResult, PolyLine};
use crate::three::{
    BufferAttribute, BufferGeometry, CanvasTexture, Group, Line, LineBasicMaterial, LineLoop, Mesh,
    MeshBasicMaterial, PerspectiveCamera, Quaternion, Scene, SphereGeometry, Sprite,
    SpriteMaterial, Vector3, WebGLRenderer,
};
use crate::worker::{WorkerMessage, WorkerResponse};

// --- Constants ---

const R: f64 = 1.0; // Radius of sphere
const DISP_R: f64 = R * 1.004; // Dist of arcs from sphere
const LABEL_R: f64 = R * 1.04; // Dist. of orbit labels from sphere

// --- Animation State ---

pub struct AnimState {
    axis: [f64; 3],
    target_angle: f64,
    start_time: f64,
    duration: f64,
    static_group: Group,
    rot_group: Group,
}

fn ease_in_out(t: f64) -> f64 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
    }
}

// --- ThreeState ---

pub struct ThreeState {
    scene: Scene,
    camera: PerspectiveCamera,
    renderer: WebGLRenderer,
    group: Group,
    cut_group: Group,
    face_group: Group,
    axis_group: Group,
    cam_dist: f64,
    pan_screen: [f64; 2],
    base_group_y: f64,
    viewport_size: [f64; 2],
}

impl ThreeState {
    pub fn new(three_canvas_id: String) -> Option<Self> {
        let window = window()?;
        let document = window.document()?;

        let canvas_el = document.get_element_by_id(&three_canvas_id)?;
        let canvas = canvas_el.dyn_into::<HtmlCanvasElement>().ok()?;

        let width = window.inner_width().ok()?.as_f64()?;
        let height = window.inner_height().ok()?.as_f64()?;

        let scene = Scene::new();
        let camera = PerspectiveCamera::new(40.0, width / height, 0.1, 100.0);
        let cam_dist = 5.0;
        camera.cam_position().set(0.0, 0.0, cam_dist);

        let options = js_sys::Object::new();
        js_sys::Reflect::set(&options, &"canvas".into(), &canvas).ok()?;
        js_sys::Reflect::set(&options, &"antialias".into(), &true.into()).ok()?;

        let renderer = WebGLRenderer::new(&options);
        renderer.setSize(width, height);
        renderer.setPixelRatio(window.device_pixel_ratio().min(2.0));
        renderer.setClearColor(0x222222);

        let group = Group::new();

        // Render sphere
        let sphere_geo = SphereGeometry::new(R, 64, 48);
        let mat_params = js_sys::Object::new();
        js_sys::Reflect::set(
            &mat_params,
            &"color".into(),
            &crate::color::SPHERE_COLOR.into(),
        )
        .ok()?;
        js_sys::Reflect::set(&mat_params, &"polygonOffset".into(), &true.into()).ok()?;
        js_sys::Reflect::set(&mat_params, &"polygonOffsetFactor".into(), &1.into()).ok()?;
        js_sys::Reflect::set(&mat_params, &"polygonOffsetUnits".into(), &1.into()).ok()?;
        let sphere_mat = MeshBasicMaterial::new(&mat_params);
        let sphere = Mesh::new(&sphere_geo, &sphere_mat);
        group.add(&sphere);

        let cut_group = Group::new();
        group.add(&cut_group);

        let face_group = Group::new();
        group.add(&face_group);

        let axis_group = Group::new();
        group.add(&axis_group);

        group.rotateX(0.35);
        group.rotateY(-0.5);
        let base_group_y = -0.3;
        group.position().set(0.0, base_group_y, 0.0);
        scene.add(&group);

        Some(Self {
            scene,
            camera,
            renderer,
            group,
            cut_group,
            face_group,
            axis_group,
            cam_dist,
            pan_screen: [0.0, 0.0],
            base_group_y,
            viewport_size: [width, height],
        })
    }

    pub fn render(&self) {
        self.renderer.render(&self.scene, &self.camera);
    }

    pub fn sync_resize(&mut self) {
        let Some(window) = window() else {
            return;
        };
        let Some(width) = window.inner_width().ok().and_then(|v| v.as_f64()) else {
            return;
        };
        let Some(height) = window.inner_height().ok().and_then(|v| v.as_f64()) else {
            return;
        };
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        if (width - self.viewport_size[0]).abs() < 0.5
            && (height - self.viewport_size[1]).abs() < 0.5
        {
            return;
        }

        self.viewport_size = [width, height];
        self.renderer.setSize(width, height);
        self.camera.set_aspect(width / height);
        self.camera.updateProjectionMatrix();
    }

    pub fn rotate_drag(&self, dx: f64, dy: f64) {
        let q = self.group.quaternion();
        let qy = Quaternion::new();
        qy.setFromAxisAngle(&Vector3::new(0.0, 1.0, 0.0), dx);
        let qx = Quaternion::new();
        qx.setFromAxisAngle(&Vector3::new(1.0, 0.0, 0.0), dy);
        q.premultiply(&qy);
        q.premultiply(&qx);
        q.normalize();
    }

    pub fn zoom(&mut self, scroll_y: f64) {
        let factor = if scroll_y > 0.0 { 0.92 } else { 1.08 };
        self.cam_dist = (self.cam_dist * factor).clamp(1.5, 20.0);
        self.apply_view_transform();
    }

    pub fn zoom_by_scale(&mut self, scale_delta: f64, sensitivity: f64) {
        if scale_delta <= 0.0 {
            return;
        }
        let factor = scale_delta.powf(-sensitivity);
        self.cam_dist = (self.cam_dist * factor).clamp(1.5, 20.0);
        self.apply_view_transform();
    }

    pub fn pan_drag(&mut self, dx: f64, dy: f64, viewport_size: [f32; 2]) {
        let denom = (viewport_size[0].min(viewport_size[1]) as f64).max(1.0);
        let pan_scale = 2.0 / denom;
        self.pan_screen[0] += dx * pan_scale;
        self.pan_screen[1] -= dy * pan_scale;
        self.apply_view_transform();
    }

    fn apply_view_transform(&self) {
        // Scale world-space pan by camera distance so wheel zoom keeps the sphere centered
        // at its current on-screen position instead of drifting toward canvas center.
        let x = self.pan_screen[0] * self.cam_dist;
        let y = self.base_group_y + self.pan_screen[1] * self.cam_dist;
        self.group.position().set(x, y, 0.0);
        self.camera.cam_position().set(0.0, 0.0, self.cam_dist);
    }

    pub fn update_geometry(&self, result: &GeometryResult) {
        crate::three::dispose_group_children(&self.cut_group);
        for poly_line in &result.lines {
            self.add_line_to_group(
                &self.cut_group,
                &poly_line.points,
                DISP_R as f32,
                poly_line.is_loop,
                crate::color::ARC_COLOR,
            );
        }
    }

    pub fn update_axis_indicators(
        &self,
        axes: &[Option<crate::puzzle::AxisDef>],
        puzzle_axes_visible: bool,
        def_vectors: &[glam::DVec3],
        builtin_axes: &[(glam::DVec3, u32)],
    ) {
        crate::three::dispose_group_children(&self.axis_group);
        let len = DISP_R as f32 * 1.3;
        // Render visible definition axes in grey
        let grey: u32 = 0x888888;
        for v in def_vectors {
            let d = v.normalize();
            let points = [
                [0.0, 0.0, 0.0],
                [d.x as f32 * len, d.y as f32 * len, d.z as f32 * len],
            ];
            self.add_line_to_group(&self.axis_group, &points, 1.0, false, grey);
        }
        // Render visible builtin reference axes in their designated colors
        for (v, color) in builtin_axes {
            let d = v.normalize();
            let points = [
                [0.0, 0.0, 0.0],
                [d.x as f32 * len, d.y as f32 * len, d.z as f32 * len],
            ];
            self.add_line_to_group(&self.axis_group, &points, 1.0, false, *color);
        }
        if !puzzle_axes_visible {
            return;
        }
        // Render puzzle axes in color
        for (i, axis) in axes.iter().enumerate() {
            let axis = match axis {
                Some(a) => a,
                None => continue,
            };
            let color = crate::color::color_to_hex(&ORBIT_COLORS[i % ORBIT_COLORS.len()].1);
            let d = axis.direction;
            let points = [
                [0.0, 0.0, 0.0],
                [d[0] as f32 * len, d[1] as f32 * len, d[2] as f32 * len],
            ];
            self.add_line_to_group(&self.axis_group, &points, 1.0, false, color);
        }
    }

    fn add_line_to_group(
        &self,
        grp: &Group,
        points: &[[f32; 3]],
        mul: f32,
        is_loop: bool,
        color: u32,
    ) {
        let geometry = BufferGeometry::new();
        let mut flat = Vec::with_capacity(points.len() * 3);
        for p in points {
            flat.push(p[0] * mul);
            flat.push(p[1] * mul);
            flat.push(p[2] * mul);
        }
        let float_array = js_sys::Float32Array::from(flat.as_slice());
        let pos_attr = BufferAttribute::new(&float_array, 3);
        geometry.setAttribute("position", &pos_attr);
        let mat_params = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&mat_params, &"color".into(), &color.into());
        let material = LineBasicMaterial::new(&mat_params);
        if is_loop {
            let line = LineLoop::new(&geometry, &material);
            grp.add(&line);
        } else {
            let line = Line::new(&geometry, &material);
            grp.add(&line);
        }
    }

    pub fn update_face_dots(&self, orbit_result: &OrbitResult, number_pieces: bool) {
        crate::three::dispose_group_children(&self.face_group);

        let n_orbits = orbit_result.orbit_count;
        // Build orbit_index -> color mapping
        let mut orbit_is_singleton = vec![false; n_orbits];
        let mut orbit_color_idx = vec![0usize; n_orbits];
        {
            // Count members per orbit
            let mut counts = vec![0usize; n_orbits];
            for &oi in &orbit_result.face_orbit_indices {
                if let Some(oi) = oi {
                    counts[oi] += 1
                }
            }
            let mut ci = 0;
            for oi in 0..n_orbits {
                if counts[oi] <= 1 {
                    orbit_is_singleton[oi] = true;
                } else {
                    orbit_color_idx[oi] = ci;
                    ci += 1;
                }
            }
        }

        let dot_geo = SphereGeometry::new(0.038, 12, 12);
        for (fi, pos) in orbit_result.face_positions.iter().enumerate() {
            let oi = match orbit_result.face_orbit_indices[fi] {
                Some(oi) => oi,
                None => continue,
            };
            let color = if orbit_is_singleton[oi] {
                SINGLETON_COLOR.1
            } else {
                ORBIT_COLORS[orbit_color_idx[oi] % ORBIT_COLORS.len()].1
            };

            if number_pieces {
                let sprite = self.create_label(&(fi + 1).to_string(), &color);
                sprite
                    .position()
                    .set(pos[0] as f64, pos[1] as f64, pos[2] as f64);
                // Move it out slightly to avoid depth fighting if it's right on the surface
                let center_norm =
                    glam::DVec3::new(pos[0] as f64, pos[1] as f64, pos[2] as f64).normalize();
                let label_pos = center_norm * LABEL_R;
                sprite.position().set(label_pos.x, label_pos.y, label_pos.z);
                self.face_group.add(&sprite);
            } else {
                let mat_params = js_sys::Object::new();
                let _ = js_sys::Reflect::set(
                    &mat_params,
                    &"color".into(),
                    &color_to_hex(&color).into(),
                );
                let mat = MeshBasicMaterial::new(&mat_params);
                let mesh = Mesh::new(&dot_geo, &mat);
                mesh.position()
                    .set(pos[0] as f64, pos[1] as f64, pos[2] as f64);
                self.face_group.add(&mesh);
            }
        }
    }

    fn create_label(&self, text: &str, color: &[f32; 3]) -> Sprite {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .ok()
            .unwrap();
        canvas.set_width(64);
        canvas.set_height(64);
        let ctx = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .ok()
            .unwrap();

        let r = (color[0] * 255.0) as u8;
        let g = (color[1] * 255.0) as u8;
        let b = (color[2] * 255.0) as u8;
        ctx.set_fill_style_str(&format!("rgb({}, {}, {})", r, g, b));
        ctx.begin_path();
        ctx.arc(32.0, 32.0, 28.0, 0.0, std::f64::consts::TAU)
            .unwrap();
        ctx.fill();

        let contrast_color = crate::color::get_contrast_color(r, g, b);
        ctx.set_fill_style_str(&contrast_color);
        ctx.set_font("bold 32px Courier New");
        ctx.set_text_align("center");
        ctx.set_text_baseline("middle");
        let _ = ctx.fill_text(text, 32.0, 34.0);

        let texture = CanvasTexture::new(&canvas);
        let mat_params = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&mat_params, &"map".into(), &texture);
        let material = SpriteMaterial::new(&mat_params);
        let sprite = Sprite::new(&material);
        sprite.scale().set(0.12, 0.12, 1.0);
        sprite
    }

    pub fn clear_face_dots(&self) {
        crate::three::dispose_group_children(&self.face_group);
    }

    /// Build the two animation groups by splitting stored arc points along the cap boundary.
    pub fn build_animation_groups(
        &self,
        lines: &[PolyLine],
        axis: [f64; 3],
        cos_colat: f64,
        boundary_circle: Option<&Circle>,
    ) -> (Group, Group) {
        let static_grp = Group::new();
        let rot_grp = Group::new();
        let eps = 1e-4;

        // Boundary circle
        if let Some(circ) = boundary_circle {
            let pts = circ.sample_arc(0.0, TAU, 128);
            self.add_line_to_group(
                &static_grp,
                &pts,
                DISP_R as f32,
                true,
                crate::color::ARC_COLOR,
            );
        }

        let pt_dot = |p: &[f32; 3]| -> f64 {
            (p[0] as f64 * axis[0] + p[1] as f64 * axis[1] + p[2] as f64 * axis[2]) / DISP_R
        };

        for poly_line in lines {
            let mut pts = poly_line.points.clone();
            if poly_line.is_loop && !pts.is_empty() {
                pts.push(pts[0]);
            }
            if pts.len() < 2 {
                continue;
            }

            let mut runs: Vec<(Vec<[f32; 3]>, bool)> = Vec::new();
            let mut cur_pts = vec![pts[0]];
            let mut cur_inside = pt_dot(&pts[0]) > cos_colat + eps;

            for j in 1..pts.len() {
                let d = pt_dot(&pts[j]);
                let is_in = d > cos_colat + eps;
                if is_in != cur_inside {
                    let prev_d = pt_dot(&pts[j - 1]);
                    let t = ((cos_colat - prev_d) / (d - prev_d)).clamp(0.001, 0.999) as f32;
                    let bp = lerp_normalize(&pts[j - 1], &pts[j], t);
                    cur_pts.push(bp);
                    if cur_pts.len() >= 2 {
                        runs.push((cur_pts, cur_inside));
                    }
                    cur_pts = vec![bp];
                    cur_inside = is_in;
                }
                cur_pts.push(pts[j]);
            }
            if cur_pts.len() >= 2 {
                runs.push((cur_pts, cur_inside));
            }

            for (run_pts, inside) in runs {
                let grp = if inside { &rot_grp } else { &static_grp };
                self.add_line_to_group(
                    grp,
                    &run_pts,
                    DISP_R as f32,
                    false,
                    crate::color::ARC_COLOR,
                );
            }
        }

        (static_grp, rot_grp)
    }
}

fn lerp_normalize(a: &[f32; 3], b: &[f32; 3], t: f32) -> [f32; 3] {
    let x = a[0] + t * (b[0] - a[0]);
    let y = a[1] + t * (b[1] - a[1]);
    let z = a[2] + t * (b[2] - a[2]);
    let len = (x * x + y * y + z * z).sqrt();
    [x / len, y / len, z / len]
}

// --- PuzzleApp ---

pub struct PuzzleApp {
    build_hash: String,
    pub(crate) params: PuzzleParams,
    pub(crate) orbit_state: OrbitAnalysisState,
    pub(crate) axis_defs: AxisDefinitions,
    pub(crate) three: Option<ThreeState>,
    worker: Option<Worker>,
    task_start_time: Option<f64>,
    is_computing: bool,
    pub(crate) compute_output: Rc<RefCell<String>>,
    pending_response: Rc<RefCell<Option<WorkerResponse>>>,
    pending_message: Option<WorkerMessage>,
    camera_input: CameraInputState,
    stored_geometry: Option<GeometryResult>,
    geometry_index: usize,
    pub(crate) anim: Option<AnimState>,
    pub(crate) orbit_result: Option<OrbitResult>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(MessageEvent)>>,

    // Dreadnaut worker
    dreadnaut_data: DreadnautManager,

    // GAP worker
    pub(crate) gap_manager: GapManager,
    pub(crate) gap_input: String,

    request_counter: usize,
    pending_dreadnaut_requests: std::collections::HashMap<usize, (usize, usize)>, // req_id -> (orbit_index, geometry_index)
    pub(crate) pending_gap_requests: std::collections::HashMap<usize, String>, // req_id -> dreadnaut hash
    pub(crate) orbit_dreadnaut: std::collections::HashMap<usize, String>,
    pub(crate) gap_cache: std::collections::HashMap<String, Option<crate::gap::GapGroupResult>>, // global table of dreadnaut hash -> gap result
}

impl PuzzleApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        three_canvas_id: String,
        build_hash: String,
    ) -> Self {
        let three = ThreeState::new(three_canvas_id);
        cc.egui_ctx.set_visuals(Visuals::dark());

        let mut app = Self {
            build_hash: build_hash.clone(),
            params: PuzzleParams::default(),
            orbit_state: OrbitAnalysisState::default(),
            axis_defs: AxisDefinitions::default(),
            three,
            worker: None,
            task_start_time: None,
            is_computing: false,
            compute_output: Rc::new(RefCell::new("Ready".to_string())),
            pending_response: Rc::new(RefCell::new(None)),
            pending_message: None,
            camera_input: CameraInputState::default(),
            stored_geometry: None,
            geometry_index: 0,
            anim: None,
            orbit_result: None,
            _on_message: None,
            _on_error: None,

            dreadnaut_data: DreadnautManager::new(),
            gap_manager: GapManager::new(),
            gap_input: String::new(),

            request_counter: 0,
            pending_dreadnaut_requests: std::collections::HashMap::new(),
            pending_gap_requests: std::collections::HashMap::new(),
            orbit_dreadnaut: std::collections::HashMap::new(),
            gap_cache: std::collections::HashMap::new(),
        };

        // Resolve default axis definitions before default puzzle params
        app.axis_defs.resolve_all();
        app.init_worker(&build_hash);
        app.dreadnaut_data.init(cc.egui_ctx.clone());
        app.gap_manager.init(cc.egui_ctx.clone());
        app.spawn_geometry_worker();
        app
    }

    fn init_worker(&mut self, build_hash: &str) {
        if self.worker.is_some() {
            return;
        }
        let options = WorkerOptions::new();
        let _ = js_sys::Reflect::set(&options, &"type".into(), &"module".into());

        let worker_url = format!("./pkg/worker.js?v={}", build_hash);

        if let Ok(w) = Worker::new_with_options(&worker_url, &options) {
            let response_clone = self.pending_response.clone();
            let on_msg = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(data) = e.data().dyn_into::<js_sys::Object>() {
                    if let Ok(response) =
                        serde_wasm_bindgen::from_value::<WorkerResponse>(data.clone().into())
                    {
                        *response_clone.borrow_mut() = Some(response);
                        return;
                    }
                    if let Ok(type_val) = js_sys::Reflect::get(&data, &"type".into()) {
                        if type_val == "success" {
                            if let Ok(res_val) = js_sys::Reflect::get(&data, &"result".into())
                                && let Ok(response) =
                                    serde_wasm_bindgen::from_value::<WorkerResponse>(res_val)
                            {
                                *response_clone.borrow_mut() = Some(response);
                            }
                        } else if type_val == "error"
                            && let Ok(err_val) = js_sys::Reflect::get(&data, &"error".into())
                            && let Some(s) = err_val.as_string()
                        {
                            *response_clone.borrow_mut() = Some(WorkerResponse::Error(s));
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);
            w.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));

            let output_err = self.compute_output.clone();
            let on_err = Closure::wrap(Box::new(move |e: MessageEvent| {
                let msg = if let Ok(data) = e.data().dyn_into::<js_sys::Object>() {
                    format!("{:?}", data)
                } else {
                    "Unknown worker error".to_string()
                };
                *output_err.borrow_mut() = format!("Worker Error: {}", msg);
            }) as Box<dyn FnMut(_)>);
            w.set_onerror(Some(on_err.as_ref().unchecked_ref()));

            self._on_message = Some(on_msg);
            self._on_error = Some(on_err);
            self.worker = Some(w);
        } else {
            *self.compute_output.borrow_mut() = "Failed to init worker".to_string();
        }
    }

    fn terminate_and_restart_worker(&mut self) {
        if let Some(w) = self.worker.take() {
            w.terminate();
        }
        self._on_message = None;
        self._on_error = None;
        self.is_computing = false;
        self.task_start_time = None;
        self.init_worker(&self.build_hash.clone());
    }

    fn post_message(&mut self, message: WorkerMessage) {
        if self.is_computing
            && let Some(start) = self.task_start_time
        {
            // Use a small timeout to avoid spawning too many workers, which causes total app failure
            if crate::time::now() - start > 200.0 {
                *self.compute_output.borrow_mut() = "Timeout, restarting worker...".to_string();
                self.terminate_and_restart_worker();
            } else {
                // Worker is busy but hasn't timed out. Queue the latest parameters instead of restarting.
                self.pending_message = Some(message);
                return;
            }
        }
        self.pending_message = None;
        if let Some(w) = &self.worker
            && let Ok(val) = serde_wasm_bindgen::to_value(&message)
        {
            let _ = w.post_message(&val);
            self.is_computing = true;
            self.task_start_time = Some(crate::time::now());
            *self.compute_output.borrow_mut() = "Computing...".to_string();
        }
    }

    pub fn set_face_group_visible(&mut self, visible: bool) {
        if let Some(three) = &self.three {
            three.face_group.set_visible(visible);
        }
    }

    fn max_iterations_cap_override(&self) -> Option<u32> {
        Some(self.params.max_iterations.max(1))
    }

    pub(crate) fn build_axes(&self) -> Vec<Option<AxisDef>> {
        let mut axes = Vec::new();
        for entry in &self.params.axes {
            if !entry.enabled {
                axes.push(None);
                continue;
            }
            // Look up the direction from resolved axis definitions
            let direction = if entry.axis_name == "X" {
                Some(glam::DVec3::X)
            } else if entry.axis_name == "Y" {
                Some(glam::DVec3::Y)
            } else if entry.axis_name == "Z" {
                Some(glam::DVec3::Z)
            } else {
                self.axis_defs.get_resolved_vector(&entry.axis_name)
            };

            if let Some(dir) = direction {
                let d = dir.normalize();
                axes.push(Some(AxisDef {
                    colat: entry.colat,
                    direction: [d.x, d.y, d.z],
                    n: entry.n,
                }));
            } else {
                axes.push(None);
            }
        }
        axes
    }

    pub(crate) fn spawn_geometry_worker(&mut self) {
        self.orbit_result = None;
        if let Some(three) = &self.three {
            three.clear_face_dots();
        }
        let axes = self.build_axes();
        if axes.is_empty() {
            *self.compute_output.borrow_mut() = "No axes selected".to_string();
            self.stored_geometry = None;
            return;
        }
        let params = GeometryParams {
            axes,
            max_iterations_cap: self.max_iterations_cap_override(),
        };
        self.post_message(WorkerMessage::ComputeGeometry(params));
    }

    pub(crate) fn spawn_orbit_worker(&mut self) {
        let axes = self.build_axes();
        if axes.is_empty() {
            *self.compute_output.borrow_mut() =
                "No valid axis angle for these parameters".to_string();
            return;
        }
        let params = OrbitParams {
            axes,
            max_iterations_cap: self.max_iterations_cap_override(),
            fudged_mode: self.orbit_state.fudged_mode,
            min_piece_angle_deg: self.orbit_state.min_piece_angle_deg,
            min_piece_perimeter: self.orbit_state.min_piece_perimeter,
        };
        self.post_message(WorkerMessage::ComputeOrbits(params));
    }

    pub(crate) fn start_rotation(&mut self, axis_index: usize, inverse: bool) {
        if self.anim.is_some() {
            return;
        }
        let stored = match &self.stored_geometry {
            Some(g) => g,
            None => return,
        };

        let axes = self.build_axes();
        let axis_def = match axes.get(axis_index) {
            Some(Some(a)) => a,
            _ => return,
        };

        let axis = axis_def.direction;
        let colat = (axis_def.colat as f64).to_radians();
        let n = axis_def.n;

        let target_angle = if inverse {
            -(TAU / n as f64)
        } else {
            TAU / n as f64
        };

        let boundary_circle = Circle::new(glam::DVec3::new(axis[0], axis[1], axis[2]), colat);

        let three = match &self.three {
            Some(t) => t,
            None => return,
        };

        let (static_grp, rot_grp) =
            three.build_animation_groups(&stored.lines, axis, colat.cos(), Some(&boundary_circle));

        three.cut_group.set_visible(false);
        three.face_group.set_visible(false);
        three.group.add(&static_grp);
        three.group.add(&rot_grp);

        let now = crate::time::now();
        self.anim = Some(AnimState {
            axis,
            target_angle,
            start_time: now,
            duration: 1200.0,
            static_group: static_grp,
            rot_group: rot_grp,
        });
    }

    fn update_animation(&mut self) {
        let now = crate::time::now();
        let finished = {
            let anim = match &self.anim {
                Some(a) => a,
                None => return,
            };
            let t = ((now - anim.start_time) / anim.duration).min(1.0);
            let eased = ease_in_out(t);
            let q = anim.rot_group.quaternion();
            let axis_v = Vector3::new(anim.axis[0], anim.axis[1], anim.axis[2]);
            q.setFromAxisAngle(&axis_v, anim.target_angle * eased);
            t >= 1.0
        };

        if finished && let Some(anim) = self.anim.take() {
            if let Some(three) = &self.three {
                crate::three::dispose_group_children(&anim.static_group);
                crate::three::dispose_group_children(&anim.rot_group);
                three.group.remove(&anim.static_group);
                three.group.remove(&anim.rot_group);
                three.cut_group.set_visible(true);
                three.face_group.set_visible(true);
            }
            self.spawn_geometry_worker();
        }
    }
}

impl eframe::App for PuzzleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // -- Animation ---
        if self.anim.is_some() {
            self.update_animation();
        }

        // -- 3D mouse / touch controls ---
        handle_camera_input(ctx, &mut self.three, &mut self.camera_input);

        if let Some(three) = &mut self.three {
            three.sync_resize();
            three.render();
        }

        // Keyboard shortcuts for rotations (disabled while typing into text fields).
        if !ctx.wants_keyboard_input() && self.anim.is_none() {
            let shift = ctx.input(|i| i.modifiers.shift);
            // A-Z for axis keybindings
            let extra_keys = egui::Key::ALL
                .iter()
                .skip(egui::Key::A as usize)
                .take(self.params.axes.len());
            for (ki, key) in extra_keys.enumerate() {
                if ctx.input(|i| i.key_pressed(*key)) {
                    self.start_rotation(ki, !shift);
                }
            }
        }

        let mut geom_response = None;
        if let Ok(mut res) = self.pending_response.try_borrow_mut() {
            geom_response = res.take();
        }
        if let Some(response) = geom_response {
            self.is_computing = false;
            self.task_start_time = None;
            match response {
                WorkerResponse::GeometryComputed(data) => {
                    self.geometry_index += 1;
                    *self.compute_output.borrow_mut() = format!("{} arcs", data.lines.len());
                    if let Some(three) = &self.three {
                        three.update_geometry(&data);
                        let axes = self.build_axes();
                        let def_vecs = self.axis_defs.get_visible_vectors();
                        let builtin_axes = self.axis_defs.get_visible_builtin_axes();
                        three.update_axis_indicators(
                            &axes,
                            self.params.show_axes,
                            &def_vecs,
                            &builtin_axes,
                        );
                    }
                    self.stored_geometry = Some(data);

                    self.orbit_state.orbits_stale = true;
                    if self.orbit_state.auto_update_orbits {
                        self.spawn_orbit_worker();
                    }
                    self.orbit_dreadnaut.clear();

                    self.orbit_result = None;
                }
                WorkerResponse::OrbitsComputed(data) => {
                    *self.compute_output.borrow_mut() =
                        format!("{} pieces, {} orbits", data.face_count, data.orbit_count);
                    if let Some(three) = &self.three {
                        three.update_face_dots(&data, self.orbit_state.number_pieces);
                    }
                    self.orbit_dreadnaut.clear();

                    let mut dreadnaut_batch = Vec::new();
                    for (oi, gens) in data.generators.iter().enumerate() {
                        let n_vertices = data
                            .face_orbit_indices
                            .iter()
                            .filter(|&&foi| match foi {
                                Some(i) => i == oi,
                                None => false,
                            })
                            .count();
                        if n_vertices > 1 && !gens.is_empty() {
                            self.request_counter += 1;
                            let req_id = self.request_counter;
                            self.pending_dreadnaut_requests
                                .insert(req_id, (oi, self.geometry_index));

                            let script = DreadnautManager::construct_script(gens, n_vertices);
                            dreadnaut_batch.push((req_id, script));
                        }
                    }
                    self.dreadnaut_data.enqueue_batch(dreadnaut_batch);

                    self.orbit_result = Some(data);

                    self.orbit_state.orbits_stale = false;
                    self.orbit_state.groups_stale = true;
                }
                WorkerResponse::Error(e) => {
                    *self.compute_output.borrow_mut() = format!("Error: {}", e);
                }
            }

            if let Some(msg) = self.pending_message.take() {
                self.post_message(msg);
            }
        }

        // -- Check dreadnaut worker results ---
        self.dreadnaut_data.process_responses();
        self.gap_manager.process_responses();

        for (req_id, dreadnaut_res) in self.dreadnaut_data.completed_jobs.drain(..) {
            if let Some(&(oi, geom_idx)) = self.pending_dreadnaut_requests.get(&req_id)
                && geom_idx == self.geometry_index
            {
                self.orbit_dreadnaut.insert(oi, dreadnaut_res.clone());
                self.pending_dreadnaut_requests.remove(&req_id);

                if self.orbit_state.auto_update_groups
                    && let Some(orbit) = &self.orbit_result
                    && let None = self.gap_cache.get(&dreadnaut_res)
                {
                    self.request_counter += 1;
                    let new_req_id = self.request_counter;
                    self.pending_gap_requests.insert(new_req_id, dreadnaut_res);
                    let cmd = GapManager::construct_group_cmd(&orbit.generators[oi]);
                    self.gap_manager.send_queued_command(new_req_id, &cmd);
                }
            }
        }

        for (req_id, res) in self.gap_manager.completed_jobs.drain(..) {
            if let Some(hash) = &self.pending_gap_requests.remove(&req_id) {
                self.gap_cache.insert(hash.to_string(), Some(res));
            }
        }

        if self.pending_dreadnaut_requests.is_empty() && self.pending_gap_requests.is_empty() {
            self.orbit_state.groups_stale = false;
        }

        crate::gui::controls::build_controls_window(ctx);
        crate::gui::gap_console::build_gap_console_window(self, ctx);

        crate::gui::orbit_analysis::build_orbit_analysis_window(self, ctx);
        crate::gui::axis_definitions::build_axis_definitions_window(self, ctx);
        crate::gui::puzzle_params::build_puzzle_params_window(self, ctx);

        ctx.request_repaint();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
