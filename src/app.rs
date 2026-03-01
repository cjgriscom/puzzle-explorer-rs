use egui::Visuals;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, MessageEvent, Worker, WorkerOptions, window};

use crate::color::{ORBIT_COLORS, SINGLETON_COLOR, color_to_hex};
use crate::dreadnaut::DreadnautManager;
use crate::gap::{GapManager, GapState};
use crate::geometry::{self, DISP_R, R, TAU};
use crate::gui::{OrbitAnalysisState, PuzzleParams, toggle};
use crate::puzzle::{GeometryParams, GeometryResult, OrbitParams, OrbitResult, PolyLine};
use crate::three::{
    BufferAttribute, BufferGeometry, Group, Line, LineBasicMaterial, LineLoop, Mesh,
    MeshBasicMaterial, PerspectiveCamera, Quaternion, Scene, SphereGeometry, Vector3,
    WebGLRenderer,
};
use crate::worker::{WorkerMessage, WorkerResponse};

// --- Animation State ---

struct AnimState {
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
    cam_dist: f64,
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
        renderer.setClearColor(0xdddddd);

        let group = Group::new();

        // White sphere
        let sphere_geo = SphereGeometry::new(R, 64, 48);
        let mat_params = js_sys::Object::new();
        js_sys::Reflect::set(&mat_params, &"color".into(), &0xffffffu32.into()).ok()?;
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

        group.rotateX(0.35);
        group.rotateY(-0.5);
        group.position().set(0.0, -0.3, 0.0);
        scene.add(&group);

