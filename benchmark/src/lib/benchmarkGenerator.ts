import { SegmentBuilder, Segment, SearchIndex, IndexSchema } from '../../..';
import { randomWords } from './randomWords';
import { initializeTantivy } from './tantivy';

const schema: IndexSchema<'id' | 'title' | 'body'> = {
  id: {
    stored: true,
    string: true,
  },
  title: {
    stored: true,
    text: true,
  },
  body: {
    text: true,
  }
}
type DocumentFields = keyof typeof schema;

export interface BenchmarkState {
  segmentBuilder: SegmentBuilder<DocumentFields>;
  segment: Segment;
  searchIndex: SearchIndex<DocumentFields>;
  exportedSegment: Uint8Array;
}

export interface BenchmarkDefinition {
  title: string;
  iterations?: number;
  executor: (state: BenchmarkState, iteration: number) => void | string | Promise<void | string>;
}

export const generateBenchmarks = ({
  documentCount = 1000,
  wordPerDocument = 1000,
} = {}) => {

  const benchmarks: BenchmarkDefinition[] = [{
    title: 'Initialize TantivyJS',
    executor: initializeTantivy
  }, {
    title: 'Create SegmentBuilder',
    executor: (state) => { state.segmentBuilder = new SegmentBuilder(schema) }
  }, {
    title: `Index ${documentCount} documents`,
    iterations: documentCount,
    executor: (state, i) => {
      const document = {
        id: i.toString(),
        title: 'This is a document title',
        body: randomWords(wordPerDocument),
      };
      state.segmentBuilder.addDocument(document);
      if ((i + 1) % 100 === 0) {
        return `Indexed document ${i + 1} / ${documentCount}`
      }
    }
  }, {
    title: 'Finalize a segment',
    executor: (state) => {
      state.segment = state.segmentBuilder.finalize();
    }
  }, {
    title: 'Export a segment',
    executor: (state) => {
      state.exportedSegment = state.segment.export();
      return `Exported size: ${state.exportedSegment.byteLength} bytes`
    }
  }, {
    title: 'Import an exported segment',
    executor: (state) => {
      state.segment = new Segment(state.exportedSegment);
    }
  }, {
    title: 'Create a SearchIndex',
    executor: (state) => {
      state.searchIndex = new SearchIndex();
    }
  }, {
    title: 'Register segment',
    executor: (state) => {
      state.searchIndex.registerSegment(state.segment)
    }
  }, {
    title: 'Execute a search query',
    executor: (state) => {
      const res = state.searchIndex.search('title', {fields: ['id', 'title', 'body'], limit: 10});
      return `Found ${res.length} documents matching the query`;
    }
  }];

  return benchmarks;
}
