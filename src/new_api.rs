use std::{collections::HashMap, mem, path::Path};

use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;

use serde_wasm_bindgen::Serializer;

use crate::errors::WasmInterfaceError;

use tantivy::{
    schema::{
        Schema as TantivySchema,
        Document as TantivyDocument,
        FAST,
        INDEXED,
        STRING,
        STORED,
        TEXT,
        TextOptions, Field, NamedFieldDocument,
    },
    DocAddress,
    Index as TantivyIndex,
    collector::TopDocs,
    query::QueryParser,
    ReloadPolicy, IndexWriter as TantivyIndexWriter, Directory, Term,
    SegmentWriter,
    Segment as TantivySegment,
};

use crate::hashmap_directory::{HashMapDirectory, SerializableHashMapDirectory};

#[derive(Serialize, Deserialize, Default)]
// Note: we must keep an Option<bool> for each field because serde(default) doesn't work with serde_wasm_bindgen: https://github.com/cloudflare/serde-wasm-bindgen/issues/20
struct FieldPRoperties {
    fast: Option<bool>,
    indexed: Option<bool>,
    string: Option<bool>,
    stored: Option<bool>,
    text: Option<bool>,
}

type Schema = HashMap<String, FieldPRoperties>;
type Document = HashMap<String, String>;

#[wasm_bindgen]
pub struct SegmentBuilder {
    // to get the index from a writer: writer.index()
    // to get the schema from an index: index.schema() (writer.index().schema())
    writer: TantivyIndexWriter,
    directory: HashMapDirectory,
    segment_writer: SegmentWriter,
    segment: TantivySegment,
}


#[wasm_bindgen]
impl SegmentBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(js_schema: JsValue, memory_arena_num_bytes: usize) -> Result<SegmentBuilder, String>{
        let schema: Schema = serde_wasm_bindgen::from_value(js_schema).map_err(|err| err.to_string())?;
        SegmentBuilder::new_inner(&schema, memory_arena_num_bytes)
    }

    fn new_inner(schema: &Schema, memory_arena_num_bytes: usize) -> Result<SegmentBuilder, String>{
        let mut schema_builder = TantivySchema::builder();

        for (field_name, option) in schema.iter(){
            
            // TODO implement the field options in a way that makes sense
            let mut field_option = TextOptions::default();
            // if option.fast.unwrap_or_default() {
                // field_option = field_option | FAST;
            // }
            // if option.indexed.unwrap_or_default() {
                // field_option = field_option | INDEXED;
            // }
            if option.string.unwrap_or_default() {
                field_option = field_option | STRING;
            }
            if option.stored.unwrap_or_default() {
                field_option = field_option | STORED;
            }
            if option.text.unwrap_or_default() {
                field_option = field_option | TEXT;
            }

            schema_builder.add_text_field(field_name, field_option);
        }
        let tantivy_schema = schema_builder.build();
        let directory= HashMapDirectory::new();
        let tantivy_index = TantivyIndex::builder().schema(tantivy_schema).open_or_create(directory.clone()).map_err(|err| err.to_string())?;
        let writer = tantivy_index.writer(memory_arena_num_bytes).map_err(|err| err.to_string())?;

        let (segment_writer, segment) = writer.get_segment_writer_and_segment().map_err(|err| err.to_string())?;

        Ok(SegmentBuilder {
            writer,
            directory,
            segment_writer,
            segment,
        })
    }

    #[wasm_bindgen(js_name = "addDocument")]
    pub fn add_document(&mut self, js_doc: JsValue) -> Result<(), String>{
        let doc : Document = serde_wasm_bindgen::from_value(js_doc).map_err(|err| err.to_string())?;

        self.add_document_inner(doc)
    }

    fn add_document_inner(&mut self, doc: Document) -> Result<(), String>{

        let mut tantivy_doc = TantivyDocument::default();
        for (field_name, data) in doc {
            let field = self.writer.index().schema().get_field(&field_name).ok_or_else(||{WasmInterfaceError::InvalidField(field_name).to_string()})?;
            tantivy_doc.add_text(field, data);
        }

        self.writer.add_document_to_segment_writer(&mut self.segment_writer, tantivy_doc).map_err(|err| err.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = "removeDocuments")]
    pub fn remove_documents(&mut self, key_field: &str, key: &str)-> Result<(), String>{
        let field = self.writer.index().schema().get_field(&key_field).ok_or_else(||{WasmInterfaceError::InvalidField(key_field.to_string()).to_string()})?;

        let term = Term::from_field_text(field, key);
        self.writer.delete_term(term.clone());
        self.writer.commit().map_err(|err| err.to_string())?;

        Ok(())
    }
    pub fn finalize(mut self) -> Result<Segment, String> {
        self.writer.finalize_document_addition(self.segment_writer, self.segment).map_err(|err| err.to_string())?;
        self.writer.commit().map_err(|err| err.to_string())?;
        // let searchable_doc_id = self.writer.index().searchable_segment_ids().map_err(|err| err.to_string())?;
        // if searchable_doc_id.len() != 0 {
        //     self.writer.merge(&searchable_doc_id).map_err(|err| err.to_string())?;
        //     self.writer.commit().map_err(|err| err.to_string())?; // TODO: voir avec François si cette façon de faire commit/merge/commit c'est logique
        // } 

        Ok(Segment{
            directory: self.directory,
        })
    }
}

