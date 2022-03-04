# Testing Tantivy with wasm

**THIS IS A REALLY CRUDE EXPERIMENT**

## Prerequisites

- [Rust](https://www.rust-lang.org/)

- [WASM-pack](https://rustwasm.github.io/wasm-pack/installer/)

- Python3 (optional)

## Try it out

```bash
$ wasm-pack build --target web
$ python3 -m http.server 8080
```

Then go [http://localhost:8080/](http://localhost:8080/), open the dev tools and see the result for yourself in the console.