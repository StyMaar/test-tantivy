# TantivyJS

TantivyJS is a WebAssembly port of [Tantivy](https://github.com/quickwit-oss/tantivy/), a highly performant full-text search library written in Rust. This is not the simplest full-text search library to use but it's highly focused on performance.

- Natural query language (e.g. (michael AND jackson) OR "king of pop")
- Phrase queries search (e.g. "michael jackson")
- LZ4 compressed document store
- Range queries
- Typescript types
- Export and import of search index in a Uint8Array

## Quick-start

TantivyJS is usable on a [browser supporting WASM](https://caniuse.com/wasm) and with NodeJS. It's installable from npm with `npm install tantivy` and then usable like so:

```ts
import { initialize, SegmentBuilder } from 'tantivy';

await initialize();
const builder = new SegmentBuilder({
  id: {string: true, stored: true},
  body: {text: true}
});
builder.addDocument({
  id: '42',
  body: 'He was an old man who fished alone in a skiff in the Gulf Stream and he had gone eighty-four days now without taking a fish.'
});
builder.addDocument({
  id: '43',
  body: 'A few miles south of Soledad, the Salinas River drops in close to the hillside bank and runs deep and green. The water is warm too, for it has slipped twinkling'
});
const segment = builder.finalize();
// Can export a segment to a Uint8Array with segment.export()

const searchIndex = new SearchIndex<'id' | 'body'>();
searchIndex.registerSegment(segment);

const result = searchIndex.search('miles', { limit: 1, fields: ['body'] });
// result is [{ id: '43' }]
```

## Why Segment / Index / Merger?

The data model of Tantivy is based on independant "segments", each containing the data necessary to search in the added documents. A segment is immutable, meaning it cannot be updated to include more documents or remove existing documents. In a lot of use-cases, it's necessary to add or remove documents from the search index. To do so, you need to create additionnal segments and "register" them in a `SearchIndex`.

A `SearchIndex` contains a collection of segments that will be used for a search query. It will basically search independantly in each segment and then merge the results. This logic has a big impact on performance: for the same number of documents, it will be much faster to search on one big segment than on 10 smaller segments.

`Merger` is used to merge multiple segments into a single one. This is useful to:
- reduce the total size of a search index: a bigger segment will compress more efficiently a lot of documents compared to multiple smaller segments
- speed up search queries: a search query on a big segment will be faster than on multiple smaller segments with the same total number of documents

There are some limitations necessary to the performance of Tantivy:
- You cannot use a segment for a search query before `finalize`ing it
- You cannot add/remove documents from a segment once it's finalized
- When removing a document, the storage used by the document will only be freed when the segment containing the "add" operation is merged with the segment having the "remove" operation

## API

### Segment builder

`SegmentBuilder` is the entry-point to add/remove/update documents in a search index. Its role is to ingest documents, do a lot of "indexing-stuff" on them and then, once you are finished, create a `Segment` with the `.finalize()` method. Once a SegmentBuilder is finalized, no other operation can be done with it, you must create another `SegmentBuilder`.

```ts
const segmentBuilder = new SegmentBuilder({
  // this is your schema declaring each field of the documents you'll index
  title: { // "title" will be one field of the indexable/searchable documents
    text: true, // this field will be tokenized and indexed
    stored: true // this field will be stored in the index and returned in the search results
  },
  body: {
    text: true, // tokenized and indexed
    stored: false // this field is not stored: it's still indexed (and so searchable) but won't be returned in search results, this will reduce the index size
  },
  id: {
    string: true, // this field will be indexed as-is, without any tokenization
    stored: true
    // this is still a "normal" field meaning that multiple documents can have the same "id"
  }
  // you can have as many fields as you want in a document
});

segmentBuilder.addDocument({
  title: 'Of Mice and Men with a typo',
  body: 'A few miles south of Soledad, the Salinas River drops in close to the hillside bank and runs deep and green. The water is warm too, for it has slipped twinkling',
  id: '42'
});

segmentBuilder.addDocument({
  title: 'Frankenstein',
  body: 'You will rejoice to hear that no disaster has accompanied the commencement of an enterprise which you have regarded with such evil forebodings.',
  id: '43'
});

// You can remove documents by filtering by a field and value pair.
segmentBuilder.removeDocuments('id', '42'); // every document with id=42 will be removed from the index

// "removeDocuments" only affects documents indexed before, you can still add documents with id-42 after this
segmentBuilder.addDocument({
  title: 'Of Mice and Men',
  body: 'A few miles south of Soledad, the Salinas River drops in close to the hillside bank and runs deep and green. The water is warm too, for it has slipped twinkling',
  id: '42'
});

const segment = segmentBuilder.finalize();
// Cannot use segmentBuilder after this point, another instance should be created if needed
```

### SearchIndex

TODO

### Merger

TODO

## How to build

You need the Rust toolchain and NodeJS/Yarn installed, then execute `yarn` to install the dependencies and `yarn build` to generate the `dist/` folder containing the built packages.

To use the benchmarks, navigate to `benchmark/`, install deps with `yarn` and either launch the dev mode with `yarn dev` or build the project with `yarn build`.
