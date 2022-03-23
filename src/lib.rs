mod hashmap_directory;
mod index;
mod utils;

pub use index::{Schema, Index, Document};
pub use utils::set_panic_hook;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s1: &str, s2: &str);

    #[wasm_bindgen(js_namespace = console, js_name=log)]
    pub fn log3(s1: &str, s2: &str, s3: &str);
}