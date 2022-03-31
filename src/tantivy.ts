import init, {
  InitInput,
  Merger,
  SearchIndex as WasmSearchIndex,
  SegmentBuilder as WasmSegmentBuilder,
  Segment as WasmSegment,
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

// Free up the Rust memory when the SegmentBuilder class is not used anymore
const segmentBuildersFinalizationRegistry = new FinalizationRegistry<WasmSegmentBuilder>((wasmSegmentBuilder) => wasmSegmentBuilder.free());

export class SegmentBuilder<Fields extends string> {
  private wasmSegmentBuilder: WasmSegmentBuilder;

  constructor(schema: IndexSchema<Fields>, memoryArenaNumBytes = 50_000_000) {
    this.wasmSegmentBuilder = new WasmSegmentBuilder(schema, memoryArenaNumBytes);
    segmentBuildersFinalizationRegistry.register(this, this.wasmSegmentBuilder);
  }

  addDocument(document: {[field in Fields]: string}) {
    this.wasmSegmentBuilder.addDocument(document);
  }

  removeDocuments() {
    this.wasmSegmentBuilder.removeDocuments();
  }

  finalize() {
    segmentBuildersFinalizationRegistry.unregister(this);
    return new Segment(this.wasmSegmentBuilder.finalize());
  }
}

// Free up the Rust memory when the SearchIndex class is not used anymore
const searchIndexFinalizationRegistry = new FinalizationRegistry<WasmSearchIndex>((wasmSearchIndex) => wasmSearchIndex.free());
export class SearchIndex<Fields extends string> {
  private wasmSearchIndex: WasmSearchIndex;

  constructor() {
    this.wasmSearchIndex = new WasmSearchIndex();
    searchIndexFinalizationRegistry.register(this, this.wasmSearchIndex);
  }

  registerSegment(segment: Segment) {
    this.wasmSearchIndex.registerSegment(segment._getWasmSegment());
  }

  removeSegment(segment: Segment) {
    this.wasmSearchIndex.removeSegment(segment._getWasmSegment());
  }

  search(query: string, options: {limit?: number, fields?: (Fields)[] } = {}): {[field in Fields]: string}[] {
    return this.wasmSearchIndex.search(query, options);
  }

  directorySummary() {
    return this.wasmSearchIndex.directorySummary();
  }
}

// Free up the Rust memory when the Segment class is not used anymore
const segmentFinalizationRegistry = new FinalizationRegistry<WasmSegment>((wasmSegment) => wasmSegment.free());
export class Segment {
  private wasmSegment: WasmSegment;

  /**
   * Create a new Segment from raw data of a previously exported segment
   * @param data raw data from Segment.export, only as Uint8Array
   */
  constructor(data: Uint8Array | WasmSegment) {
    if (data instanceof Uint8Array) {
      this.wasmSegment = new WasmSegment(data);
    } else {
      this.wasmSegment = data;
    }
    segmentFinalizationRegistry.register(this, this.wasmSegment);
  }

  /**
   * Export the raw data in a segment as a Uint8Array
   * @returns raw data as Uint8Array
   */
  export() {
    return this.wasmSegment.export();
  }

  _getWasmSegment() {
    return this.wasmSegment;
  }
}

export { Merger };
