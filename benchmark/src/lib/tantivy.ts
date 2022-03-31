import {initialize} from '../../../dist/es-slim/index_slim';

import wasm from '../../../dist/tantivy.wasm?url';

let initialized = false;
export const initializeTantivy = async () => {
  if (initialized) {
    return;
  }
  await initialize({
    wasm
  });
  initialized = true;
}


export * from '../../../dist/es-slim/index_slim';
