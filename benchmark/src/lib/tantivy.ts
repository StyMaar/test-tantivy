// import {initialize} from '../../../dist/es-slim/index_slim';
import {initialize} from '../../..';

// import wasm from '../../../dist/tantivy.wasm?url';

let initialized = false;
export const initializeTantivy = async () => {
  if (initialized) {
    return;
  }
  await initialize({
    // wasm
  });
  initialized = true;
}