        Some(Self {
            scene,
            camera,
            renderer,
            group,
            cut_group,
            face_group,
            cam_dist,
        })
    }

    pub fn render(&self) {
        self.renderer.render(&self.scene, &self.camera);
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
        self.camera.cam_position().set(0.0, 0.0, self.cam_dist);
    }

    pub fn update_geometry(&self, result: &GeometryResult) {
        self.cut_group.clear();
        for poly_line in &result.lines {
            self.add_line_to_group(
                &self.cut_group,
                &poly_line.points,
                poly_line.is_loop,
                0x000000,
            );
        }
    }

    fn add_line_to_group(&self, grp: &Group, points: &[[f32; 3]], is_loop: bool, color: u32) {
        let geometry = BufferGeometry::new();
        let mut flat = Vec::with_capacity(points.len() * 3);
        for p in points {
            flat.push(p[0]);
            flat.push(p[1]);
            flat.push(p[2]);
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

    pub fn update_face_dots(&self, orbit_result: &OrbitResult) {
        self.face_group.clear();

        let n_orbits = orbit_result.orbit_count;
        // Build orbit_index -> color mapping
        let mut orbit_is_singleton = vec![false; n_orbits];
        let mut orbit_color_idx = vec![0usize; n_orbits];
        {
            // Count members per orbit
            let mut counts = vec![0usize; n_orbits];
            for &oi in &orbit_result.face_orbit_indices {
                counts[oi] += 1;
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
            let oi = orbit_result.face_orbit_indices[fi];
            let color = if orbit_is_singleton[oi] {
                SINGLETON_COLOR.1
            } else {
                ORBIT_COLORS[orbit_color_idx[oi] % ORBIT_COLORS.len()].1
            };

            let mat_params = js_sys::Object::new();
            let _ =
                js_sys::Reflect::set(&mat_params, &"color".into(), &color_to_hex(&color).into());
            let mat = MeshBasicMaterial::new(&mat_params);
            let mesh = Mesh::new(&dot_geo, &mat);
            mesh.position()
                .set(pos[0] as f64, pos[1] as f64, pos[2] as f64);
            self.face_group.add(&mesh);
        }
    }

    pub fn clear_face_dots(&self) {
        self.face_group.clear();
    }

    /// Build the two animation groups by splitting stored arc points along the cap boundary.
    pub fn build_animation_groups(
        &self,
        lines: &[PolyLine],
        axis: [f64; 3],
        cos_colat: f64,
        boundary_circle: Option<&geometry::Circle>,
    ) -> (Group, Group) {
        let static_grp = Group::new();
        let rot_grp = Group::new();
        let eps = 1e-4;

        // Boundary circle
        if let Some(circ) = boundary_circle {
            let pts = geometry::sample_arc(circ, 0.0, TAU, 128);
            self.add_line_to_group_raw(&static_grp, &pts, true, 0x000000);
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
                self.add_line_to_group_raw(grp, &run_pts, false, 0x000000);
            }
        }

        (static_grp, rot_grp)
    }

    fn add_line_to_group_raw(&self, grp: &Group, points: &[[f32; 3]], is_loop: bool, color: u32) {
        let geometry = BufferGeometry::new();
        let mut flat = Vec::with_capacity(points.len() * 3);
        for p in points {
            flat.push(p[0]);
            flat.push(p[1]);
            flat.push(p[2]);
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
}

fn lerp_normalize(a: &[f32; 3], b: &[f32; 3], t: f32) -> [f32; 3] {
    let x = a[0] + t * (b[0] - a[0]);
    let y = a[1] + t * (b[1] - a[1]);
    let z = a[2] + t * (b[2] - a[2]);
    let len = (x * x + y * y + z * z).sqrt();
    let disp_r = DISP_R as f32;
    [x / len * disp_r, y / len * disp_r, z / len * disp_r]
}

// --- PuzzleApp ---

pub struct PuzzleApp {
    params: PuzzleParams,
    orbit_state: OrbitAnalysisState,
    three: Option<ThreeState>,
    worker: Option<Worker>,
    task_start_time: Option<f64>,
    is_computing: bool,
    compute_output: Rc<RefCell<String>>,
    pending_response: Rc<RefCell<Option<WorkerResponse>>>,
    pending_message: Option<WorkerMessage>,
    is_dragging: bool,
    last_mouse_pos: [f32; 2],
    stored_geometry: Option<GeometryResult>,
    anim: Option<AnimState>,
    orbit_result: Option<OrbitResult>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(MessageEvent)>>,

    // Dreadnaut worker
    dreadnaut_data: DreadnautManager,

    // GAP worker
    gap_manager: GapManager,
    gap_input: String,

    request_counter: usize,
    pending_dreadnaut_requests: std::collections::HashMap<usize, usize>, // req_id -> orbit_index
    pending_gap_requests: std::collections::HashMap<usize, usize>,       // req_id -> orbit_index
    orbit_dreadnaut: std::collections::HashMap<usize, String>,
    orbit_gap: std::collections::HashMap<usize, crate::gap::GapGroupResult>,
    gap_cache: std::collections::HashMap<String, crate::gap::GapGroupResult>, // global table of dreadnaut hash -> gap result
}

impl PuzzleApp {
    pub fn new(cc: &eframe::CreationContext<'_>, three_canvas_id: String) -> Self {
        let three = ThreeState::new(three_canvas_id);
        cc.egui_ctx.set_visuals(Visuals::light());

        let mut app = Self {
            params: PuzzleParams::default(),
            orbit_state: OrbitAnalysisState::default(),
            three,
            worker: None,
            task_start_time: None,
            is_computing: false,
            compute_output: Rc::new(RefCell::new("Ready".to_string())),
            pending_response: Rc::new(RefCell::new(None)),
            pending_message: None,
            is_dragging: false,
            last_mouse_pos: [0.0, 0.0],
            stored_geometry: None,
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
            orbit_gap: std::collections::HashMap::new(),
            gap_cache: std::collections::HashMap::new(),
        };

        app.init_worker();
        app.dreadnaut_data.init(cc.egui_ctx.clone());
        app.gap_manager.init(cc.egui_ctx.clone());
        app.spawn_geometry_worker();
        app
    }

    fn init_worker(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let options = WorkerOptions::new();
        let _ = js_sys::Reflect::set(&options, &"type".into(), &"module".into());

        if let Ok(w) = Worker::new_with_options("./worker.js", &options) {
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
        self.init_worker();
    }

    fn post_message(&mut self, message: WorkerMessage) {
        if self.is_computing
            && let Some(start) = self.task_start_time
        {
            // Use a small timeout to avoid spawning too many workers, which causes total app failure
            if window().unwrap().performance().unwrap().now() - start > 200.0 {
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
            self.task_start_time = Some(window().unwrap().performance().unwrap().now());
            *self.compute_output.borrow_mut() = "Computing...".to_string();
        }
    }

    fn spawn_geometry_worker(&mut self) {
        self.orbit_result = None;
        if let Some(three) = &self.three {
            three.clear_face_dots();
        }
        let params = GeometryParams {
            n_a: self.params.n_a,
            n_b: self.params.n_b,
            p: self.params.p,
            q: self.params.q,
            colat_a: self.params.colat_a,
            colat_b: self.params.colat_b,
        };
        self.post_message(WorkerMessage::ComputeGeometry(params));
    }

    fn spawn_orbit_worker(&mut self) {
        let params = OrbitParams {
            n_a: self.params.n_a,
            n_b: self.params.n_b,
            p: self.params.p,
            q: self.params.q,
            colat_a: self.params.colat_a,
            colat_b: self.params.colat_b,
        };
        self.post_message(WorkerMessage::ComputeOrbits(params));
    }

    fn start_rotation(&mut self, which: char, inverse: bool) {
        if self.anim.is_some() {
            return;
        }
        let stored = match &self.stored_geometry {
            Some(g) => g,
            None => return,
        };

        let axis_angle = match geometry::derive_axis_angle(
            self.params.n_a,
            self.params.n_b,
            self.params.p,
            self.params.q,
        ) {
            Some(a) => a,
            None => return,
        };

        let colat_a_rad = (self.params.colat_a as f64).to_radians();
        let colat_b_rad = (self.params.colat_b as f64).to_radians();

        let (axis, colat, n) = if which == 'A' {
            ([0.0, 0.0, 1.0], colat_a_rad, self.params.n_a)
        } else {
            let sa = axis_angle.sin();
            let ca = axis_angle.cos();
            ([sa, 0.0, ca], colat_b_rad, self.params.n_b)
        };

        let target_angle = if inverse {
            -(TAU / n as f64)
        } else {
            TAU / n as f64
        };
        let cos_colat = colat.cos();

        let boundary_circle =
            geometry::make_circ(glam::DVec3::new(axis[0], axis[1], axis[2]), colat);

        let three = match &self.three {
            Some(t) => t,
            None => return,
        };

        let (static_grp, rot_grp) =
            three.build_animation_groups(&stored.lines, axis, cos_colat, Some(&boundary_circle));

        three.cut_group.set_visible(false);
        three.face_group.set_visible(false);
        three.group.add(&static_grp);
        three.group.add(&rot_grp);

        let now = window().unwrap().performance().unwrap().now();
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
        let now = window().unwrap().performance().unwrap().now();
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

        // -- 3D mouse controls ---
        let pointer_over_ui = ctx.is_pointer_over_area();
        let anim_active = self.anim.is_some();

        if !pointer_over_ui && !anim_active {
            if ctx.input(|i| i.pointer.primary_down()) {
                if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    if self.is_dragging {
                        let dx = (pos.x - self.last_mouse_pos[0]) as f64 * 0.005;
                        let dy = (pos.y - self.last_mouse_pos[1]) as f64 * 0.005;
                        if let Some(three) = &self.three {
                            three.rotate_drag(dx, dy);
                        }
                    }
                    self.is_dragging = true;
                    self.last_mouse_pos = [pos.x, pos.y];
                }
            } else {
                self.is_dragging = false;
            }
            let scroll_y = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll_y != 0.0
                && let Some(three) = &mut self.three
            {
                three.zoom(scroll_y as f64);
            }
        } else {
            self.is_dragging = false;
        }

        if let Some(three) = &self.three {
            three.render();
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
                    *self.compute_output.borrow_mut() = format!("{} arcs", data.lines.len());
                    if let Some(three) = &self.three {
                        three.update_geometry(&data);
                    }
                    self.stored_geometry = Some(data);

                    self.orbit_state.orbits_stale = true;
                    if self.orbit_state.auto_update_orbits {
                        self.spawn_orbit_worker();
                    }
                    self.orbit_dreadnaut.clear();
                    self.orbit_gap.clear();
                    self.dreadnaut_data.clear_queue();
                    self.gap_manager.clear_queue();
                    self.pending_dreadnaut_requests.clear();
                    self.pending_gap_requests.clear();

                    self.orbit_result = None;
                }
                WorkerResponse::OrbitsComputed(data) => {
                    *self.compute_output.borrow_mut() =
                        format!("{} pieces, {} orbits", data.face_count, data.orbit_count);
                    if let Some(three) = &self.three {
                        three.update_face_dots(&data);
                    }
                    self.orbit_dreadnaut.clear();
                    self.orbit_gap.clear();
                    self.pending_dreadnaut_requests.clear();
                    self.pending_gap_requests.clear();

                    let mut dreadnaut_batch = Vec::new();
                    for (oi, gens) in data.generators.iter().enumerate() {
                        let n_vertices =
                            data.face_orbit_indices.iter().filter(|&&i| i == oi).count();
                        if n_vertices > 1 && !gens.is_empty() {
                            self.request_counter += 1;
                            let req_id = self.request_counter;
                            self.pending_dreadnaut_requests.insert(req_id, oi);

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

        let group_update_requested = self.orbit_state.requested_groups_update;

        for (req_id, res) in self.dreadnaut_data.completed_jobs.drain(..) {
            if let Some(&oi) = self.pending_dreadnaut_requests.get(&req_id) {
                self.orbit_dreadnaut.insert(oi, res.clone());
                self.pending_dreadnaut_requests.remove(&req_id);

                if (self.orbit_state.auto_update_groups || group_update_requested)
                    && let Some(orbit) = &self.orbit_result
                {
                    if let Some(cached) = self.gap_cache.get(&res) {
                        self.orbit_gap.insert(oi, cached.clone());
                    } else {
                        self.request_counter += 1;
                        let new_req_id = self.request_counter;
                        self.pending_gap_requests.insert(new_req_id, oi);
                        let cmd = GapManager::construct_group_cmd(&orbit.generators[oi]);
                        self.gap_manager.send_queued_command(new_req_id, &cmd);
                    }
                }
            }
        }

        for (req_id, res) in self.gap_manager.completed_jobs.drain(..) {
            if let Some(&oi) = self.pending_gap_requests.get(&req_id) {
                self.orbit_gap.insert(oi, res.clone());
                self.pending_gap_requests.remove(&req_id);

                if let Some(hash) = self.orbit_dreadnaut.get(&oi) {
                    self.gap_cache.insert(hash.clone(), res);
                }
            }
        }

        if self.pending_dreadnaut_requests.is_empty() && self.pending_gap_requests.is_empty() {
            self.orbit_state.groups_stale = false;
            self.orbit_state.requested_groups_update = false;
        }

        // -- Controls Window ---
        let buttons_enabled = self.anim.is_none();

        egui::Window::new("Puzzle Controls")
            .default_pos([50.0, 50.0])
            .show(ctx, |ui| {
                // Bigger slider than default
                ui.spacing_mut().slider_width = 250.0;

                ui.heading("Parameters");
                let mut changed = false;

                ui.horizontal(|ui| {
                    ui.label("nA:");
                    egui::ComboBox::from_id_salt("nA")
                        .selected_text(format!("{}", self.params.n_a))
                        .show_ui(ui, |ui| {
                            for i in 2..=8 {
                                if ui
                                    .selectable_value(&mut self.params.n_a, i, format!("{}", i))
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                    ui.label("nB:");
                    egui::ComboBox::from_id_salt("nB")
                        .selected_text(format!("{}", self.params.n_b))
                        .show_ui(ui, |ui| {
                            for i in 2..=8 {
                                if ui
                                    .selectable_value(&mut self.params.n_b, i, format!("{}", i))
                                    .changed()
                                {
                                    changed = true;
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("p/q:");

                    if ui
                        .add(
                            egui::DragValue::new(&mut self.params.p)
                                .range(1..=20)
                                .speed(0.02),
                        )
                        .changed()
                    {
                        changed = true;
                    }

                    ui.label("/");

                    if ui
                        .add(
                            egui::DragValue::new(&mut self.params.q)
                                .range(2..=30)
                                .speed(0.02),
                        )
                        .changed()
                    {
                        changed = true;
                    }

                    if let Some(ang) = crate::geometry::derive_axis_angle(
                        self.params.n_a,
                        self.params.n_b,
                        self.params.p,
                        self.params.q,
                    ) {
                        ui.label(format!("Cut: {:.4}\u{00B0}", ang.to_degrees()));
                    }
                });

                ui.separator();

                if ui
                    .checkbox(&mut self.params.lock_cuts, "Lock cuts together")
                    .changed()
                    && self.params.lock_cuts
                {
                    self.params.colat_b = self.params.colat_a;
                    changed = true;
                }

                ui.separator();

                ui.label(format!("Cut A: {:.1}\u{00B0}", self.params.colat_a));
                if ui
                    .add(
                        egui::Slider::new(&mut self.params.colat_a, 10.0..=170.0)
                            .smallest_positive(0.1)
                            .fixed_decimals(1)
                            .step_by(0.1)
                            .drag_value_speed(0.1)
                            .show_value(true)
                            .trailing_fill(true),
                    )
                    .changed()
                {
                    if self.params.lock_cuts {
                        self.params.colat_b = self.params.colat_a;
                    }
                    changed = true;
                }

                ui.label(format!("Cut B: {:.1}\u{00B0}", self.params.colat_b));
                ui.add_enabled_ui(!self.params.lock_cuts, |ui| {
                    if ui
                        .add(
                            egui::Slider::new(&mut self.params.colat_b, 10.0..=170.0)
                                .smallest_positive(0.1)
                                .fixed_decimals(1)
                                .step_by(0.1)
                                .drag_value_speed(0.1)
                                .show_value(true)
                                .trailing_fill(true),
                        )
                        .changed()
                    {
                        if self.params.lock_cuts {
                            self.params.colat_a = self.params.colat_b;
                        }
                        changed = true;
                    }
                });

                if changed {
                    self.spawn_geometry_worker();
                }

                ui.separator();

                ui.add_enabled_ui(buttons_enabled, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Rotate A").clicked() {
                            self.start_rotation('A', true);
                        }
                        if ui.button("A'").clicked() {
                            self.start_rotation('A', false);
                        }
                        if ui.button("Rotate B").clicked() {
                            self.start_rotation('B', true);
                        }
                        if ui.button("B'").clicked() {
                            self.start_rotation('B', false);
                        }
                    });
                });
            });

        // Orbit Analysis Window
        egui::Window::new("Orbit Analysis")
            .default_pos([50.0, 350.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add(toggle(&mut self.orbit_state.annotate_pieces))
                        .changed()
                        && let Some(three) = &self.three
                    {
                        three
                            .face_group
                            .set_visible(self.orbit_state.annotate_pieces);
                    }
                    ui.label("Annotate pieces");
                });

                ui.horizontal(|ui| {
                    if ui
                        .add(toggle(&mut self.orbit_state.auto_update_orbits))
                        .changed()
                        && self.orbit_state.auto_update_orbits
                        && self.orbit_state.orbits_stale
                    {
                        self.spawn_orbit_worker();
                    }
                    ui.label("Automatically update orbits");
                });

                ui.horizontal(|ui| {
                    if ui
                        .add(toggle(&mut self.orbit_state.auto_update_groups))
                        .changed()
                        && self.orbit_state.auto_update_groups
                    {
                        self.orbit_state.groups_stale = true;
                        self.orbit_dreadnaut.clear();
                        self.orbit_gap.clear();
                        self.spawn_orbit_worker();
                    }
                    ui.label("Automatically update groups");
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            buttons_enabled
                                && (!self.orbit_state.auto_update_orbits
                                    || self.orbit_state.orbits_stale),
                            egui::Button::new("Recompute Orbits"),
                        )
                        .clicked()
                    {
                        self.spawn_orbit_worker();
                    }

                    if ui
                        .add_enabled(
                            buttons_enabled
                                && self.orbit_result.is_some()
                                && (!self.orbit_state.auto_update_groups
                                    || self.orbit_state.groups_stale),
                            egui::Button::new("Recompute Groups"),
                        )
                        .clicked()
                    {
                        self.orbit_state.requested_groups_update = true;
                        self.orbit_state.groups_stale = true;
                        self.orbit_dreadnaut.clear();
                        self.orbit_gap.clear();
                        self.spawn_orbit_worker();
                    }
                });

                let err_msg = self.compute_output.borrow().clone();
                if err_msg.starts_with("Error:") && self.orbit_result.is_none() {
                    ui.separator();
                    ui.label(egui::RichText::new(&err_msg).color(egui::Color32::RED));
                }

                // Show orbit tree
                if let Some(orbit) = &self.orbit_result {
                    ui.separator();
                    egui::ScrollArea::vertical().vscroll(true).show(ui, |ui| {
                        let msg = self.compute_output.borrow().clone();
                        if msg.starts_with("Error:") {
                            ui.label(egui::RichText::new(msg).color(egui::Color32::RED));
                        } else {
                            ui.label(format!("Pieces: {}", orbit.face_count));
                        }
                        ui.label(format!("Total Orbits: {}", orbit.orbit_count));

                        let mut orbits_with_members: Vec<(usize, usize, Vec<usize>)> = (0..orbit
                            .orbit_count)
                            .map(|oi| {
                                (
                                    oi,
                                    0, // placeholder
                                    orbit
                                        .face_orbit_indices
                                        .iter()
                                        .enumerate()
                                        .filter(|&(_, &o)| o == oi)
                                        .map(|(i, _)| i + 1)
                                        .collect::<Vec<usize>>(),
                                )
                            })
                            .filter(|(_, _, members)| members.len() > 1)
                            .collect();

                        // Give them an original color index based on the unsorted layout (skipping singletons)
                        (0..orbits_with_members.len()).for_each(|i| {
                            orbits_with_members[i].1 = i;
                        });

                        orbits_with_members
                            .sort_by_key(|(_, _, members)| -(members.len() as isize));

                        for (oi, color_idx, members) in orbits_with_members {
                            let c = crate::color::ORBIT_COLORS
                                [color_idx % crate::color::ORBIT_COLORS.len()];
                            let rgb = c.1;
                            let color_name = c.0;

                            let header_text =
                                format!("     {}: {} pieces", color_name, members.len());

                            // Draw circle in header
                            let collapsing_resp = egui::CollapsingHeader::new(header_text)
                                .id_salt(format!("orbit_header_{}", oi))
                                .default_open(true)
                                .show(ui, |ui| {
                                    if let Some(hash) = self.orbit_dreadnaut.get(&oi) {
                                        ui.label(format!("Canon Hash: {}", hash));
                                    } else {
                                        ui.label("Canon Hash: Computing...");
                                    }

                                    if let Some(struct_desc) = self.orbit_gap.get(&oi) {
                                        ui.label(format!("Size: {}", struct_desc.size));
                                        ui.label(format!("Structure: {}", struct_desc.structure));
                                    } else {
                                        ui.label("Size: (not computed)");
                                        ui.label("Structure: (not computed)");
                                    }
                                });

                            // Draw circle on the header rect
                            let circle_center = collapsing_resp.header_response.rect.left_center()
                                + egui::vec2(24.0, 0.0);
                            ui.painter().circle_filled(
                                circle_center,
                                5.0,
                                egui::Color32::from_rgb(
                                    (rgb[0] * 255.0) as u8,
                                    (rgb[1] * 255.0) as u8,
                                    (rgb[2] * 255.0) as u8,
                                ),
                            );
                        }
                    });
                }
            });

        // GAP Window
        egui::Window::new("GAP Console")
            .default_pos([500.0, 50.0])
            .default_width(500.0)
            .default_open(false)
            .show(ctx, |ui| match &self.gap_manager.state {
                GapState::NotStarted => {
                    ui.label("GAP is not started.");
                }
                GapState::Loading(status, progress) => {
                    ui.label(status);
                    ui.add(egui::ProgressBar::new(*progress));
                    ui.spinner();
                }
                GapState::Error(err) => {
                    ui.label(format!("Error loading GAP: {}", err));
                }
                GapState::Ready => {
                    egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .auto_shrink(false)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.monospace(&self.gap_manager.output_history);
                        });

                    ui.horizontal(|ui| {
                        ui.label("gap>");
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.gap_input)
                                .desired_width(f32::INFINITY),
                        );
                        if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let cmd = self.gap_input.clone();
                            self.gap_input.clear();
                            self.gap_manager.send_command(&cmd);
                            response.request_focus();
                        }
                    });
                }
            });

        ctx.request_repaint();
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
