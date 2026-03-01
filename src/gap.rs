use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use web_sys::{MessageEvent, Worker, WorkerOptions};

#[derive(Clone, Debug, PartialEq)]
pub struct GapGroupResult {
    pub size: String,
    pub structure: String,
}

#[derive(Clone, PartialEq)]
pub enum GapState {
    NotStarted,
    Loading(String, f32), // Status text, progress (0.0 to 1.0 roughly)
    Ready,
    Error(String),
}

pub struct GapManager {
    pub worker: Option<Worker>,
    pub state: GapState,
    pub output_history: String,

    // SharedArrayBuffer and its views for simulating stdin
    pub int32_array: Option<js_sys::Int32Array>,
    pub uint8_array: Option<js_sys::Uint8Array>,

    // Queue of messages received from worker but not yet processed
    pub pending_messages: Rc<RefCell<Vec<js_sys::Object>>>,

    // Command queue and buffered output state
    pub queue: Vec<usize>,
    pub pending_commands: std::collections::VecDeque<(usize, String)>,
    pub completed_jobs: Vec<(usize, GapGroupResult)>,
    pub current_buffer: String,
    pub current_job: Option<usize>,

    _on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    _on_error: Option<Closure<dyn FnMut(MessageEvent)>>,
}

impl GapManager {
    pub fn new() -> Self {
        Self {
            worker: None,
            state: GapState::NotStarted,
            output_history: String::from("GAP quiet mode output will appear here.\n"),
            int32_array: None,
            uint8_array: None,
            pending_messages: Rc::new(RefCell::new(Vec::new())),
            queue: Vec::new(),
            pending_commands: std::collections::VecDeque::new(),
            completed_jobs: Vec::new(),
            current_buffer: String::new(),
            current_job: None,
            _on_message: None,
            _on_error: None,
        }
    }

