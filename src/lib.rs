use wasm_bindgen::prelude::wasm_bindgen;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{schema::*, directory};
use tantivy::{doc, Index, ReloadPolicy};

mod hashmap_directory;

use hashmap_directory::HashMapDirectory;

use std::panic;



#[wasm_bindgen]
pub fn index() -> String {
    // pour faire en sorte que les panic! soient bien gérés avec WebAssembly
    panic::set_hook(Box::new(console_error_panic_hook::hook));
    
    let virtual_directory = HashMapDirectory::new();

    let (schema, title, body) = build_schema();

    /////////////////////////////////////////////

    let index = Index::builder().schema(schema.clone()).open_or_create(virtual_directory.clone()).unwrap();
    let mut index_writer = index.writer(50_000_000).unwrap();

    let mut old_man_doc = Document::default();
    old_man_doc.add_text(title, "The Old Man and the Sea");
    old_man_doc.add_text(
        body,
        "He was an old man who fished alone in a skiff in the Gulf Stream and \
         he had gone eighty-four days now without taking a fish.",
    );

    // ... and add it to the `IndexWriter`.
    index_writer.add_document(old_man_doc);

    // For convenience, tantivy also comes with a macro to
    // reduce the boilerplate above.
    index_writer.add_document(doc!(
    title => "Of Mice and Men",
    body => "A few miles south of Soledad, the Salinas River drops in close to the hillside \
            bank and runs deep and green. The water is warm too, for it has slipped twinkling \
            over the yellow sands in the sunlight before reaching the narrow pool. On one \
            side of the river the golden foothill slopes curve up to the strong and rocky \
            Gabilan Mountains, but on the valley side the water is lined with trees—willows \
            fresh and green with every spring, carrying in their lower leaf junctures the \
            debris of the winter’s flooding; and sycamores with mottled, white, recumbent \
            limbs and branches that arch over the pool"
    ));

    // Multivalued field just need to be repeated.
    index_writer.add_document(doc!(
    title => "Frankenstein",
    title => "The Modern Prometheus",
    body => "You will rejoice to hear that no disaster has accompanied the commencement of an \
             enterprise which you have regarded with such evil forebodings.  I arrived here \
             yesterday, and my first task is to assure my dear sister of my welfare and \
             increasing confidence in the success of my undertaking."
    ));

    // Multivalued field just need to be repeated.
    index_writer.add_document(doc!(
    title => "Lord of the Rings",
    body => "Home is behind, the world ahead,
and there are many paths to tread
through shadows to the edge of night,
until the stars are all alight."
    ));

    index_writer.commit().unwrap();

    /// ///////////////////////////////////////////


    let serialized_directory = serde_json::to_string(&virtual_directory).unwrap();

    serialized_directory
}

#[wasm_bindgen]
pub fn search(query: &str, directory: &str) -> String {
    
    let (schema, title, body) = build_schema();

    let deserialized_directory: HashMapDirectory = serde_json::from_str(&directory).unwrap();

    let index = Index::builder().schema(schema.clone()).open_or_create(deserialized_directory).unwrap();

    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::Manual)
        .try_into().unwrap();

    let searcher = reader.searcher();

    let query_parser = QueryParser::for_index(&index, vec![title, body]);

    let query = query_parser.parse_query(query).unwrap();

    let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

    let mut results_string = String::new();
    for (_score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address).unwrap();
        results_string.push_str(&schema.to_json(&retrieved_doc));
    }
    results_string
}

fn build_schema()-> (Schema, Field, Field){
        
    let mut schema_builder = Schema::builder();

    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT);

    let schema = schema_builder.build();

    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();

    (schema, title, body)
}

