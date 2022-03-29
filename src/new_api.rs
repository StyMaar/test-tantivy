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
        TextOptions, Field,
    },
    DocAddress,
    Index as TantivyIndex,
    collector::TopDocs,
    query::QueryParser,
    ReloadPolicy, IndexWriter as TantivyIndexWriter, Directory,
};

use crate::hashmap_directory::{HashMapDirectory, SerializableHashMapDirectory};

#[derive(Serialize, Deserialize)]
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
}

#[wasm_bindgen]
impl SegmentBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(js_schema: JsValue, memory_arena_num_bytes: usize) -> Result<SegmentBuilder, String>{
        let schema: Schema = serde_wasm_bindgen::from_value(js_schema).map_err(|err| err.to_string())?;
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

        Ok(SegmentBuilder {
            writer,
            directory,
        })
    }
    #[wasm_bindgen(js_name = "addDocument")]
    pub fn add_document(&mut self, js_doc: JsValue) -> Result<(), String>{
        let doc : Document = serde_wasm_bindgen::from_value(js_doc).map_err(|err| err.to_string())?;

        let mut tantivy_doc = TantivyDocument::default();
        for (field_name, data) in doc {
            let field = self.writer.index().schema().get_field(&field_name).ok_or_else(||{WasmInterfaceError::InvalidField(field_name).to_string()})?;
            tantivy_doc.add_text(field, data);
        }

        self.writer.add_document(tantivy_doc).map_err(|err| err.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name = "removeDocuments")]
    pub fn remove_documents(&self){
        todo!()
    }
    pub fn finalize(mut self) -> Result<Segment, String> {
        self.writer.commit().map_err(|err| err.to_string())?;
        let searchable_doc_id = self.writer.index().searchable_segment_ids().map_err(|err| err.to_string())?;
        self.writer.merge(&searchable_doc_id).map_err(|err| err.to_string())?;
        self.writer.commit().map_err(|err| err.to_string())?; // TODO: voir avec François si cette façon de faire commit/merge/commit c'est logique

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
            let this_index = TantivyIndex::open_from_dir(directory.clone()).map_err(|err| err.to_string())?;
            let index_to_add = TantivyIndex::open_from_dir(segment.directory.clone()).map_err(|err| err.to_string())?;

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
            let this_index = TantivyIndex::open_from_dir(directory.clone()).map_err(|err| err.to_string())?;
            let index_to_remove = TantivyIndex::open_from_dir(segment.directory.clone()).map_err(|err| err.to_string())?;
    
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
        if let Some(ref directory) = self.directory {
            let index = TantivyIndex::open_from_dir(directory.clone()).map_err(|err| err.to_string())?;
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

            let serializer = Serializer::new().serialize_maps_as_objects(true);
            Ok(results.serialize(&serializer).map_err(|err| err.to_string())?)
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



type SearchResult= Vec<Document>;

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

                let tantivy_index = TantivyIndex::open_from_dir(directory.clone()).map_err(|err| err.to_string())?;
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