    pub fn init(&mut self, ctx: egui::Context) {
        if self.worker.is_some() {
            return;
        }
        let options = WorkerOptions::new();

        if let Ok(w) = Worker::new_with_options("./gap/gap-worker.js", &options) {
            self.state = GapState::Loading("Starting worker...".to_string(), 0.0);

            let messages_clone = self.pending_messages.clone();
            let ctx_clone = ctx.clone();

            let on_msg = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(data) = e.data().dyn_into::<js_sys::Object>() {
                    messages_clone.borrow_mut().push(data);
                    ctx_clone.request_repaint();
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
        } else {
            self.state = GapState::Error("Failed to initialize worker".to_string());
        }
    }

    pub fn process_responses(&mut self) {
        let mut new_msgs = Vec::new();
        if let Ok(mut pending) = self.pending_messages.try_borrow_mut() {
            new_msgs.extend(pending.drain(..));
        }

        for msg in new_msgs {
            if let Ok(type_val) = js_sys::Reflect::get(&msg, &"type".into())
                && let Some(t) = type_val.as_string()
            {
                match t.as_str() {
                    "init_sab" => {
                        if let Ok(sab_val) = js_sys::Reflect::get(&msg, &"sab".into())
                            && let Ok(sab) = sab_val.dyn_into::<js_sys::SharedArrayBuffer>()
                        {
                            self.int32_array = Some(js_sys::Int32Array::new(&sab));
                            self.uint8_array = Some(js_sys::Uint8Array::new(&sab));
                        }
                    }
                    "status" => {
                        if let Ok(data_val) = js_sys::Reflect::get(&msg, &"data".into())
                            && let Some(s) = data_val.as_string()
                        {
                            if let GapState::Loading(_, prog) = self.state {
                                self.state = GapState::Loading(s, prog);
                            } else {
                                self.state = GapState::Loading(s, 0.0);
                            }
                        }
                    }
                    "progress" => {
                        if let Ok(data_val) = js_sys::Reflect::get(&msg, &"data".into())
                            && let Some(prog) = data_val.as_f64()
                            && let GapState::Loading(ref s, _) = self.state
                        {
                            self.state = GapState::Loading(s.clone(), prog as f32);
                        }
                    }
                    "output" => {
                        if let Ok(data_val) = js_sys::Reflect::get(&msg, &"data".into())
                            && let Some(mut s) = data_val.as_string()
                        {
                            // 1. Check if we are starting a job
                            if self.current_job.is_none() && !self.queue.is_empty() {
                                let expected_start = format!("---START_{}---", self.queue[0]);
                                if let Some(idx) = s.find(&expected_start) {
                                    self.current_job = Some(self.queue.remove(0));
                                    s = s[idx + expected_start.len()..].to_string(); // Strip start marker
                                    // Remove trailing newline if it's there from the Print
                                    if s.starts_with('\n') {
                                        s = s[1..].to_string();
                                    }
                                }
                            }

                            // 2. Buffer output for the current job
                            if let Some(job_id) = self.current_job {
                                let expected_end = format!("---END_{}---", job_id);
                                if let Some(idx) = s.find(&expected_end) {
                                    // Job finished!
                                    self.current_buffer.push_str(&s[..idx]);

                                    // Save result
                                    let result_text =
                                        std::mem::take(&mut self.current_buffer).trim().to_string();

                                    let mut lines = result_text.lines();
                                    // The output looks like: `<size>\n<structure>`
                                    let size_str = lines.next().unwrap_or("").trim().to_string();
                                    let struct_str = lines.next().unwrap_or("").trim().to_string();

                                    let result = GapGroupResult {
                                        size: if size_str.is_empty() {
                                            "Unknown".to_string()
                                        } else {
                                            size_str
                                        },
                                        structure: if struct_str.is_empty() {
                                            "Unknown".to_string()
                                        } else {
                                            struct_str
                                        },
                                    };

                                    self.completed_jobs.push((job_id, result));
                                    self.current_job = None;
                                } else {
                                    // Still reading output for this job
                                    self.current_buffer.push_str(&s);
                                }
                            }

                            self.output_history.push_str(&s);
                        }
                    }
                    "error" => {
                        if let Ok(data_val) = js_sys::Reflect::get(&msg, &"data".into())
                            && let Some(s) = data_val.as_string()
                        {
                            self.output_history.push_str(&format!("ERROR: {}\n", s));
                        }
                    }
                    "ready" => {
                        self.state = GapState::Ready;
                    }
                    "read_request" => {
                        // Waiting on STDIN
                        self.state = GapState::Ready;
                        self.try_send_next_job();
                    }
                    _ => {}
                }
            }
        }

        // If there are no pending messages, let's see if we should send a command from the queue
        // But we must be waiting for input
        self.try_send_next_job();
    }

    fn try_send_next_job(&mut self) {
        if self.state != GapState::Ready {
            return;
        }

        if !self.pending_commands.is_empty()
            && let (Some(i32_arr), Some(_)) = (&self.int32_array, &self.uint8_array)
        {
            let state = js_sys::Atomics::load(i32_arr, 0).unwrap_or(0);
            if state == 1 {
                // Ready to read
                let (job_id, cmd) = self.pending_commands.pop_front().unwrap();
                self.queue.push(job_id);
                let wrapped_cmd = format!(
                    "Print(\"---START_{}---\\n\");\n{}\nPrint(\"---END_{}---\\n\");\n",
                    job_id, cmd, job_id
                );
                self.send_command(&wrapped_cmd);
            }
        }
    }

    pub fn send_command(&mut self, cmd: &str) {
        if self.state != GapState::Ready {
            self.output_history.push_str("GAP not ready.\n");
            return;
        }

        if let (Some(i32_arr), Some(u8_arr)) = (&self.int32_array, &self.uint8_array) {
            let state = js_sys::Atomics::load(i32_arr, 0).unwrap_or(0);
            if state != 1 {
                self.output_history
                    .push_str("GAP is not waiting for input right now.\n");
                return;
            }

            self.output_history.push_str(&format!("gap> {}\n", cmd));

            let mut final_cmd = cmd.to_string();
            if !final_cmd.ends_with('\n') {
                final_cmd.push('\n');
            }

            let bytes = final_cmd.as_bytes();
            let len = bytes.len() as i32;

            let _ = js_sys::Atomics::store(i32_arr, 1, len);

            for (i, &b) in bytes.iter().enumerate() {
                let _ = js_sys::Atomics::store(u8_arr, 8 + i as u32, b as i32);
            }

            let _ = js_sys::Atomics::store(i32_arr, 0, 2);
            let _ = js_sys::Atomics::notify(i32_arr, 0);
        } else {
            self.output_history
                .push_str("Internal error: SAB not initialized.\n");
        }
    }

    pub fn send_queued_command(&mut self, job_id: usize, cmd: &str) {
        self.pending_commands.push_back((job_id, cmd.to_string()));
        self.try_send_next_job();
    }

    pub fn clear_queue(&mut self) {
        self.queue.clear();
        self.pending_commands.clear();
        self.completed_jobs.clear();
        self.current_job = None;
        self.current_buffer.clear();
    }

    pub fn construct_group_cmd(generators: &[Vec<Vec<usize>>]) -> String {
        let mut gap_parts = Vec::new();
        for generator in generators {
            if generator.is_empty() {
                gap_parts.push("()".to_string());
            } else {
                let cycle_str = generator
                    .iter()
                    .map(|cycle| {
                        let c_str = cycle
                            .iter()
                            .map(|&idx| (idx + 1).to_string()) // 1-indexed for GAP
                            .collect::<Vec<_>>()
                            .join(",");
                        format!("({})", c_str)
                    })
                    .collect::<Vec<_>>()
                    .join("");
                gap_parts.push(cycle_str);
            }
        }

        format!(
            "g := Group([{}]);; Print(Size(g), \"\\n\", StructureDescription(g));",
            gap_parts.join(",")
        )
    }
}