#[wasm_bindgen]
pub struct Segment{
    directory: HashMapDirectory,
}

#[wasm_bindgen]
impl Segment {
    pub fn export(&self) -> Result<Vec<u8>, String>{ // self or &self ?
        let directory : SerializableHashMapDirectory = (&self.directory).into();
        let bytes = rkyv::to_bytes::<_, 256>(&directory).map_err(|_err|WasmInterfaceError::FailedToSerializeDirectory.to_string())?.into_vec();
        Ok(bytes)
    }
    
    #[wasm_bindgen(constructor)]
    pub fn new(segment_data: &[u8])-> Result<Segment, String> {
        use rkyv::{Deserialize};
        let archived = rkyv::check_archived_root::<SerializableHashMapDirectory>(segment_data).map_err(|_err|WasmInterfaceError::FailedToCreateArchiveRoot.to_string())?;
        let directory: SerializableHashMapDirectory = archived.deserialize(&mut rkyv::Infallible).map_err(|_err|WasmInterfaceError::FailedToDeSerializeDirectory.to_string())?;
        Ok(Segment{directory: directory.into()})
    }
}

// TODO: discuter de l'API parce que là ça ne va pas marcher …
pub fn merge_segment(segments: Vec<Segment>) -> Segment{
    // il faut ajouter à la main les fichiers des directory des segments n°N (avec n>0) dans le directory du segment n=0
    todo!()
} 

#[wasm_bindgen]
pub struct SearchIndex {
    directory: Option<HashMapDirectory>,
}

#[wasm_bindgen]
impl SearchIndex {
    #[wasm_bindgen(constructor)]
    // Note: API change: there's no need to add the schema as a parameter here, it's already part of the metadata of the segment
    pub fn new() -> SearchIndex {
        SearchIndex{
            directory: None,
        }
    }

    // TODO ensure that the different segments added have the same underlying schema
    #[wasm_bindgen(js_name = "registerSegment")]
    pub fn register_segment(&mut self, segment: Segment)-> Result<(), String>{

        if let Some(ref mut directory) = self.directory {
            let this_index = TantivyIndex::open(directory.clone()).map_err(|err| err.to_string())?;
            let index_to_add = TantivyIndex::open(segment.directory.clone()).map_err(|err| err.to_string())?;

            let mut this_meta = this_index.load_metas().map_err(|err| err.to_string())?;
            let segments_to_add = index_to_add.load_metas().map_err(|err| err.to_string())?.segments;

            for to_add in segments_to_add {
                let exists = this_meta.segments.iter().find(|segment| to_add.id() == segment.id()).is_some();
                if !exists {
                    this_meta.segments.push(to_add);
                }

            }

            directory.atomic_write(Path::new("meta.json"), &serde_json::to_vec(&this_meta).map_err(|err| err.to_string())?).map_err(|err| err.to_string())?;
            directory.agregate(segment.directory)
        }else{
            self.directory = Some(segment.directory);
        }
        Ok(())
    }
    
