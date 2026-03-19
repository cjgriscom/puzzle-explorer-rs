//! Test utilities

pub fn wrap_promise_in_timeout(time_ms: i32, promise: js_sys::Promise) -> js_sys::Promise {
    use wasm_bindgen::closure::ScopedClosure;
    use wasm_bindgen::{JsCast, JsValue};

    // Promise that rejects after timeout
    let timeout_promise = js_sys::Promise::new(&mut |_, reject| {
        let reject = reject.clone();
        let closure = ScopedClosure::once(move || {
            let err = JsValue::from_str("timeout");
            let _ = reject.call1(&JsValue::NULL, &err);
        });
        web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                time_ms,
            )
            .expect("failed to set timeout");

        // Transfer ownership to JS so rust doesn't drop the closure
        closure.forget();
    });

    // Race the main promise vs timeout
    let race_array = js_sys::Array::new();
    race_array.push(&promise);
    race_array.push(&timeout_promise);

    js_sys::Promise::race(&race_array)
}

/// Parses GAP generator notation like `[(1,2)(3,4),(5,6)]`
/// Each operation is a list of cycles, each cycle is a list of element indices
pub fn parse_generator_string(generator_string: &str) -> Option<Vec<Vec<Vec<usize>>>> {
    let s = generator_string.trim();
    if s.len() < 2 || !s.starts_with('[') || !s.ends_with(']') {
        return None;
    }
    let inner = &s[1..s.len() - 1];

    let mut operations = Vec::new();
    for part in inner.split("),(") {
        let mut operation = Vec::new();
        for cycle_str in part.split(")(") {
            let cleaned: String = cycle_str
                .chars()
                .filter(|c| *c != '(' && *c != ')')
                .collect();
            let cycle: Vec<usize> = if cleaned.is_empty() {
                vec![]
            } else {
                cleaned
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.parse().ok())
                    .collect::<Option<Vec<_>>>()?
            };
            operation.push(cycle);
        }
        operations.push(operation);
    }
    Some(operations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn parse_generator_string_single_operation() {
        let result = parse_generator_string("[(1,2,5)(3,4)]");
        assert_eq!(result, Some(vec![vec![vec![1, 2, 5], vec![3, 4]]]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn parse_generator_string_multiple_operations() {
        let result = parse_generator_string("[(1,2)(3,4),(5,6,9)]");
        assert_eq!(
            result,
            Some(vec![vec![vec![1, 2], vec![3, 4]], vec![vec![5, 6, 9]]])
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn parse_generator_string_empty_cycle() {
        let result = parse_generator_string("[(),(1,2)]");
        assert_eq!(result, Some(vec![vec![vec![]], vec![vec![1, 2]]]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    fn parse_generator_string_invalid_returns_none() {
        assert_eq!(parse_generator_string(""), None);
        assert_eq!(parse_generator_string("foo"), None);
        assert_eq!(parse_generator_string("("), None);
    }
}
