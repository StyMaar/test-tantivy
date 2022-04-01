mod hashmap_directory;
mod index;
mod utils;
mod new_api;
mod errors;

pub use index::{Schema, Index, Document};
use log::Level;
pub use utils::set_panic_hook;

pub use new_api::{SegmentBuilder, Segment, SearchIndex};

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = "setupLogs")]
pub fn setup_logs(module_prefix: Option<String>){
    let mut wasm_logger_cfg = wasm_logger::Config::new(Level::Trace).message_on_new_line();
    match module_prefix {
        Some(module_prefix) => {
           wasm_logger_cfg = wasm_logger_cfg.module_prefix(&module_prefix);
        }
        _ => {},   
    }
    wasm_logger::init(wasm_logger_cfg);
    
}