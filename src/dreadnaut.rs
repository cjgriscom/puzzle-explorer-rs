use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{MessageEvent, Worker, WorkerOptions};

pub struct DreadnautManager {
    pub worker: Option<Worker>,
    pub task_start_time: Option<f64>,
    pub is_computing: bool,
    pub queue: Vec<usize>,
    pub completed_jobs: Vec<(usize, String)>,
    pub pending_responses: Rc<RefCell<Vec<String>>>,
    wakeup: Rc<dyn Fn()>,
    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(MessageEvent)>>,
}

impl DreadnautManager {
    pub fn new(wakeup: impl Fn() + 'static) -> Self {
        Self {
            worker: None,
            task_start_time: None,
            is_computing: false,
            queue: Vec::new(),
            completed_jobs: Vec::new(),
            pending_responses: Rc::new(RefCell::new(Vec::new())),
            wakeup: Rc::new(wakeup),
            _on_message: None,
            _on_error: None,
        }
    }

    pub fn init(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let options = WorkerOptions::new();
        let _ = js_sys::Reflect::set(&options, &"type".into(), &"module".into());

        if let Ok(w) = Worker::new_with_options("./dreadnaut/dreadnaut-worker.js", &options) {
            let response_clone = self.pending_responses.clone();
            let wakeup = self.wakeup.clone();
            let on_msg = Closure::wrap(Box::new(move |e: MessageEvent| {
                let mut pushed = 0;
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
                            pushed += 1;
                        }
                    }
                }
                if pushed > 0 {
                    (wakeup.as_ref())();
                }
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

    pub fn enqueue_batch(&mut self, jobs: Vec<(usize, String)>) {
        // Enqueue a batch of scripts with unique IDs
        let mut full_script = String::new();

        for (request_id, script) in jobs {
            full_script.push_str(&script);
            self.queue.push(request_id);
        }

        if !full_script.is_empty()
            && let Some(w) = &self.worker
        {
            let msg = js_sys::Object::new();
            js_sys::Reflect::set(&msg, &"type".into(), &"command".into()).unwrap();
            js_sys::Reflect::set(&msg, &"data".into(), &full_script.into()).unwrap();
            let _ = w.post_message(&msg);
            self.is_computing = true;
            self.task_start_time = Some(crate::time::now());
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
                    let request_id = self.queue.remove(0);
                    self.completed_jobs.push((request_id, res));
                }
            }
            if self.queue.is_empty() {
                self.is_computing = false;
                self.task_start_time = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::wrap_promise_in_timeout;

    use super::*;
    use puzzle_explorer_math::canon;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;

    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_canonized_orbits() {
        let test_pairs = vec![
            (
                "[(6,12,18)(24,28,26),(6,21,24)(8,29,12)]",
                "[N1a6b3ea 60141710 60c94144]",
            ),
            (
                "[(14,26)(30,52)(38,47)(62,67),(4,52,62)(14,58,18)(26,38,30)]",
                "[N67bbf135 57b681ea e896b98]",
            ),
            (
                "[(5,17)(35,46)(42,51)(61,66),(5,35,42)(13,57,17)(27,61,46)]",
                "[N68d0be14 56a98d49 d755d39]",
            ),
        ];
        let mut dreadnaut_test = DreadnautTest::new();

        for (generator, expected) in test_pairs {
            let gen_raw = crate::test::parse_generator_string(generator).unwrap();

            let (gen_renumbered, num_vertices) = canon::renumber_generator_for_dreadnaut(&gen_raw);
            dreadnaut_test.enqueue_script(canon::orbit_graph_hash_script(
                &gen_renumbered,
                num_vertices,
            ));

            assert_eq!(dreadnaut_test.await_result().await.unwrap(), expected);
        }
    }

    struct DreadnautTest {
        dreadnaut_manager: DreadnautManager,
        resolve_holder: Rc<RefCell<Option<js_sys::Function>>>,
        promise: Option<js_sys::Promise>,
    }

    impl DreadnautTest {
        fn new() -> Self {
            let resolve_holder: Rc<RefCell<Option<js_sys::Function>>> = Rc::new(RefCell::new(None));
            let promise = Self::new_promise(&resolve_holder);

            let resolve_holder_wakeup = resolve_holder.clone();
            let mut dreadnaut_manager = DreadnautManager::new(move || {
                if let Some(resolve) = resolve_holder_wakeup.borrow_mut().take() {
                    let _ = resolve.call0(&JsValue::NULL);
                }
            });
            dreadnaut_manager.init();
            Self {
                dreadnaut_manager,
                resolve_holder,
                promise: Some(promise),
            }
        }

        fn new_promise(resolve_holder: &Rc<RefCell<Option<js_sys::Function>>>) -> js_sys::Promise {
            let promise = js_sys::Promise::new(&mut |resolve, _reject| {
                *resolve_holder.borrow_mut() = Some(resolve);
            });

            wrap_promise_in_timeout(1000, promise)
        }

        fn enqueue_script(&mut self, script: String) {
            self.dreadnaut_manager.enqueue_batch(vec![(0, script)]);
        }

        async fn await_result(&mut self) -> Result<String, String> {
            let promise = std::mem::take(&mut self.promise).unwrap();
            JsFuture::from(promise)
                .await
                .map_err(|_e| "worker failed to complete")?;
            self.dreadnaut_manager.process_responses();
            self.promise = Some(Self::new_promise(&self.resolve_holder));

            match (
                self.dreadnaut_manager.completed_jobs.len(),
                self.dreadnaut_manager.completed_jobs.first(),
            ) {
                (1, Some((0, _res))) => Ok(self.dreadnaut_manager.completed_jobs.remove(0).1),
                (n, _) => Err(format!("unexpected number of completed jobs {}", n)),
            }
        }
    }
}
