use wasm_bindgen::prelude::*;

/// Get citations from text.
/// Full implementation in Phase 12.
#[wasm_bindgen(js_name = "getCitations")]
pub fn get_citations(_text: &str) -> JsValue {
    JsValue::NULL
}
