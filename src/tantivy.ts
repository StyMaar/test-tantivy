import init, {
  InitInput,
  Merger,
  SearchIndex as WasmSearchIndex,
  SegmentBuilder as WasmSegmentBuilder,
  Segment,
  set_panic_hook
} from "./pkg/tantivy_js";


/**
 * Customize how tantivy is loaded
 */
export interface TantivyLoadOptions {
  /**
   * Controls how the Wasm module is instantiated.
   */
  wasm?: InitInput;
}

let wasmInit: (() => InitInput) | undefined = undefined;
export const setWasmInit = (arg: () => InitInput) => {
  wasmInit = arg;
};

let initialized: Promise<void> | undefined = undefined;
export const initialize = async (options?: TantivyLoadOptions) => {
  if (initialized === undefined) {
    //@ts-ignore
    const loadModule = options?.wasm ?? wasmInit();
    initialized = init(loadModule).then(() => void 0);
  }

  await initialized;
  set_panic_hook();
  return;
};

export type IndexSchema<Fields extends string> = {
  [field in Fields]: {
    string?: boolean;
    text?: boolean;
    stored?: boolean;
  }
}

export class SegmentBuilder<Fields extends string> {
  private wasmSegmentBuilder: WasmSegmentBuilder;

  constructor(schema: IndexSchema<Fields>, memoryArenaNumBytes = 50_000_000) {
    this.wasmSegmentBuilder = new WasmSegmentBuilder(schema, memoryArenaNumBytes);
  }

  addDocument(document: {[field in Fields]: string}) {
    this.wasmSegmentBuilder.addDocument(document);
  }

  removeDocuments() {
    this.wasmSegmentBuilder.removeDocuments();
  }

  finalize() {
    return this.wasmSegmentBuilder.finalize();
  }
}

export class SearchIndex<Fields extends string> {
  private wasmSearchIndex: WasmSearchIndex;

  constructor() {
    this.wasmSearchIndex = new WasmSearchIndex();
  }

  registerSegment(segment: Segment) {
    this.wasmSearchIndex.registerSegment(segment);
  }

  removeSegment(segment: Segment) {
    this.wasmSearchIndex.removeSegment(segment);
  }

  search(query: string, options: {limit?: number, fields?: (Fields)[] } = {}): {[field in Fields]: string}[] {
    return this.wasmSearchIndex.search(query, options);
  }

  directorySummary() {
    return this.wasmSearchIndex.directorySummary();
  }
}

export { Segment, Merger };
