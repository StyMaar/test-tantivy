export * from "./tantivy";
import tantivy_wasm from "./pkg/tantivy_js_bg.wasm";
import { setWasmInit } from "./tantivy";

// @ts-ignore
setWasmInit(() => tantivy_wasm());
