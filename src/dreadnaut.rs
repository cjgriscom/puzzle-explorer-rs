use crate::puzzle::OrbitResult;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{MessageEvent, Worker, WorkerOptions, window};

pub struct DreadnautManager {
    pub worker: Option<Worker>,
    pub task_start_time: Option<f64>,
    pub is_computing: bool,
    pub queue: Vec<usize>,
    pub results: HashMap<usize, String>,
    pub pending_responses: Rc<RefCell<Vec<String>>>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(MessageEvent)>>,
}

impl DreadnautManager {
    pub fn new() -> Self {
        Self {
            worker: None,
            task_start_time: None,
            is_computing: false,
            queue: Vec::new(),
            results: HashMap::new(),
            pending_responses: Rc::new(RefCell::new(Vec::new())),
            _on_message: None,
            _on_error: None,
        }
    }

    pub fn init(&mut self, ctx: egui::Context) {
        if self.worker.is_some() {
            return;
        }
        let options = WorkerOptions::new();
        let _ = js_sys::Reflect::set(&options, &"type".into(), &"module".into());

        if let Ok(w) = Worker::new_with_options("./js/dreadnaut-worker.js", &options) {
            let response_clone = self.pending_responses.clone();
            let ctx_clone = ctx.clone();
            let on_msg = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(data) = e.data().dyn_into::<js_sys::Object>()
                    && let Ok(type_val) = js_sys::Reflect::get(&data, &"type".into())
                    && type_val.as_string().as_deref() == Some("output")
                    && let Ok(res_val) = js_sys::Reflect::get(&data, &"data".into())
                    && let Some(s) = res_val.as_string()
                {
                    // Ported from DreadnautInterface.java in GroupExplorer
                    for line in s.split('\n') {
                        let trimmed = line.trim();
                        if let Some(start) = trimmed.find('[')
                            && let Some(end) = trimmed.rfind(']')
                            && end > start
                        {
                            let hash = &trimmed[start..=end];
                            response_clone.borrow_mut().push(hash.to_string());
                        }
                    }
                }
                ctx_clone.request_repaint();
            }) as Box<dyn FnMut(_)>);
            w.set_onmessage(Some(on_msg.as_ref().unchecked_ref()));

            let on_err = Closure::wrap(Box::new(move |_e: MessageEvent| {
                // Ignore for now
            }) as Box<dyn FnMut(_)>);
            w.set_onerror(Some(on_err.as_ref().unchecked_ref()));

            self._on_message = Some(on_msg);
            self._on_error = Some(on_err);
            self.worker = Some(w);
        }
    }

    pub fn construct_script(combined_gen: &[Vec<Vec<usize>>], n_vertices: usize) -> String {
        // Ported from DreadnautInterface.java in GroupExplorer

        let mut all_two = true;
        for generator in combined_gen {
            for cycle in generator {
                if cycle.len() != 2 {
                    all_two = false;
                }
            }
        }
        let actual_directed = !all_two;

        let mut adj: Vec<std::collections::BTreeSet<usize>> =
            vec![std::collections::BTreeSet::new(); n_vertices];
        for generator in combined_gen {
            for cycle in generator {
                if cycle.len() < 2 {
                    continue;
                }
                for i in 0..cycle.len() {
                    let u = cycle[i];
                    let v = cycle[(i + 1) % cycle.len()];
                    adj[u].insert(v);
                    if !actual_directed {
                        adj[v].insert(u);
                    }
                }
            }
        }

        let mut script = String::new();
        script.push_str("l=0\n-m\n");
        if actual_directed {
            script.push_str("Ad\nd\n");
        } else {
            script.push_str("At\n");
        }
        script.push_str(&format!("n={} g\n", n_vertices));
        (0..n_vertices).for_each(|i| {
            script.push_str(&format!("{}:", i));
            let neigh = &adj[i];
            if !neigh.is_empty() {
                for &j in neigh {
                    script.push_str(&format!(" {}", j));
                }
            }
            if i == n_vertices - 1 {
                script.push_str(".\n");
            } else {
                script.push_str(";\n");
            }
        });
        script.push_str("c -a\nx\nz\n");
        script
    }

    pub fn recompute_all(&mut self, orbit: &OrbitResult) {
        self.queue.clear();
        self.results.clear();
        self.pending_responses.borrow_mut().clear();

        let mut full_script = String::new();

        for (oi, gens) in orbit.generators.iter().enumerate() {
            let n_vertices = orbit
                .face_orbit_indices
                .iter()
                .filter(|&&i| i == oi)
                .count();
            if n_vertices > 1 && !gens.is_empty() {
                let script = Self::construct_script(gens, n_vertices);
                full_script.push_str(&script);
                self.queue.push(oi);
            }
        }

        if !full_script.is_empty()
            && let Some(w) = &self.worker
        {
            let msg = js_sys::Object::new();
            js_sys::Reflect::set(&msg, &"type".into(), &"command".into()).unwrap();
            js_sys::Reflect::set(&msg, &"data".into(), &full_script.into()).unwrap();
            let _ = w.post_message(&msg);
            self.is_computing = true;
            self.task_start_time = Some(window().unwrap().performance().unwrap().now());
        }
    }

    pub fn process_responses(&mut self) {
        let mut new_responses = Vec::new();
        if let Ok(mut pending) = self.pending_responses.try_borrow_mut() {
            new_responses.extend(pending.drain(..));
        }

        if !new_responses.is_empty() {
            for res in new_responses {
                if !self.queue.is_empty() {
                    let orbit_idx = self.queue.remove(0);
                    self.results.insert(orbit_idx, res);
                }
            }
            if self.queue.is_empty() {
                self.is_computing = false;
                self.task_start_time = None;
            }
        }
    }
}
