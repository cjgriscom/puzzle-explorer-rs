//! Puzzle definition import/export in YAML format

use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;
use std::cell::RefCell;
use std::rc::Rc;

use crate::types::{AxisDefinitions, OrbitAnalysisState, PuzzleParams};

/// File format version for transformer awareness
pub const PUZZLE_FORMAT_VERSION: u32 = 1;

// Raw YAML passes through here on import - use for future version incompatibilities
pub fn transform_import_yaml(yaml: &str) -> Result<String, String> {
    let mut value: Value = serde_yaml_ng::from_str(yaml).map_err(|e| e.to_string())?;
    if let Some(mapping) = value.as_mapping_mut() {
        let imported_version = mapping.insert(
            Value::String("version".into()),
            Value::Number(PUZZLE_FORMAT_VERSION.into()), // Overwrite with imported version
        );
        if let Some(v) = imported_version
            && let Some(v_int) = v.as_u64()
        {
            log::info!("Imported puzzle version: {}", v_int);
            if v != PUZZLE_FORMAT_VERSION {
                return Err(format!(
                    "Puzzle definition version {} is not supported",
                    v_int
                ));
            }
        } else {
            return Err("Could not read imported puzzle version".to_string());
        }
    }
    serde_yaml_ng::to_string(&value).map_err(|e| e.to_string())
}

/// Top level YAML structure
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PuzzleExport {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub puzzle_name: Option<String>,
    pub params: PuzzleParams,
    pub axis_defs: AxisDefinitions,
    pub orbit_state: OrbitAnalysisState,
}

impl PuzzleExport {
    pub fn to_yaml(&self) -> Result<String, String> {
        serde_yaml_ng::to_string(self).map_err(|e| e.to_string())
    }

    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        let transformed = transform_import_yaml(yaml)?;
        serde_yaml_ng::from_str(&transformed).map_err(|e| e.to_string())
    }
}

#[cfg(target_arch = "wasm32")]
pub fn trigger_download(yaml: &str, base_name: &str) {
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, HtmlAnchorElement, Url};

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let blob_parts = js_sys::Array::new();
    blob_parts.push(&js_sys::JsString::from(yaml));

    let opts = web_sys::BlobPropertyBag::new();
    opts.set_type("application/x-yaml");

    let blob = match Blob::new_with_str_sequence_and_options(&blob_parts, &opts) {
        Ok(b) => b,
        Err(_) => return,
    };

    let url = match Url::create_object_url_with_blob(&blob) {
        Ok(u) => u,
        Err(_) => return,
    };

    let anchor = match document.create_element("a") {
        Ok(a) => a,
        Err(_) => return,
    };
    let anchor: HtmlAnchorElement = match anchor.dyn_into() {
        Ok(a) => a,
        Err(_) => return,
    };

    let filename = format!("{}.yml", base_name);
    anchor.set_href(&url);
    anchor.set_download(&filename);
    anchor.style().set_property("display", "none").ok();
    if let Some(body) = document.body() {
        let _ = body.append_child(&anchor);
        anchor.click();
        let _ = body.remove_child(&anchor);
    }
    let _ = Url::revoke_object_url(&url);
}

#[cfg(target_arch = "wasm32")]
pub fn trigger_file_picker(pending: Rc<RefCell<Option<(String, String)>>>) {
    use wasm_bindgen::JsCast;
    use web_sys::{FileReader, HtmlInputElement, ProgressEvent};

    let window = match web_sys::window() {
        Some(w) => w,
        None => return,
    };
    let document = match window.document() {
        Some(d) => d,
        None => return,
    };

    let input = match document.create_element("input") {
        Ok(i) => i,
        Err(_) => return,
    };
    let input: HtmlInputElement = match input.dyn_into() {
        Ok(i) => i,
        Err(_) => return,
    };

    input.set_attribute("type", "file").ok();
    input.set_attribute("accept", ".yml,.yaml").ok();
    input.style().set_property("display", "none").ok();
    if let Some(body) = document.body() {
        let _ = body.append_child(&input);
    }

    let pending_clone = pending.clone();
    let on_change = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::Event| {
        let target = match e.target() {
            Some(t) => t,
            None => return,
        };
        let input_el: HtmlInputElement = match target.dyn_into() {
            Ok(i) => i,
            Err(_) => return,
        };
        // Remove input from DOM after use
        if let Some(parent) = input_el.parent_element() {
            let _ = parent.remove_child(&input_el);
        }
        let files = match input_el.files() {
            Some(f) => f,
            None => return,
        };
        let file = match files.get(0) {
            Some(f) => f,
            None => return,
        };
        let filename = file.name();
        let base_name = filename
            .strip_suffix(".yml")
            .or_else(|| filename.strip_suffix(".yaml"))
            .unwrap_or(&filename)
            .to_string();

        let reader = match FileReader::new() {
            Ok(r) => r,
            Err(_) => return,
        };

        let pending_inner = pending_clone.clone();
        let on_load = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: ProgressEvent| {
            let reader: FileReader = match e.target().and_then(|t| t.dyn_into().ok()) {
                Some(r) => r,
                None => return,
            };
            let result = match reader.result() {
                Ok(r) => r,
                Err(_) => return,
            };
            let content = match result.as_string() {
                Some(s) => s,
                None => return,
            };
            *pending_inner.borrow_mut() = Some((base_name.clone(), content));
        })
            as Box<dyn FnMut(ProgressEvent)>);

        reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
        reader.read_as_text(&file).ok();
        on_load.forget();
    }) as Box<dyn FnMut(web_sys::Event)>);

    input.set_onchange(Some(on_change.as_ref().unchecked_ref()));
    input.click();
    on_change.forget();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn trigger_download(_yaml: &str, _base_name: &str) {}

#[cfg(not(target_arch = "wasm32"))]
pub fn trigger_file_picker(_pending: Rc<RefCell<Option<(String, String)>>>) {}