    // TODO définir la gestion d'erreur: qu'est-ce qu'on fait si on essaie de supprimer quelque chose qui n'est pas dans le directory
    #[wasm_bindgen(js_name = "removeSegment")]
    pub fn remove_segment(&mut self, segment: Segment)-> Result<(), String>{

        if let Some(ref mut directory) = self.directory {
            let this_index = TantivyIndex::open(directory.clone()).map_err(|err| err.to_string())?;
            let index_to_remove = TantivyIndex::open(segment.directory.clone()).map_err(|err| err.to_string())?;
    
            let mut this_meta = this_index.load_metas().map_err(|err| err.to_string())?;
            let segments_to_remove = &index_to_remove.load_metas().map_err(|err| err.to_string())?.segments;
    
            for to_remove in segments_to_remove {
                this_meta.segments.retain(|segment| to_remove.id() == segment.id());
            }
    
            directory.atomic_write(Path::new("meta.json"), &serde_json::to_vec(&this_meta).map_err(|err| err.to_string())?).map_err(|err| err.to_string())?;
            directory.remove_directory(segment.directory);
        }
        Ok(())
    }

    // -> SearchResult
    pub fn search(&self, query: &str, js_option: JsValue)-> Result<JsValue, String>{
        let option: SearchOption = serde_wasm_bindgen::from_value(js_option).map_err(|err| err.to_string())?;

        let results = self.search_inner(query, option)?;
        let serializer = Serializer::new().serialize_maps_as_objects(true);
        Ok(results.serialize(&serializer).map_err(|err| err.to_string())?)
    }
    fn search_inner(&self, query: &str, option: SearchOption)-> Result<SearchResult, String>{
        if let Some(ref directory) = self.directory {
            let index = TantivyIndex::open(directory.clone()).map_err(|err| err.to_string())?;
            let reader = index
                .reader_builder()
                .reload_policy(ReloadPolicy::Manual)
                .try_into().map_err(|err| err.to_string())?;

            let searcher = reader.searcher();
            let fields_res = option.fields.iter().map(|field_name|{
                let field = index.schema()
                                 .get_field(&field_name)
                                 .ok_or_else(||{
                    WasmInterfaceError::InvalidField(field_name.to_owned()).to_string()
                })?;
                Ok(field)
            }).collect::<Result<Vec<Field>, String>>();
            let fields = fields_res?;
            let query_parser = QueryParser::for_index(&index, fields);
            let query = query_parser.parse_query(query).map_err(|err| err.to_string())?;
            let top_docs = searcher.search(&query, &TopDocs::with_limit(option.limit))
                                   .map_err(|err| err.to_string())?;

            let mut results = Vec::new();
            for (_score, doc_address) in top_docs {
                let retrieved_doc = searcher.doc(doc_address)
                                            .map_err(|err| err.to_string())?;
                results.push(index.schema().to_named_doc(&retrieved_doc));
            }
            Ok(results)
        }else{
            Err(WasmInterfaceError::EmptyDirectory.to_string())
        }
    }

    #[wasm_bindgen(js_name = "directorySummary")]
    pub fn directory_summary(&self){
        if let Some(ref directory) = self.directory {
            directory.summary();
        }
    }
}



type SearchResult= Vec<NamedFieldDocument>;

#[derive(Serialize, Deserialize)]
struct SearchOption{
    fields: Vec<String>,
    limit: usize,
}

#[wasm_bindgen]
pub struct Merger{
    // we use a SearchIndex here because we want to reuse the segment merging mechanism. TODO: refactor to remove this strange dependency
    search_index: SearchIndex,
    added_segments: usize,
}

#[wasm_bindgen]
impl Merger{
    #[wasm_bindgen(constructor)]
    pub fn new() -> Merger {
        Merger{search_index: SearchIndex::new(), added_segments: 0}
    }

    #[wasm_bindgen(js_name = "addSegment")]
    pub fn add_segment(&mut self, segment: Segment) -> Result<(), String>{
        self.search_index.register_segment(segment)?;
        self.added_segments +=1;
        Ok(())
    }

