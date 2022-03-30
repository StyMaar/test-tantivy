import { initialize, SegmentBuilder, SearchIndex } from '../';

// @ts-ignore
import randomWords from 'random-words';

const DOCUMENT_COUNT = 1000;
const DOCUMENT_WORDS_COUNT = 1000;

export const runBenchmark = async () => {
  console.time('initialize');
  await initialize();
  console.timeEnd('initialize');

  console.time('new SegmentBuilder');
  const builder = new SegmentBuilder({
    id: {
      string: true,
    },
    text: {
      text: true,
      stored: true
    }
  });
  console.timeEnd('new SegmentBuilder');

  let rawTextSize = 0;

  console.time(`Indexing ${DOCUMENT_COUNT} documents with ${DOCUMENT_WORDS_COUNT} words each`);
  for (let i = 0; i < DOCUMENT_COUNT; i++) {
    if (i % 100 === 0) {
      console.log(`Adding document ${i}`);
      // sleep of 0ms to let other user actions be executed
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
    const documentText = randomWords(DOCUMENT_WORDS_COUNT).join(' ') as string;
    rawTextSize += documentText.length;
    builder.addDocument({
      id: i.toString(),
      text: documentText
    });
  }
  console.timeEnd(`Indexing ${DOCUMENT_COUNT} documents with ${DOCUMENT_WORDS_COUNT} words each`);

  const segment = builder.finalize();
  const segmentData = segment.export();

  console.log(`Segment size: ${segmentData.byteLength} bytes, raw text size ratio: ${segmentData.byteLength / rawTextSize}`);

  console.time('Create search index');
  const searchIndex = new SearchIndex<'id' | 'text'>();
  console.timeEnd('Create search index');

  console.time('Register segment');
  searchIndex.registerSegment(segment);
  console.timeEnd('Register segment');

  console.time('Random search with one word and limit: 10');
  const res = searchIndex.search(randomWords(1)[0], {limit: 10, fields: ['text']});
  console.log(res);
  console.timeEnd('Random search with one word and limit: 10');
}