    pub fn merge(self) -> Result<Segment, String> {
        match self.added_segments{
            0 => panic!("Cannot merge empty segments"),
            1 => {
                let directory = self.search_index.directory.ok_or_else(||{WasmInterfaceError::EmptyDirectory.to_string()})?;

                Ok(Segment{
                    directory
                })
            },
            _ => {
                // we need to create a TantivyIndex, to create a writer in order to perform the merge.
                let directory = self.search_index.directory.ok_or_else(||{WasmInterfaceError::EmptyDirectory.to_string()})?;

                let tantivy_index = TantivyIndex::open(directory.clone()).map_err(|err| err.to_string())?;
                let mut writer = tantivy_index.writer(50_000_000).map_err(|err| err.to_string())?;
                let searchable_doc_id = writer.index().searchable_segment_ids().map_err(|err| err.to_string())?;
                writer.merge(&searchable_doc_id).map_err(|err| err.to_string())?;
                writer.commit().map_err(|err| err.to_string())?;

                Ok(Segment{
                    directory,
                })
            }
        }
    } 
}

#[cfg(test)]
mod test{
    use common_macros::hash_map;

    use crate::{SegmentBuilder, SearchIndex, new_api::Merger};

    use super::{FieldPRoperties, SearchOption};

    #[test]
    fn simple_search(){
        let schema = hash_map! {
                "id".to_string() => FieldPRoperties{text: Some(true), ..Default::default()},
                "body".to_string() => FieldPRoperties{text: Some(true), ..Default::default()},
                "title".to_string() => FieldPRoperties{text: Some(true), stored: Some(true), ..Default::default()},
            };

        let mut segment_builder = SegmentBuilder::new_inner(&schema, 50_000_000).unwrap(); 

        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "0".to_string(),
          "title".to_string() => "Lord Of The Rings".to_string(),
          "body".to_string() => "And some things that should not have been forgotten were lost. History became legend. Legend became myth. And for two and a half thousand years, the ring passed out of all knowledge.".to_string(),
        }).unwrap();
        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "1".to_string(),
          "title".to_string() =>  "The Old Man and the Sea".to_string(),
          "body".to_string() => r#"He was an old man who fished alone in a skiff in the Gulf Stream and 
          he had gone eighty-four days now without taking a fish."#.to_string(),
        }).unwrap();
        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "2".to_string(),
          "title".to_string() => "Frankenstein".to_string(),
          "body".to_string() => r#"You will rejoice to hear that no disaster has accompanied the commencement of an 
          enterprise which you have regarded with such evil forebodings.  I arrived here 
          yesterday, and my first task is to assure my dear sister of my welfare and 
          increasing confidence in the success of my undertaking."#.to_string(),
          
        }).unwrap();

        let segment = segment_builder.finalize().unwrap();

        let mut search_index = SearchIndex::new();
        search_index.register_segment(segment).unwrap();

        let results = search_index.search_inner("the", SearchOption{fields: vec!["title".to_string()], limit: 10}).unwrap();
        assert_eq!(2, results.len());
        let results = search_index.search_inner("the", SearchOption{fields: vec!["body".to_string()], limit: 10}).unwrap();
        assert_eq!(3, results.len());
    }

    #[test]
    fn merge_and_search(){
        let schema = hash_map! {
                "id".to_string() => FieldPRoperties{text: Some(true), ..Default::default()},
                "body".to_string() => FieldPRoperties{text: Some(true), ..Default::default()},
                "title".to_string() => FieldPRoperties{text: Some(true), stored: Some(true), ..Default::default()},
            };

        let mut segment_builder = SegmentBuilder::new_inner(&schema, 50_000_000).unwrap(); 

        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "0".to_string(),
          "title".to_string() => "Lord Of The Rings".to_string(),
          "body".to_string() => "And some things that should not have been forgotten were lost. History became legend. Legend became myth. And for two and a half thousand years, the ring passed out of all knowledge.".to_string(),
        }).unwrap();
        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "1".to_string(),
          "title".to_string() =>  "The Old Man and the Sea".to_string(),
          "body".to_string() => r#"He was an old man who fished alone in a skiff in the Gulf Stream and 
          he had gone eighty-four days now without taking a fish."#.to_string(),
        }).unwrap();
        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "2".to_string(),
          "title".to_string() => "Frankenstein".to_string(),
          "body".to_string() => r#"You will rejoice to hear that no disaster has accompanied the commencement of an 
          enterprise which you have regarded with such evil forebodings.  I arrived here 
          yesterday, and my first task is to assure my dear sister of my welfare and 
          increasing confidence in the success of my undertaking."#.to_string(),
        }).unwrap();

        let segment1 = segment_builder.finalize().unwrap();

        let mut segment_builder = SegmentBuilder::new_inner(&schema, 50_000_000).unwrap();

        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "3".to_string(),
          "title".to_string() => "Le seigneur des anneaux".to_string(),
          "body".to_string() =>  "C'est une étrange fatalité que nous devions éprouver tant de peurs et de doutes, pour une si petite chose.".to_string(),
        }).unwrap();
        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "12".to_string(),
          "title".to_string() => "Of Mice and Men".to_string(),
          "body".to_string() =>   r#"A few miles south of Soledad, the Salinas River drops in close to the hillside 
          bank and runs deep and green. The water is warm too, for it has slipped twinkling 
          over the yellow sands in the sunlight before reaching the narrow pool. On one 
          side of the river the golden foothill slopes curve up to the strong and rocky 
          Gabilan Mountains, but on the valley side the water is lined with trees—willows 
          fresh and green with every spring, carrying in their lower leaf junctures the 
          debris of the winter’s flooding; and sycamores with mottled, white, recumbent 
          limbs and branches that arch over the pool"#.to_string(),
        }).unwrap();
        
        let segment2 = segment_builder.finalize().unwrap();

        let mut merger = Merger::new();
        merger.add_segment(segment1).unwrap();
        merger.add_segment(segment2).unwrap();

        let merged_segment = merger.merge().unwrap();

        let mut search_index = SearchIndex::new();
        search_index.register_segment(merged_segment).unwrap();

        let results = search_index.search_inner("the", SearchOption{fields: vec!["title".to_string()], limit: 10}).unwrap();
        assert_eq!(2, results.len());
        let results = search_index.search_inner("the", SearchOption{fields: vec!["body".to_string()], limit: 10}).unwrap();
        assert_eq!(4, results.len());
    }

    #[test]
    fn remove_and_merge(){
        let schema = hash_map! {
                "id".to_string() => FieldPRoperties{text: Some(true), ..Default::default()},
                "body".to_string() => FieldPRoperties{text: Some(true), ..Default::default()},
                "title".to_string() => FieldPRoperties{text: Some(true), stored: Some(true), ..Default::default()},
            };

        let mut segment_builder = SegmentBuilder::new_inner(&schema, 50_000_000).unwrap(); 

        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "0".to_string(),
          "title".to_string() => "Lord Of The Rings".to_string(),
          "body".to_string() => "And some things that should not have been forgotten were lost. History became legend. Legend became myth. And for two and a half thousand years, the ring passed out of all knowledge.".to_string(),
        }).unwrap();
        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "1".to_string(),
          "title".to_string() =>  "The Old Man and the Sea".to_string(),
          "body".to_string() => r#"He was an old man who fished alone in a skiff in the Gulf Stream and 
          he had gone eighty-four days now without taking a fish."#.to_string(),
        }).unwrap();
        segment_builder.add_document_inner(hash_map! {
          "id".to_string() => "2".to_string(),
          "title".to_string() => "Frankenstein".to_string(),
          "body".to_string() => r#"You will rejoice to hear that no disaster has accompanied the commencement of an 
          enterprise which you have regarded with such evil forebodings.  I arrived here 
          yesterday, and my first task is to assure my dear sister of my welfare and 
          increasing confidence in the success of my undertaking."#.to_string(),
        }).unwrap();

        let segment1 = segment_builder.finalize().unwrap();

        let mut segment_builder = SegmentBuilder::new_inner(&schema, 50_000_000).unwrap();

        segment_builder.remove_documents("id", "2").unwrap();

        let segment2 = segment_builder.finalize().unwrap();

        let mut merger = Merger::new();
        merger.add_segment(segment1).unwrap();
        merger.add_segment(segment2).unwrap();

        let merged_segment = merger.merge().unwrap();

        let mut search_index = SearchIndex::new();
        search_index.register_segment(merged_segment).unwrap();

        let results = search_index.search_inner("the", SearchOption{fields: vec!["title".to_string()], limit: 10}).unwrap();
        assert_eq!(2, results.len());
        let results = search_index.search_inner("the", SearchOption{fields: vec!["body".to_string()], limit: 10}).unwrap();
        assert_eq!(2, results.len());
    }

}
